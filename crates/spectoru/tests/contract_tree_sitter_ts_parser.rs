//! `TreeSitterTsParser` の契約テスト。
//!
//! 「Vitest のテストソースを spectoru がどう仕様として解釈するか」を固定する。
//! 検証対象は `tests/fixtures/vitest/` 配下のソースであり、フィクスチャ自体が
//! 読める仕様書になるよう書かれている。
//!
//! ここに書かれた性質は tree-sitter を別のパーサに差し替えても守られなければならない。

#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};

use spectoru::adapters::tree_sitter_ts_parser::TreeSitterTsParser;
use spectoru::core::ir::{Diagnostic, DiagnosticCode, Group, Language, Spec, SpecStatus};
use spectoru::ports::rust_parser::ParsedFile;
use spectoru::ports::ts_parser::TsParser;

fn parse(name: &str) -> ParsedFile {
    let relative = PathBuf::from("tests/fixtures/vitest").join(name);
    let absolute = Path::new(env!("CARGO_MANIFEST_DIR")).join(&relative);
    let source = fs::read_to_string(&absolute)
        .unwrap_or_else(|e| panic!("fixture {}: {e}", absolute.display()));
    TreeSitterTsParser
        .parse_file(&relative, &source)
        .unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

fn spec_summary(specs: &[Spec]) -> Vec<(&str, u32, SpecStatus)> {
    specs
        .iter()
        .map(|spec| (spec.name.as_str(), spec.line, spec.status))
        .collect()
}

fn spec_names(specs: &[Spec]) -> Vec<&str> {
    specs.iter().map(|spec| spec.name.as_str()).collect()
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

fn codes(diagnostics: &[Diagnostic]) -> Vec<DiagnosticCode> {
    diagnostics.iter().map(|d| d.code).collect()
}

#[test]
fn ファイルパスが最上位グループになる() {
    let parsed = parse("flat.ts");
    assert_eq!(parsed.group.name, "tests/fixtures/vitest/flat.ts");
    assert_eq!(parsed.group.line, None);
}

#[test]
fn itとtestは同等に扱われる() {
    let parsed = parse("flat.ts");
    assert_eq!(
        spec_summary(&parsed.group.specs),
        [
            (
                "招待リンクからクリエイター登録が完了する",
                3,
                SpecStatus::Active
            ),
            ("無効な招待コードでは登録できない", 5, SpecStatus::Active),
        ]
    );
}

#[test]
fn テスト以外の呼び出しは無視される() {
    let parsed = parse("flat.ts");
    assert_eq!(parsed.group.specs.len(), 2);
    assert_eq!(parsed.group.children, []);
}

#[test]
fn 言語はtypescriptとして記録される() {
    let parsed = parse("flat.ts");
    assert!(
        parsed
            .group
            .specs
            .iter()
            .all(|spec| spec.language == Language::TypeScript)
    );
}

#[test]
fn describeがサブグループになる() {
    let parsed = parse("nested.ts");
    let group = child(&parsed.group, "有効な画像がアップロードされたとき");
    assert_eq!(group.line, Some(3));
    assert_eq!(
        spec_summary(&group.specs),
        [
            ("作品が公開状態で作成される", 4, SpecStatus::Active),
            (
                "コラボレーターにクレジットが付与される",
                5,
                SpecStatus::Active
            ),
        ]
    );
}

#[test]
fn describeは任意の深さでネストできる() {
    let parsed = parse("nested.ts");
    let outer = child(&parsed.group, "タイトルが未入力のとき");
    let inner = child(outer, "さらに説明も未入力のとき");
    assert_eq!(inner.line, Some(9));
    assert_eq!(
        spec_summary(&inner.specs),
        [("バリデーションエラーが返される", 10, SpecStatus::Active)]
    );
}

#[test]
fn テストを含まないdescribeはグループにならない() {
    let parsed = parse("nested.ts");
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
fn skipとtodoはskipped状態になる() {
    let parsed = parse("modifiers.ts");
    assert_eq!(
        spec_summary(&parsed.group.specs),
        [
            ("一時的に無効化されたテスト", 3, SpecStatus::Skipped),
            ("これから書くテスト", 5, SpecStatus::Skipped),
            ("これだけ実行するテスト", 7, SpecStatus::Active),
            ("並行実行されるテスト", 9, SpecStatus::Active),
            ("修飾子が連なるテスト", 11, SpecStatus::Skipped),
        ]
    );
}

#[test]
fn onlyは実行の絞り込みなので状態を変えない() {
    // `.only` はどのテストを走らせるかの指定であって、仕様が無効という意味ではない。
    let parsed = parse("modifiers.ts");
    let only = &parsed.group.specs[2];
    assert_eq!(only.name, "これだけ実行するテスト");
    assert_eq!(only.status, SpecStatus::Active);
}

#[test]
fn describe_skipの中のテストもskipped状態になる() {
    let parsed = parse("modifiers.ts");
    let group = child(&parsed.group, "まるごと無効化されたグループ");
    assert_eq!(
        spec_summary(&group.specs),
        [("中のテストもスキップ扱いになる", 14, SpecStatus::Skipped)]
    );
}

#[test]
fn 補間を含むテンプレートリテラルは除外され診断が出る() {
    let parsed = parse("dynamic.ts");
    assert!(
        !spec_names(&parsed.group.specs)
            .iter()
            .any(|name| name.contains("のとき成功する"))
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::DynamicTestName && d.line == Some(6))
    );
}

#[test]
fn eachによるパラメタライズドテストは除外され診断が出る() {
    let parsed = parse("dynamic.ts");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::DynamicTestName && d.line == Some(8))
    );
}

