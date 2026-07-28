//! 実バイナリに対する E2E テスト。
//!
//! ここまでの層別テストは、合成が正しいことまでは保証しない。実際に
//! `spectoru build` を叩いて成果物と終了コードを確かめるのがこのファイルの役割。
//!
//! 終了コードの約束:
//!
//! | 状況 | code |
//! |---|---|
//! | 正常終了 | 0 |
//! | 品質ゲートに掛かった | 1 |
//! | spectoru が動けなかった | 2 |

#![allow(non_snake_case)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

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

/// 標準的なプロジェクトを一時ディレクトリに作る。
fn project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    for (relative, contents) in files {
        let path = dir.path().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(&path, contents).expect("write");
    }
    dir
}

fn default_project() -> TempDir {
    project(&[
        ("spec-site.toml", CONFIG),
        ("src/artwork.rs", RUST_TEST),
        ("app/registration.test.ts", TS_TEST),
    ])
}

fn spectoru(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_spectoru"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("spectoru を実行できる")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("終了コード")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// 一時ディレクトリは git リポジトリではないため、revision を明示しないと
/// `git_revision_unavailable` の警告が必ず出る。挙動を切り分けたいテストでは
/// これを渡してノイズを消す。
const REVISION: [&str; 2] = ["--revision", "e2e"];

#[test]
fn buildが静的サイトを生成して正常終了する() {
    let dir = default_project();

    let output = spectoru(dir.path(), &["build", REVISION[0], REVISION[1]]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let html = fs::read_to_string(dir.path().join("dist/index.html")).expect("生成されている");
    assert!(html.contains("作品が公開状態で作成される"));
    assert!(html.contains("招待リンクから登録が完了する"));
    assert!(html.contains("Astralys"));
}

#[test]
fn 出力先ディレクトリは指定できる() {
    let dir = default_project();

    let output = spectoru(
        dir.path(),
        &["build", "--out", "public/site", REVISION[0], REVISION[1]],
    );

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(dir.path().join("public/site/index.html").exists());
}

#[test]
fn extractしたフラグメントをrenderに渡してサイトを生成できる() {
    // 複数リポジトリ集約パターンの経路そのもの。
    let dir = default_project();

    let extract = spectoru(dir.path(), &["extract", REVISION[0], REVISION[1]]);
    assert_eq!(code(&extract), 0, "{}", stderr(&extract));

    let fragment = fs::read_to_string(dir.path().join("spec-fragment.json")).expect("フラグメント");
    assert!(fragment.contains("\"project\""));
    assert!(fragment.contains("作品が公開状態で作成される"));

    let render = spectoru(dir.path(), &["render", "--fragments", "spec-fragment.json"]);
    assert_eq!(code(&render), 0, "{}", stderr(&render));

    let html = fs::read_to_string(dir.path().join("dist/index.html")).expect("生成されている");
    assert!(html.contains("作品が公開状態で作成される"));
}

#[test]
fn 複数のフラグメントを1つのサイトに集約できる() {
    let dir = default_project();

    assert_eq!(
        code(&spectoru(
            dir.path(),
            &["extract", "--out", "backend.json", REVISION[0], REVISION[1]],
        )),
        0
    );
    // 2 つ目は別プロジェクト名で抽出する。
    fs::write(
        dir.path().join("other.toml"),
        CONFIG.replace("Astralys", "Lumina"),
    )
    .expect("write");
    assert_eq!(
        code(&spectoru(
            dir.path(),
            &[
                "extract",
                "--config",
                "other.toml",
                "--out",
                "frontend.json",
                REVISION[0],
                REVISION[1],
            ],
        )),
        0
    );

    let render = spectoru(
        dir.path(),
        &["render", "--fragments", "backend.json", "frontend.json"],
    );

    assert_eq!(code(&render), 0, "{}", stderr(&render));
    let html = fs::read_to_string(dir.path().join("dist/index.html")).expect("生成されている");
    assert!(html.contains("Astralys"));
    assert!(html.contains("Lumina"));
}

#[test]
fn lintは成果物を書き出さない() {
    let dir = default_project();

    let output = spectoru(dir.path(), &["lint", REVISION[0], REVISION[1]]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        !dir.path().join("dist").exists(),
        "サイトを書いてはいけない"
    );
}

#[test]
fn 警告があってもstrictでなければ正常終了する() {
    let dir = project(&[
        ("spec-site.toml", CONFIG),
        ("src/artwork.rs", RUST_TEST),
        ("app/registration.test.ts", "it(\"\", () => {});\n"),
    ]);

    let output = spectoru(dir.path(), &["lint", REVISION[0], REVISION[1]]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(stderr(&output).contains("empty_name"));
}

#[test]
fn strictなら警告1件で品質ゲートに掛かる() {
    let dir = project(&[
        ("spec-site.toml", CONFIG),
        ("src/artwork.rs", RUST_TEST),
        ("app/registration.test.ts", "it(\"\", () => {});\n"),
    ]);

    let output = spectoru(dir.path(), &["lint", "--strict", REVISION[0], REVISION[1]]);

    assert_eq!(code(&output), 1, "{}", stderr(&output));
}

#[test]
fn errorはstrictでなくても品質ゲートに掛かる() {
    // 解釈できないファイルがあると、その仕様はサイトから丸ごと欠落する。
    let dir = project(&[
        ("spec-site.toml", CONFIG),
        ("src/artwork.rs", RUST_TEST),
        ("src/broken.rs", "fn broken( {\n"),
        ("app/registration.test.ts", TS_TEST),
    ]);

    let output = spectoru(dir.path(), &["lint", REVISION[0], REVISION[1]]);

    assert_eq!(code(&output), 1, "{}", stderr(&output));
    assert!(stderr(&output).contains("parse_error"));
}

#[test]
fn errorがあってもサイトの生成自体は続行する() {
    // 何も出さないより、欠落を明示しつつ読める分を見せる方が有益。
    let dir = project(&[
        ("spec-site.toml", CONFIG),
        ("src/artwork.rs", RUST_TEST),
        ("src/broken.rs", "fn broken( {\n"),
        ("app/registration.test.ts", TS_TEST),
    ]);

    let output = spectoru(dir.path(), &["build", REVISION[0], REVISION[1]]);

    assert_eq!(code(&output), 1);
    let html = fs::read_to_string(dir.path().join("dist/index.html")).expect("生成されている");
    assert!(html.contains("作品が公開状態で作成される"));
}

#[test]
fn 診断はファイルと行が分かる形で報告される() {
    let dir = project(&[
        ("spec-site.toml", CONFIG),
        ("src/artwork.rs", RUST_TEST),
        ("app/registration.test.ts", "it(\"\", () => {});\n"),
    ]);

    let output = spectoru(dir.path(), &["lint", REVISION[0], REVISION[1]]);

    assert!(
        stderr(&output).contains("app/registration.test.ts:1: warning[empty_name]"),
        "got {}",
        stderr(&output)
    );
}

#[test]
fn revisionを渡さない環境ではgitの警告が出る() {
    let dir = default_project();

    let output = spectoru(dir.path(), &["lint"]);

    assert_eq!(code(&output), 0);
    assert!(stderr(&output).contains("git_revision_unavailable"));
}

#[test]
fn 設定ファイルが無ければ実行エラーになる() {
    // 品質ゲート（1）とは区別できること。
    let dir = TempDir::new().expect("tempdir");

    let output = spectoru(dir.path(), &["build"]);

    assert_eq!(code(&output), 2, "{}", stderr(&output));
    assert!(stderr(&output).contains("spectoru:"));
}

#[test]
fn 壊れた設定ファイルは実行エラーになる() {
    let dir = project(&[("spec-site.toml", "[project")]);

    let output = spectoru(dir.path(), &["build"]);

    assert_eq!(code(&output), 2, "{}", stderr(&output));
}

#[test]
fn 存在しない探索パスは実行エラーになる() {
    let dir = project(&[("spec-site.toml", CONFIG)]);

    let output = spectoru(dir.path(), &["build", REVISION[0], REVISION[1]]);

    assert_eq!(code(&output), 2, "{}", stderr(&output));
}

#[test]
fn 壊れたフラグメントはどれが原因か分かる形で実行エラーになる() {
    let dir = project(&[("broken.json", "not json at all")]);

    let output = spectoru(dir.path(), &["render", "--fragments", "broken.json"]);

    assert_eq!(code(&output), 2);
    assert!(
        stderr(&output).contains("broken.json"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn 未知のサブコマンドは実行エラーになる() {
    let dir = TempDir::new().expect("tempdir");

    let output = spectoru(dir.path(), &["publish"]);

    assert_eq!(code(&output), 2);
}

#[test]
fn helpは使い方を表示して正常終了する() {
    let dir = TempDir::new().expect("tempdir");

    let output = spectoru(dir.path(), &["--help"]);

    assert_eq!(code(&output), 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for subcommand in ["build", "extract", "render", "lint"] {
        assert!(stdout.contains(subcommand), "got {stdout}");
    }
}

#[test]
fn versionを表示して正常終了する() {
    let dir = TempDir::new().expect("tempdir");

    let output = spectoru(dir.path(), &["--version"]);

    assert_eq!(code(&output), 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn 生成サイトはテスト名をエスケープする() {
    // パイプライン全体を通した XSS の回帰テスト。層別の契約テストが通っていても、
    // 配線を誤れば生の値が出力されうる。
    // Rust の識別子には `<` を書けないため、注入経路は Vitest 側の文字列になる。
    let dir = project(&[
        ("spec-site.toml", CONFIG),
        ("src/artwork.rs", RUST_TEST),
        (
            "app/xss.test.ts",
            "it(\"<script>alert(1)</script>\", () => {});\n",
        ),
    ]);

    let output = spectoru(dir.path(), &["build", REVISION[0], REVISION[1]]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    let html = fs::read_to_string(dir.path().join("dist/index.html")).expect("生成されている");
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
}

#[test]
fn 生成サイトは外部ホストからリソースを読み込まない() {
    let dir = default_project();

    assert_eq!(
        code(&spectoru(dir.path(), &["build", REVISION[0], REVISION[1]])),
        0
    );

    let html = fs::read_to_string(dir.path().join("dist/index.html")).expect("生成されている");
    for forbidden in ["<script src", "<link ", "<img ", "@import", "fetch("] {
        assert!(!html.contains(forbidden), "外部依存の疑い: {forbidden}");
    }
}

#[test]
fn 同じ入力からは常に同じフラグメントが出る() {
    // 集約パターンでは差分が読めることが前提になる。
    let dir = default_project();

    assert_eq!(
        code(&spectoru(
            dir.path(),
            &["extract", "--out", "a.json", REVISION[0], REVISION[1]]
        )),
        0
    );
    assert_eq!(
        code(&spectoru(
            dir.path(),
            &["extract", "--out", "b.json", REVISION[0], REVISION[1]]
        )),
        0
    );

    let a = fs::read_to_string(dir.path().join("a.json")).expect("a");
    let b = fs::read_to_string(dir.path().join("b.json")).expect("b");
    // extracted_at だけは実行時刻なので比較から外す。
    let strip = |json: &str| {
        json.lines()
            .filter(|line| !line.contains("extracted_at"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(strip(&a), strip(&b));
}
