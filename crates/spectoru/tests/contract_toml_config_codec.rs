//! `TomlConfigCodec` の契約テスト。
//!
//! `spec-site.toml` として何が受理され、何が拒否されるかを固定する。
//! TOML ライブラリを差し替えてもここに書かれた性質は守られなければならない。

#![allow(non_snake_case)]

use std::path::{Path, PathBuf};

use spectoru::adapters::toml_config_codec::TomlConfigCodec;
use spectoru::core::config::{SourceKind, SpectoruConfig};
use spectoru::error::SpectoruError;
use spectoru::ports::toml_codec::TomlCodec;

fn decode(source: &str) -> Result<SpectoruConfig, SpectoruError> {
    TomlConfigCodec.decode_config(Path::new("spec-site.toml"), source)
}

fn decoded(source: &str) -> SpectoruConfig {
    decode(source).unwrap_or_else(|e| panic!("decode failed: {e}"))
}

/// 方向性ドキュメントに載っている完全な設定例。
const FULL: &str = r#"
[project]
name = "Astralys"
repository = "https://github.com/HermitianHQ/astralys"

[[sources]]
name = "Backend"
kind = "rust"
paths = ["src/", "tests/"]

[[sources]]
name = "Frontend"
kind = "vitest"
paths = ["app/"]

[lint]
max_depth = 6
"#;

/// 必須項目だけの最小構成。
const MINIMAL: &str = r#"
[project]
name = "Minimal"

[[sources]]
name = "Backend"
kind = "rust"
paths = ["tests/"]
"#;

#[test]
fn 方向性ドキュメントの設定例をそのまま読める() {
    let config = decoded(FULL);
    assert_eq!(config.project.name, "Astralys");
    assert_eq!(
        config.project.repository.as_deref(),
        Some("https://github.com/HermitianHQ/astralys")
    );
    assert_eq!(config.lint.max_depth, 6);
}

#[test]
fn 複数のsourceを宣言順に読む() {
    let config = decoded(FULL);
    let names: Vec<&str> = config
        .sources
        .iter()
        .map(|source| source.name.as_str())
        .collect();
    assert_eq!(names, ["Backend", "Frontend"]);
    assert_eq!(config.sources[0].kind, SourceKind::Rust);
    assert_eq!(config.sources[1].kind, SourceKind::Vitest);
    assert_eq!(
        config.sources[0].paths,
        [PathBuf::from("src/"), PathBuf::from("tests/")]
    );
}

#[test]
fn excludeを省略すると空になる() {
    assert_eq!(decoded(MINIMAL).sources[0].exclude, Vec::<PathBuf>::new());
}

#[test]
fn excludeで探索対象から外すパスを指定できる() {
    let source = r#"
[project]
name = "Minimal"

[[sources]]
name = "Tests"
kind = "rust"
paths = ["tests/"]
exclude = ["tests/fixtures/", "tests/support/"]
"#;
    assert_eq!(
        decoded(source).sources[0].exclude,
        [
            PathBuf::from("tests/fixtures/"),
            PathBuf::from("tests/support/")
        ]
    );
}

#[test]
fn repositoryは省略できる() {
    assert_eq!(decoded(MINIMAL).project.repository, None);
}

#[test]
fn lintセクションを省略するとmax_depthは4になる() {
    // デフォルト値はドメイン側の LintConfig::default() に由来する。
    assert_eq!(decoded(MINIMAL).lint.max_depth, 4);
}

#[test]
fn lintセクションはあってもmax_depthを省略できる() {
    let source = format!("{MINIMAL}\n[lint]\n");
    assert_eq!(decoded(&source).lint.max_depth, 4);
}

#[test]
fn 未知のキーは黙って無視せずエラーにする() {
    // 打ち間違えた設定が無視されると「設定したのに効かない」という
    // 最も分かりにくい失敗になる。
    let source = format!("{MINIMAL}\nunknown_key = 1\n");
    assert!(matches!(
        decode(&source),
        Err(SpectoruError::TomlParse { .. })
    ));
}

#[test]
fn sourceの未知のキーもエラーにする() {
    let source = r#"
[project]
name = "Minimal"

[[sources]]
name = "Backend"
kind = "rust"
paths = ["tests/"]
repository = "https://example.com"
"#;
    assert!(matches!(
        decode(source),
        Err(SpectoruError::TomlParse { .. })
    ));
}

#[test]
fn 未知のkindはエラーにする() {
    let source = r#"
[project]
name = "Minimal"

[[sources]]
name = "Backend"
kind = "jest"
paths = ["tests/"]
"#;
    assert!(matches!(
        decode(source),
        Err(SpectoruError::TomlParse { .. })
    ));
}

#[test]
fn projectセクションが無ければエラーにする() {
    let source = r#"
[[sources]]
name = "Backend"
kind = "rust"
paths = ["tests/"]
"#;
    assert!(matches!(
        decode(source),
        Err(SpectoruError::TomlParse { .. })
    ));
}

#[test]
fn sourceが1つも無ければエラーにする() {
    // 抽出対象が無い設定は書き間違いとしか考えられない。
    let source = r#"
[project]
name = "Minimal"
"#;
    assert!(matches!(
        decode(source),
        Err(SpectoruError::TomlParse { .. })
    ));
}

#[test]
fn pathsが空のsourceはエラーにする() {
    let source = r#"
[project]
name = "Minimal"

[[sources]]
name = "Backend"
kind = "rust"
paths = []
"#;
    assert!(matches!(
        decode(source),
        Err(SpectoruError::TomlParse { .. })
    ));
}

#[test]
fn max_depthが0ならエラーにする() {
    // 0 だとあらゆるグループが違反になり、設定として意味をなさない。
    let source = format!("{MINIMAL}\n[lint]\nmax_depth = 0\n");
    assert!(matches!(
        decode(&source),
        Err(SpectoruError::TomlParse { .. })
    ));
}

#[test]
fn 壊れたTOMLはTomlParseエラーになる() {
    assert!(matches!(
        decode("[project"),
        Err(SpectoruError::TomlParse { .. })
    ));
}

#[test]
fn エラーには設定ファイルのパスが含まれる() {
    // どのファイルを直せばよいかが分からなければエラーの意味がない。
    let error = TomlConfigCodec
        .decode_config(Path::new("config/spec-site.toml"), "[project")
        .expect_err("should fail");
    let SpectoruError::TomlParse { path, .. } = error else {
        panic!("expected TomlParse");
    };
    assert_eq!(path, PathBuf::from("config/spec-site.toml"));
}

#[test]
fn 日本語のプロジェクト名を扱える() {
    let source = r#"
[project]
name = "アストラリス"

[[sources]]
name = "バックエンド"
kind = "rust"
paths = ["tests/"]
"#;
    let config = decoded(source);
    assert_eq!(config.project.name, "アストラリス");
    assert_eq!(config.sources[0].name, "バックエンド");
}
