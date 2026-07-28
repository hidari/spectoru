//! `SerdeJsonCodec` の契約テスト。
//!
//! ここで検証する性質は「`serde_json` をどんな別ライブラリに差し替えても、
//! `JsonCodec` ポートを実装する以上は守られなければならない契約」である。
//! ライブラリ差し替え時、新 adapter がこのテストを通過すれば application 層
//! は影響を受けない。

#![allow(non_snake_case)]

use std::path::PathBuf;

use spectoru::adapters::serde_json_codec::SerdeJsonCodec;
use spectoru::core::ir::{
    Diagnostic, DiagnosticCode, DiagnosticLevel, Group, IntermediateRepresentation, Language,
    Languages, ProjectMeta, Source, Spec, SpecStatus, Stats,
};
use spectoru::error::SpectoruError;
use spectoru::ports::json_codec::JsonCodec;

fn full_ir() -> IntermediateRepresentation {
    IntermediateRepresentation {
        project: ProjectMeta {
            name: "Astralys".to_string(),
            repository: Some("https://github.com/HermitianHQ/astralys".to_string()),
            revision: Some("abc1234".to_string()),
            extracted_at: "2026-04-13T12:00:00Z".to_string(),
        },
        sources: vec![Source {
            name: "Backend".to_string(),
            groups: vec![Group {
                name: "tests/integration/artwork_creation.rs".to_string(),
                file: PathBuf::from("tests/integration/artwork_creation.rs"),
                line: None,
                children: vec![Group {
                    name: "有効な画像がアップロードされたとき".to_string(),
                    file: PathBuf::from("tests/integration/artwork_creation.rs"),
                    line: Some(3),
                    children: vec![],
                    specs: vec![Spec {
                        name: "作品が公開状態で作成される".to_string(),
                        file: PathBuf::from("tests/integration/artwork_creation.rs"),
                        line: 5,
                        language: Language::Rust,
                        status: SpecStatus::Active,
                    }],
                }],
                specs: vec![],
            }],
        }],
        diagnostics: vec![Diagnostic {
            level: DiagnosticLevel::Warning,
            code: DiagnosticCode::NestingTooDeep,
            message: "Nesting depth exceeds limit (5 > 4)".to_string(),
            file: Some(PathBuf::from("tests/integration/complex_flow.rs")),
            line: Some(30),
        }],
        stats: Stats {
            total_specs: 1,
            warnings: 1,
            languages: Languages {
                rust: 1,
                typescript: 0,
            },
        },
    }
}

fn ir_without_optional_project_fields() -> IntermediateRepresentation {
    let mut ir = full_ir();
    ir.project.repository = None;
    ir.project.revision = None;
    ir
}

fn ir_with_typescript_and_skipped_specs() -> IntermediateRepresentation {
    IntermediateRepresentation {
        project: ProjectMeta {
            name: "Astralys".to_string(),
            repository: None,
            revision: None,
            extracted_at: "2026-04-13T12:00:00Z".to_string(),
        },
        sources: vec![Source {
            name: "Frontend".to_string(),
            groups: vec![Group {
                name: "app/tests/foo.test.ts".to_string(),
                file: PathBuf::from("app/tests/foo.test.ts"),
                line: None,
                children: vec![],
                specs: vec![
                    Spec {
                        name: "active test".to_string(),
                        file: PathBuf::from("app/tests/foo.test.ts"),
                        line: 1,
                        language: Language::TypeScript,
                        status: SpecStatus::Active,
                    },
                    Spec {
                        name: "skipped test".to_string(),
                        file: PathBuf::from("app/tests/foo.test.ts"),
                        line: 5,
                        language: Language::TypeScript,
                        status: SpecStatus::Skipped,
                    },
                ],
            }],
        }],
        diagnostics: vec![],
        stats: Stats {
            total_specs: 2,
            warnings: 0,
            languages: Languages {
                rust: 0,
                typescript: 2,
            },
        },
    }
}

