//! `Extractor` の統合テスト。
//!
//! I/O ポートだけを Fake に差し替え、設定の解釈・ファイル探索・パース・lint・
//! 集計が 1 本のユースケースとして噛み合うことを確かめる。TOML codec と両パーサは
//! 本物を使うため、「実際にパースできるのか」もここで担保される。

#![allow(non_snake_case)]

mod support;

use std::path::{Path, PathBuf};

use spectoru::adapters::syn_rust_parser::SynRustParser;
use spectoru::adapters::toml_config_codec::TomlConfigCodec;
use spectoru::adapters::tree_sitter_ts_parser::TreeSitterTsParser;
use spectoru::app::extract::Extractor;
use spectoru::core::ir::{
    Diagnostic, DiagnosticCode, DiagnosticLevel, IntermediateRepresentation, Language, SpecStatus,
};
use spectoru::error::SpectoruError;
use spectoru::ports::file_walker::FileWalker;
use support::{FakeFileWalker, FakeGitProvider, FixedClock, UnreadableFileWalker};

const REVISION: &str = "abc1234";
const EXTRACTED_AT: &str = "2026-04-13T12:00:00Z";

const CONFIG: &str = r#"
[project]
name = "Astralys"
repository = "https://github.com/HermitianHQ/astralys"

[[sources]]
name = "Backend"
kind = "rust"
paths = ["src/"]

[[sources]]
name = "Frontend"
kind = "vitest"
paths = ["app/"]
"#;

const RUST_TEST: &str = "#[test]\nfn 作品が公開状態で作成される() {}\n";
const TS_TEST: &str = "it(\"招待リンクから登録が完了する\", () => {});\n";

fn run(
    walker: &dyn FileWalker,
    revision_from_git: Option<&'static str>,
    config_path: &str,
    revision_override: Option<&str>,
) -> Result<IntermediateRepresentation, SpectoruError> {
    let git = FakeGitProvider(revision_from_git);
    let clock = FixedClock::default();
    let extractor = Extractor {
        config_codec: &TomlConfigCodec,
        walker,
        rust_parser: &SynRustParser,
        ts_parser: &TreeSitterTsParser,
        git: &git,
        clock: &clock,
    };
    extractor.extract(Path::new(config_path), revision_override)
}

/// 標準的な構成での抽出。git revision は取得できる前提。
fn extract(files: &[(&str, &str)]) -> IntermediateRepresentation {
    let walker = FakeFileWalker::new(files);
    run(&walker, Some(REVISION), "spec-site.toml", None).expect("extract")
}

