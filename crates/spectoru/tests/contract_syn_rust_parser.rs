//! `SynRustParser` の契約テスト。
//!
//! 「Rust のテストソースを spectoru がどう仕様として解釈するか」を固定する。
//! 検証対象は `tests/fixtures/rust/` 配下のソースであり、フィクスチャ自体が
//! 読める仕様書になるよう書かれている。
//!
//! ここに書かれた性質は `syn` を別のパーサに差し替えても守られなければならない。

#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};

use spectoru::adapters::syn_rust_parser::SynRustParser;
use spectoru::core::ir::{Group, Language, Spec, SpecStatus};
use spectoru::error::SpectoruError;
use spectoru::ports::rust_parser::{ParsedFile, RustParser};

/// フィクスチャを読み、表示用パスは相対パスのままパースする。
fn parse(name: &str) -> ParsedFile {
    let relative = PathBuf::from("tests/fixtures/rust").join(name);
    let absolute = Path::new(env!("CARGO_MANIFEST_DIR")).join(&relative);
    let source = fs::read_to_string(&absolute)
        .unwrap_or_else(|e| panic!("fixture {}: {e}", absolute.display()));
    SynRustParser
        .parse_file(&relative, &source)
        .unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

fn spec_summary(specs: &[Spec]) -> Vec<(&str, u32, SpecStatus)> {
    specs
        .iter()
        .map(|spec| (spec.name.as_str(), spec.line, spec.status))
        .collect()
}

fn child<'a>(group: &'a Group, name: &str) -> &'a Group {
    group
        .children
        .iter()
        .find(|child| child.name == name)
        .unwrap_or_else(|| {
            let found: Vec<&str> = group.children.iter().map(|c| c.name.as_str()).collect();
            panic!("child `{name}` not found; got {found:?}")
        })
}

#[test]
fn ファイルパスが最上位グループになる() {
    let parsed = parse("flat.rs");
    assert_eq!(parsed.group.name, "tests/fixtures/rust/flat.rs");
    assert_eq!(
        parsed.group.file,
        PathBuf::from("tests/fixtures/rust/flat.rs")
    );
    assert_eq!(parsed.group.line, None);
}

#[test]
fn フラットなテストはグループ直下に並ぶ() {
    let parsed = parse("flat.rs");
    assert_eq!(
        spec_summary(&parsed.group.specs),
        [
            (
                "招待リンクからクリエイター登録が完了する",
                5,
                SpecStatus::Active
            ),
            ("無効な招待コードでは登録できない", 8, SpecStatus::Active),
            (
                "外部サービスが必要なため通常は実行しない",
                12,
                SpecStatus::Skipped
            ),
            ("returns_error_when_title_is_empty", 17, SpecStatus::Active),
        ]
    );
    assert_eq!(parsed.group.children, []);
}

#[test]
fn テスト名は一切変換されずそのまま仕様文になる() {
    // 日本語はもちろん、snake_case をスペース区切りに開くような変換も行わない。
    // テスト名がそのまま読める仕様文であることは書き手側の責任とする。
    let parsed = parse("flat.rs");
    assert_eq!(
        parsed.group.specs[0].name,
        "招待リンクからクリエイター登録が完了する"
    );
    assert_eq!(
        parsed.group.specs[3].name,
        "returns_error_when_title_is_empty"
    );
}

#[test]
fn testアトリビュートを持たない関数は無視される() {
    let parsed = parse("flat.rs");
    assert!(
        !parsed
            .group
            .specs
            .iter()
            .any(|spec| spec.name == "ヘルパー関数はテストではない")
    );
}

#[test]
fn tokio_testなど別ランタイムのアトリビュートも認識する() {
    // 判定はパスの最終セグメントが `test` であることのみに依存するため、
    // `#[test]` と `#[tokio::test]` を区別せず同じ規則で拾える。
    let parsed = parse("flat.rs");
    assert_eq!(parsed.group.specs.len(), 4);
}