/// 1 プロジェクトに Rust / Vitest 両方の source がぶら下がるモノレポ構成。
fn ir_with_multiple_sources() -> IntermediateRepresentation {
    let mut ir = full_ir();
    ir.sources
        .extend(ir_with_typescript_and_skipped_specs().sources);
    ir.stats = Stats {
        total_specs: 3,
        warnings: 1,
        languages: Languages {
            rust: 1,
            typescript: 2,
        },
    };
    ir
}

#[test]
fn IRをエンコードしてデコードすると等価なIRが返る() {
    let ir = full_ir();
    let codec = SerdeJsonCodec;
    let json = codec.encode(&ir).expect("encode");
    let decoded = codec.decode(&json).expect("decode");
    assert_eq!(ir, decoded);
}

#[test]
fn 異なる言語と状態を含むIRもラウンドトリップで等価になる() {
    let ir = ir_with_typescript_and_skipped_specs();
    let codec = SerdeJsonCodec;
    let json = codec.encode(&ir).expect("encode");
    let decoded = codec.decode(&json).expect("decode");
    assert_eq!(ir, decoded);
}

#[test]
fn 複数ソースを持つIRは宣言順を保ってラウンドトリップする() {
    let ir = ir_with_multiple_sources();
    let codec = SerdeJsonCodec;
    let json = codec.encode(&ir).expect("encode");
    let decoded = codec.decode(&json).expect("decode");
    assert_eq!(ir, decoded);
    assert_eq!(decoded.sources[0].name, "Backend");
    assert_eq!(decoded.sources[1].name, "Frontend");
}

#[test]
fn ソースが空のIRもラウンドトリップできる() {
    let ir = IntermediateRepresentation::default();
    let codec = SerdeJsonCodec;
    let json = codec.encode(&ir).expect("encode");
    assert_eq!(codec.decode(&json).expect("decode"), ir);
}

#[test]
fn 方向性ドキュメントで規定されたトップレベルフィールド名が含まれる() {
    let json = SerdeJsonCodec.encode(&full_ir()).unwrap();
    for key in ["project", "sources", "diagnostics", "stats"] {
        assert!(
            json.contains(&format!("\"{key}\"")),
            "missing top-level key: {key}\nactual: {json}"
        );
    }
}

#[test]
fn projectフィールドはnameとextracted_atとrepositoryとrevisionを含む() {
    let json = SerdeJsonCodec.encode(&full_ir()).unwrap();
    for key in ["name", "extracted_at", "repository", "revision"] {
        assert!(
            json.contains(&format!("\"{key}\"")),
            "missing project key: {key}"
        );
    }
}

#[test]
fn sourceフィールドはnameとgroupsを持つ() {
    let json = SerdeJsonCodec.encode(&full_ir()).unwrap();
    assert!(json.contains("\"groups\""), "missing source key: groups");
    assert!(json.contains("\"Backend\""), "missing source name");
}

#[test]
fn revisionがNoneのときフィールド自体が省略される() {
    let json = SerdeJsonCodec
        .encode(&ir_without_optional_project_fields())
        .unwrap();
    assert!(!json.contains("\"revision\""), "revision should be omitted");
    assert!(
        !json.contains("\"repository\""),
        "repository should be omitted"
    );
}

#[test]
fn 言語enumは小文字でシリアライズされる() {
    let json_rust = SerdeJsonCodec.encode(&full_ir()).unwrap();
    assert!(json_rust.contains("\"language\": \"rust\""));

    let json_ts = SerdeJsonCodec
        .encode(&ir_with_typescript_and_skipped_specs())
        .unwrap();
    assert!(json_ts.contains("\"language\": \"typescript\""));
}

#[test]
fn skipped状態は文字列skippedとしてシリアライズされる() {
    let json = SerdeJsonCodec
        .encode(&ir_with_typescript_and_skipped_specs())
        .unwrap();
    assert!(json.contains("\"status\": \"active\""));
    assert!(json.contains("\"status\": \"skipped\""));
}