fn default_tree() -> Vec<(&'static str, &'static str)> {
    vec![
        ("spec-site.toml", CONFIG),
        ("src/artwork.rs", RUST_TEST),
        ("app/registration.test.ts", TS_TEST),
    ]
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<DiagnosticCode> {
    diagnostics.iter().map(|d| d.code).collect()
}

#[test]
fn 設定に宣言されたsourceを宣言順に抽出する() {
    let ir = extract(&default_tree());

    let names: Vec<&str> = ir.sources.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Backend", "Frontend"]);
}

#[test]
fn RustとVitestの両方を1つのIRにまとめる() {
    let ir = extract(&default_tree());

    assert_eq!(
        ir.sources[0].groups[0].specs[0].name,
        "作品が公開状態で作成される"
    );
    assert_eq!(
        ir.sources[1].groups[0].specs[0].name,
        "招待リンクから登録が完了する"
    );
    assert_eq!(ir.sources[0].groups[0].specs[0].language, Language::Rust);
    assert_eq!(
        ir.sources[1].groups[0].specs[0].language,
        Language::TypeScript
    );
}

#[test]
fn projectメタは設定とgitと時計から埋まる() {
    let ir = extract(&default_tree());

    assert_eq!(ir.project.name, "Astralys");
    assert_eq!(
        ir.project.repository.as_deref(),
        Some("https://github.com/HermitianHQ/astralys")
    );
    assert_eq!(ir.project.revision.as_deref(), Some(REVISION));
    assert_eq!(ir.project.extracted_at, EXTRACTED_AT);
}

#[test]
fn ファイルパスは設定ファイルのあるディレクトリからの相対になる() {
    // 生成サイトのパス表記が、利用者がリポジトリルートで見る表記と一致すること。
    let walker = FakeFileWalker::new(&[
        ("repo/spec-site.toml", CONFIG),
        ("repo/src/artwork.rs", RUST_TEST),
        ("repo/app/registration.test.ts", TS_TEST),
    ]);

    let ir = run(&walker, Some(REVISION), "repo/spec-site.toml", None).expect("extract");

    assert_eq!(ir.sources[0].groups[0].name, "src/artwork.rs");
    assert_eq!(
        ir.sources[0].groups[0].specs[0].file,
        PathBuf::from("src/artwork.rs")
    );
    assert_eq!(ir.sources[1].groups[0].name, "app/registration.test.ts");
}

#[test]
fn revisionを明示するとgitを参照しない() {
    // shallow clone やコンテナ内ビルドなど、git メタデータが無い CI 向けの逃げ道。
    let walker = FakeFileWalker::new(&default_tree());

    let ir = run(&walker, None, "spec-site.toml", Some("deadbeef")).expect("extract");

    assert_eq!(ir.project.revision.as_deref(), Some("deadbeef"));
    assert_eq!(codes(&ir.diagnostics), []);
}

#[test]
fn gitのrevisionが取れないと警告を積むが抽出は続行する() {
    let walker = FakeFileWalker::new(&default_tree());

    let ir = run(&walker, None, "spec-site.toml", None).expect("extract");

    assert_eq!(ir.project.revision, None);
    assert_eq!(
        codes(&ir.diagnostics),
        [DiagnosticCode::GitRevisionUnavailable]
    );
    assert_eq!(ir.diagnostics[0].level, DiagnosticLevel::Warning);
    assert_eq!(ir.stats.total_specs, 2);
}

#[test]
fn パースできないファイルはerror診断になり他のファイルは抽出される() {
    // 1 ファイルの構文エラーで全体が止まると、他の仕様まで見えなくなる。
    let mut files = default_tree();
    files.push(("src/broken.rs", "fn broken( {\n"));
    let ir = extract(&files);

    assert_eq!(codes(&ir.diagnostics), [DiagnosticCode::ParseError]);
    assert_eq!(ir.diagnostics[0].level, DiagnosticLevel::Error);
    assert_eq!(ir.diagnostics[0].file, Some(PathBuf::from("src/broken.rs")));
    assert_eq!(ir.stats.total_specs, 2);
}

#[test]
fn 読み込めないファイルはFileUnreadable診断になり他のファイルは抽出される() {
    let walker = UnreadableFileWalker::new(
        &[
            ("spec-site.toml", CONFIG),
            ("src/artwork.rs", RUST_TEST),
            ("src/locked.rs", RUST_TEST),
            ("app/registration.test.ts", TS_TEST),
        ],
        "src/locked.rs",
    );

    let ir = run(&walker, Some(REVISION), "spec-site.toml", None).expect("extract");

    assert_eq!(codes(&ir.diagnostics), [DiagnosticCode::FileUnreadable]);
    assert_eq!(ir.diagnostics[0].level, DiagnosticLevel::Error);
    assert_eq!(ir.diagnostics[0].file, Some(PathBuf::from("src/locked.rs")));
    assert_eq!(ir.stats.total_specs, 2);
}

#[test]
fn テストを含まないファイルはグループにならない() {
    let mut files = default_tree();
    files.push(("src/helpers.rs", "pub fn build() {}\n"));
    let ir = extract(&files);

    let group_names: Vec<&str> = ir.sources[0]
        .groups
        .iter()
        .map(|group| group.name.as_str())
        .collect();
    assert_eq!(group_names, ["src/artwork.rs"]);
}

#[test]
fn パーサが出した診断がIRに引き継がれる() {
    let mut files = default_tree();
    files.push((
        "app/dynamic.test.ts",
        "const x = 1;\nit(`${x} のとき`, () => {});\n",
    ));
    let ir = extract(&files);

    assert_eq!(codes(&ir.diagnostics), [DiagnosticCode::DynamicTestName]);
}

#[test]
fn lint違反が診断とstatsのwarningsの両方に反映される() {
    let config = r#"
[project]
name = "Astralys"

[[sources]]
name = "Frontend"
kind = "vitest"
paths = ["app/"]

[lint]
max_depth = 1
"#;
    // ファイル直下のグループが深さ 1。describe はそれを超える。
    let ir = extract(&[
        ("spec-site.toml", config),
        (
            "app/nested.test.ts",
            "describe(\"条件\", () => {\n  it(\"結果\", () => {});\n});\n",
        ),
    ]);

    assert_eq!(codes(&ir.diagnostics), [DiagnosticCode::NestingTooDeep]);
    assert_eq!(ir.stats.warnings, 1);
}

#[test]
fn 空のテスト名は警告になる() {
    let ir = extract(&[
        ("spec-site.toml", CONFIG),
        ("src/artwork.rs", RUST_TEST),
        ("app/empty.test.ts", "it(\"\", () => {});\n"),
    ]);

    assert_eq!(codes(&ir.diagnostics), [DiagnosticCode::EmptyName]);
}

#[test]
fn 診断はgitパーサlintの順に決定的な順序で並ぶ() {
    let config = r#"
[project]
name = "Astralys"

[[sources]]
name = "Frontend"
kind = "vitest"
paths = ["app/"]

[lint]
max_depth = 1
"#;
    let walker = FakeFileWalker::new(&[
        ("spec-site.toml", config),
        (
            "app/nested.test.ts",
            "describe(\"条件\", () => {\n  it(\"\", () => {});\n});\n",
        ),
    ]);

    let ir = run(&walker, None, "spec-site.toml", None).expect("extract");

    assert_eq!(
        codes(&ir.diagnostics),
        [
            DiagnosticCode::GitRevisionUnavailable,
            DiagnosticCode::NestingTooDeep,
            DiagnosticCode::EmptyName,
        ]
    );
}

#[test]
fn 言語別の内訳を集計する() {
    let ir = extract(&default_tree());

    assert_eq!(ir.stats.total_specs, 2);
    assert_eq!(ir.stats.languages.rust, 1);
    assert_eq!(ir.stats.languages.typescript, 1);
    assert_eq!(ir.stats.warnings, 0);
}

#[test]
fn skippedなテストもspec数に数える() {
    let ir = extract(&[
        ("spec-site.toml", CONFIG),
        (
            "src/artwork.rs",
            "#[test]\n#[ignore]\nfn 通常は実行しない() {}\n",
        ),
        (
            "app/registration.test.ts",
            "it.skip(\"未実装\", () => {});\n",
        ),
    ]);

    assert_eq!(ir.stats.total_specs, 2);
    assert_eq!(ir.sources[0].groups[0].specs[0].status, SpecStatus::Skipped);
    assert_eq!(ir.sources[1].groups[0].specs[0].status, SpecStatus::Skipped);
}

#[test]
fn テストが1件も無いプロジェクトでも成功する() {
    let ir = extract(&[
        ("spec-site.toml", CONFIG),
        ("src/helpers.rs", "pub fn build() {}\n"),
        ("app/setup.ts", "export const config = {};\n"),
    ]);

    assert_eq!(ir.stats.total_specs, 0);
    assert!(ir.sources.iter().all(|source| source.groups.is_empty()));
    assert_eq!(ir.sources.len(), 2, "source 自体は宣言どおり残る");
}

#[test]
fn 同じ入力からは常に同じIRが出る() {
    // フラグメントの差分が読めることが集約パターンの前提になる。
    assert_eq!(extract(&default_tree()), extract(&default_tree()));
}

#[test]
fn 設定ファイルが読めなければエラーになる() {
    let walker = FakeFileWalker::new(&[("other.toml", CONFIG)]);

    let result = run(&walker, Some(REVISION), "spec-site.toml", None);

    assert!(matches!(result, Err(SpectoruError::Io { .. })));
}

#[test]
fn 壊れた設定ファイルはエラーになる() {
    let walker = FakeFileWalker::new(&[("spec-site.toml", "[project")]);

    let result = run(&walker, Some(REVISION), "spec-site.toml", None);

    assert!(matches!(result, Err(SpectoruError::TomlParse { .. })));
}

#[test]
fn 存在しないpathsを指定するとエラーになる() {
    // 設定の打ち間違いが「なぜか何も出ない」形で現れるのを防ぐ。
    let walker = FakeFileWalker::new(&[("spec-site.toml", CONFIG)]);

    let result = run(&walker, Some(REVISION), "spec-site.toml", None);

    assert!(matches!(result, Err(SpectoruError::FileWalk { .. })));
}