#[test]
fn ignoreが付いたテストはskipped状態になる() {
    let parsed = parse("flat.rs");
    let ignored = &parsed.group.specs[2];
    assert_eq!(ignored.status, SpecStatus::Skipped);
}

#[test]
fn 言語はrustとして記録される() {
    let parsed = parse("flat.rs");
    assert!(
        parsed
            .group
            .specs
            .iter()
            .all(|spec| spec.language == Language::Rust)
    );
}

#[test]
fn modがサブグループになる() {
    let parsed = parse("nested.rs");
    let group = child(&parsed.group, "有効な画像がアップロードされたとき");
    assert_eq!(group.line, Some(3));
    assert_eq!(
        spec_summary(&group.specs),
        [
            ("作品が公開状態で作成される", 5, SpecStatus::Active),
            (
                "コラボレーターにクレジットが付与される",
                8,
                SpecStatus::Active
            ),
        ]
    );
}

#[test]
fn modは任意の深さでネストできる() {
    let parsed = parse("nested.rs");
    let outer = child(&parsed.group, "タイトルが未入力のとき");
    let inner = child(outer, "さらに説明も未入力のとき");
    assert_eq!(inner.line, Some(12));
    assert_eq!(
        spec_summary(&inner.specs),
        [("バリデーションエラーが返される", 14, SpecStatus::Active)]
    );
}

#[test]
fn テストを含まないmodはグループにならない() {
    let parsed = parse("nested.rs");
    let names: Vec<&str> = parsed
        .group
        .children
        .iter()
        .map(|child| child.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "有効な画像がアップロードされたとき",
            "タイトルが未入力のとき"
        ]
    );
}

#[test]
fn 容器としてのmod_testsは階層から取り除かれる() {
    // `mod tests` は仕様文としての意味を持たないため、中身を親に引き上げる。
    let parsed = parse("unit_tests.rs");
    assert!(
        !parsed.group.children.iter().any(|c| c.name == "tests"),
        "`tests` グループが残っている"
    );
    assert_eq!(
        spec_summary(&parsed.group.specs),
        [("正の数どうしを足せる", 13, SpecStatus::Active)]
    );
}

#[test]
fn 取り除かれたmod_testsの子グループは親に引き上げられる() {
    let parsed = parse("unit_tests.rs");
    let group = child(&parsed.group, "負の数を含むとき");
    assert_eq!(group.line, Some(15));
    assert_eq!(
        spec_summary(&group.specs),
        [("符号を保って計算する", 17, SpecStatus::Active)]
    );
}

#[test]
fn 本体を持たないmod宣言はグループを作らない() {
    // `mod foo;` の指し先は FileWalker が別ファイルとして独立に拾う。
    let parsed = parse("external_mod.rs");
    assert_eq!(parsed.group.children, []);
    assert_eq!(
        spec_summary(&parsed.group.specs),
        [("このファイル自身のテスト", 7, SpecStatus::Active)]
    );
}

#[test]
fn テストを含まないファイルは空のグループになる() {
    let parsed = parse("no_tests.rs");
    assert_eq!(parsed.group.specs, []);
    assert_eq!(parsed.group.children, []);
}

#[test]
fn 正常にパースできたファイルは診断を出さない() {
    for name in ["flat.rs", "nested.rs", "unit_tests.rs", "no_tests.rs"] {
        let parsed = parse(name);
        assert_eq!(parsed.diagnostics, [], "{name} が診断を出している");
    }
}

#[test]
fn 構文エラーを含むファイルはRustParseエラーになる() {
    // syn は部分的な AST を返さないため、読めたところまでで続行はできない。
    let relative = PathBuf::from("tests/fixtures/rust/invalid.rs");
    let absolute = Path::new(env!("CARGO_MANIFEST_DIR")).join(&relative);
    let source = fs::read_to_string(&absolute).expect("fixture");
    let result = SynRustParser.parse_file(&relative, &source);
    assert!(matches!(result, Err(SpectoruError::RustParse { .. })));
}

#[test]
fn 同じ入力からは常に同じ結果が得られる() {
    assert_eq!(parse("nested.rs").group, parse("nested.rs").group);
}