#[test]
fn diagnostic_codeはsnake_caseでシリアライズされる() {
    let json = SerdeJsonCodec.encode(&full_ir()).unwrap();
    assert!(json.contains("\"code\": \"nesting_too_deep\""));
}

/// すべての `DiagnosticCode` を列挙する。
///
/// `match` を置いているのは網羅性のため。変異体を増やすとここがコンパイル
/// エラーになり、JSON 表現の追従漏れに気づける。
fn all_diagnostic_codes() -> Vec<DiagnosticCode> {
    let all = vec![
        DiagnosticCode::NestingTooDeep,
        DiagnosticCode::EmptyName,
        DiagnosticCode::DynamicTestName,
        DiagnosticCode::GitRevisionUnavailable,
        DiagnosticCode::ParseError,
        DiagnosticCode::FileUnreadable,
    ];
    for code in &all {
        match code {
            DiagnosticCode::NestingTooDeep
            | DiagnosticCode::EmptyName
            | DiagnosticCode::DynamicTestName
            | DiagnosticCode::GitRevisionUnavailable
            | DiagnosticCode::ParseError
            | DiagnosticCode::FileUnreadable => {}
        }
    }
    all
}

#[test]
fn すべてのdiagnostic_codeがラウンドトリップする() {
    for code in all_diagnostic_codes() {
        let mut ir = full_ir();
        ir.diagnostics[0].code = code;

        let codec = SerdeJsonCodec;
        let json = codec.encode(&ir).expect("encode");
        let decoded = codec.decode(&json).expect("decode");

        assert_eq!(decoded.diagnostics[0].code, code);
    }
}

#[test]
fn すべてのdiagnostic_levelがラウンドトリップする() {
    for level in [DiagnosticLevel::Warning, DiagnosticLevel::Error] {
        let mut ir = full_ir();
        ir.diagnostics[0].level = level;

        let codec = SerdeJsonCodec;
        let json = codec.encode(&ir).expect("encode");
        let decoded = codec.decode(&json).expect("decode");

        assert_eq!(decoded.diagnostics[0].level, level);
    }
}

#[test]
fn diagnostic_levelは小文字でシリアライズされる() {
    let json = SerdeJsonCodec.encode(&full_ir()).unwrap();
    assert!(json.contains("\"level\": \"warning\""));
}

#[test]
fn statsは方向性docどおりにtotal_specsとlanguagesを持つ() {
    let json = SerdeJsonCodec.encode(&full_ir()).unwrap();
    for key in ["total_specs", "warnings", "languages", "rust", "typescript"] {
        assert!(
            json.contains(&format!("\"{key}\"")),
            "missing stats key: {key}"
        );
    }
}

#[test]
fn 不正なJSONはJsonDecodeエラーを返す() {
    let result = SerdeJsonCodec.decode("not json at all");
    assert!(matches!(result, Err(SpectoruError::JsonDecode { .. })));
}

#[test]
fn 旧形式のフラグメントはJsonDecodeエラーになる() {
    // project / sources 階層の導入前は `source` 単数 + `groups` がトップレベルだった。
    // 黙って一部だけ読めてしまうと不完全なサイトが生成されるため、明確に失敗させる。
    let legacy = r#"{
        "source": { "name": "Backend", "extracted_at": "2026-04-13T12:00:00Z" },
        "groups": [],
        "diagnostics": [],
        "stats": { "total_specs": 0, "warnings": 0, "languages": { "rust": 0, "typescript": 0 } }
    }"#;
    let result = SerdeJsonCodec.decode(legacy);
    assert!(matches!(result, Err(SpectoruError::JsonDecode { .. })));
}

#[test]
fn Unicodeテスト名がそのまま保持される() {
    let ir = full_ir();
    let codec = SerdeJsonCodec;
    let json = codec.encode(&ir).unwrap();
    assert!(json.contains("作品が公開状態で作成される"));
    let decoded = codec.decode(&json).unwrap();
    assert_eq!(
        decoded.sources[0].groups[0].children[0].specs[0].name,
        "作品が公開状態で作成される"
    );
}