#[test]
fn 変数をテスト名に渡した場合も除外され診断が出る() {
    let parsed = parse("dynamic.ts");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::DynamicTestName && d.line == Some(10))
    );
}

#[test]
fn 補間の無いテンプレートリテラルは通常の文字列として扱う() {
    let parsed = parse("dynamic.ts");
    assert_eq!(
        spec_summary(&parsed.group.specs),
        [(
            "補間の無いテンプレートリテラルは静的に決まる",
            12,
            SpecStatus::Active
        )]
    );
    assert_eq!(
        codes(&parsed.diagnostics),
        [
            DiagnosticCode::DynamicTestName,
            DiagnosticCode::DynamicTestName,
            DiagnosticCode::DynamicTestName,
        ]
    );
}

#[test]
fn オプション引数が挟まってもコールバックを見つける() {
    let parsed = parse("callbacks.ts");
    let group = child(&parsed.group, "オプション引数を挟むグループ");
    assert_eq!(
        spec_summary(&group.specs),
        [("オプション引数を挟むテスト", 4, SpecStatus::Active)]
    );
}

#[test]
fn 関数式で書かれたコールバックも辿れる() {
    let parsed = parse("callbacks.ts");
    let group = child(&parsed.group, "関数式で書かれたグループ");
    assert_eq!(
        spec_summary(&group.specs),
        [("関数式で書かれたテスト", 8, SpecStatus::Active)]
    );
}

#[test]
fn コールバックを持たないdescribeは空なので取り除かれる() {
    let parsed = parse("callbacks.ts");
    assert!(
        !parsed
            .group
            .children
            .iter()
            .any(|c| c.name == "コールバックを持たないグループ")
    );
}

#[test]
fn エスケープシーケンスは実際の文字に戻される() {
    let parsed = parse("escapes.ts");
    assert_eq!(
        spec_names(&parsed.group.specs),
        [
            "ダブルクォート \" を含む",
            "バックスラッシュ \\ を含む",
            "タブ\tを含む",
            "コードポイント \u{1F600} を含む",
            "シングルクォートで囲まれた名前",
        ]
    );
}

#[test]
fn テストを含まないファイルは空のグループになる() {
    let parsed = parse("no_tests.ts");
    assert_eq!(parsed.group.specs, []);
    assert_eq!(parsed.group.children, []);
    assert_eq!(parsed.diagnostics, []);
}

#[test]
fn 構文エラーがあっても読めた範囲のテストは抽出される() {
    // tree-sitter はエラー耐性があるため、壊れた箇所以外は仕様として活かす。
    let parsed = parse("broken.ts");
    assert_eq!(
        spec_summary(&parsed.group.specs),
        [("壊れていない範囲のテスト", 4, SpecStatus::Active)]
    );
    assert!(codes(&parsed.diagnostics).contains(&DiagnosticCode::ParseError));
}

#[test]
fn tsxファイルはJSXを含んでいてもパースできる() {
    let parsed = parse("component.test.tsx");
    let group = child(&parsed.group, "ボタンコンポーネント");
    assert_eq!(
        spec_summary(&group.specs),
        [("ラベルが表示される", 5, SpecStatus::Active)]
    );
    assert_eq!(parsed.diagnostics, []);
}

#[test]
fn 正常にパースできたファイルは診断を出さない() {
    for name in ["flat.ts", "nested.ts", "modifiers.ts", "callbacks.ts"] {
        let parsed = parse(name);
        assert_eq!(parsed.diagnostics, [], "{name} が診断を出している");
    }
}

#[test]
fn 同じ入力からは常に同じ結果が得られる() {
    assert_eq!(parse("nested.ts").group, parse("nested.ts").group);
}
