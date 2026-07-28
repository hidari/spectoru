//! `FragmentStore` と `Renderer` の統合テスト。
//!
//! 複数リポジトリ集約パターンの経路（extract → フラグメント → render）が
//! 往復して壊れないことを確かめる。

#![allow(non_snake_case)]

mod support;

use std::path::{Path, PathBuf};

use spectoru::adapters::serde_json_codec::SerdeJsonCodec;
use spectoru::app::fragment::FragmentStore;
use spectoru::app::render::Renderer;
use spectoru::core::ir::{
    Group, IntermediateRepresentation, Language, ProjectMeta, Source, Spec, SpecStatus, Stats,
};
use spectoru::error::SpectoruError;
use spectoru::ports::json_codec::JsonCodec;
use support::{FailingTemplateEngine, FakeFileWalker, FakeFileWriter, FakeTemplateEngine};

fn encoded(ir: &IntermediateRepresentation) -> String {
    SerdeJsonCodec.encode(ir).expect("encode")
}

fn ir(name: &str, spec_name: &str) -> IntermediateRepresentation {
    IntermediateRepresentation {
        project: ProjectMeta {
            name: name.to_string(),
            repository: None,
            revision: Some("abc1234".to_string()),
            extracted_at: "2026-04-13T12:00:00Z".to_string(),
        },
        sources: vec![Source {
            name: "Backend".to_string(),
            groups: vec![Group {
                name: "tests/artwork.rs".to_string(),
                file: PathBuf::from("tests/artwork.rs"),
                line: None,
                children: vec![],
                specs: vec![Spec {
                    name: spec_name.to_string(),
                    file: PathBuf::from("tests/artwork.rs"),
                    line: 2,
                    language: Language::Rust,
                    status: SpecStatus::Active,
                }],
            }],
        }],
        diagnostics: vec![],
        stats: Stats {
            total_specs: 1,
            ..Stats::default()
        },
    }
}

#[test]
fn フラグメントを書き出して読み戻すと等価なIRになる() {
    let writer = FakeFileWriter::default();
    let original = ir("Astralys", "作品が公開状態で作成される");

    let empty = FakeFileWalker::default();
    let store = FragmentStore {
        json_codec: &SerdeJsonCodec,
        walker: &empty,
        writer: &writer,
    };
    store
        .save(&original, Path::new("out/fragment.json"))
        .expect("save");

    let json = writer
        .contents_of("out/fragment.json")
        .expect("fragment written");
    let walker = FakeFileWalker::new(&[("fragment.json", &json)]);
    let store = FragmentStore {
        json_codec: &SerdeJsonCodec,
        walker: &walker,
        writer: &writer,
    };

    let loaded = store.load_all(&["fragment.json"]).expect("load");

    assert_eq!(loaded, [original]);
}

#[test]
fn 複数のフラグメントを渡した順に読み込む() {
    let writer = FakeFileWriter::default();
    let backend = encoded(&ir("Backend", "作品が作成される"));
    let frontend = encoded(&ir("Frontend", "登録が完了する"));
    let walker = FakeFileWalker::new(&[("backend.json", &backend), ("frontend.json", &frontend)]);
    let store = FragmentStore {
        json_codec: &SerdeJsonCodec,
        walker: &walker,
        writer: &writer,
    };

    let loaded = store
        .load_all(&["frontend.json", "backend.json"])
        .expect("load");

    let names: Vec<&str> = loaded.iter().map(|i| i.project.name.as_str()).collect();
    assert_eq!(names, ["Frontend", "Backend"]);
}

#[test]
fn 壊れたフラグメントはどのファイルが原因か分かる形で失敗する() {
    // `render --fragments a.json b.json` でどれが悪いか言えなければ直せない。
    let writer = FakeFileWriter::default();
    let walker = FakeFileWalker::new(&[
        ("good.json", &encoded(&ir("Good", "テスト"))),
        ("broken.json", "not json at all"),
    ]);
    let store = FragmentStore {
        json_codec: &SerdeJsonCodec,
        walker: &walker,
        writer: &writer,
    };

    let error = store
        .load_all(&["good.json", "broken.json"])
        .expect_err("should fail");

    let SpectoruError::Fragment { path, .. } = error else {
        panic!("expected Fragment error, got {error:?}");
    };
    assert_eq!(path, PathBuf::from("broken.json"));
}

#[test]
fn 存在しないフラグメントはIoエラーになる() {
    let writer = FakeFileWriter::default();
    let walker = FakeFileWalker::default();
    let store = FragmentStore {
        json_codec: &SerdeJsonCodec,
        walker: &walker,
        writer: &writer,
    };

    let result = store.load_all(&["missing.json"]);

    assert!(matches!(result, Err(SpectoruError::Io { .. })));
}

#[test]
fn サイトは出力ディレクトリ直下のindex_htmlに書かれる() {
    let writer = FakeFileWriter::default();
    let renderer = Renderer {
        template: &FakeTemplateEngine,
        writer: &writer,
    };

    renderer
        .render(&[ir("Astralys", "テスト")], Path::new("dist"))
        .expect("render");

    let written = writer.written();
    assert_eq!(written.len(), 1);
    assert_eq!(written[0].0, PathBuf::from("dist/index.html"));
}

#[test]
fn 複数プロジェクトを渡した順にテンプレートへ渡す() {
    // サイドバー最上位の並び順が入力順に一致すること。
    let writer = FakeFileWriter::default();
    let renderer = Renderer {
        template: &FakeTemplateEngine,
        writer: &writer,
    };

    renderer
        .render(
            &[ir("Backend", "テスト"), ir("Frontend", "テスト")],
            Path::new("dist"),
        )
        .expect("render");

    assert_eq!(
        writer.contents_of("dist/index.html").as_deref(),
        Some("projects=Backend,Frontend")
    );
}

#[test]
fn プロジェクトが0件でもサイトを書き出す() {
    let writer = FakeFileWriter::default();
    let renderer = Renderer {
        template: &FakeTemplateEngine,
        writer: &writer,
    };

    renderer.render(&[], Path::new("dist")).expect("render");

    assert_eq!(
        writer.contents_of("dist/index.html").as_deref(),
        Some("projects=")
    );
}

#[test]
fn 描画に失敗したらファイルを書き出さない() {
    // 途中まで書かれた壊れたサイトが残るより、何も残らない方がよい。
    let writer = FakeFileWriter::default();
    let renderer = Renderer {
        template: &FailingTemplateEngine,
        writer: &writer,
    };

    let result = renderer.render(&[ir("Astralys", "テスト")], Path::new("dist"));

    assert!(matches!(result, Err(SpectoruError::TemplateRender { .. })));
    assert_eq!(writer.written(), []);
}
