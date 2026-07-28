//! `InlineHtmlTemplate` の契約テスト。
//!
//! 最重要の契約は 2 つ。
//!
//! 1. **エスケープ**: テスト名は利用者のリポジトリ由来の任意文字列であり、
//!    外部コントリビュータが書いたものがそのまま埋め込まれる経路がある。
//!    信頼できない入力として扱わなければ、仕様サイトがそのまま XSS になる。
//! 2. **外部依存ゼロ**: 生成物がどこかのホストへリソースを取りに行った時点で、
//!    そのホストの侵害が閲覧者への攻撃経路になる。
//!
//! テンプレート実装を差し替えてもこの 2 つは守られなければならない。

#![allow(non_snake_case)]

use std::path::PathBuf;

use spectoru::adapters::inline_html_template::InlineHtmlTemplate;
use spectoru::core::ir::{
    Diagnostic, DiagnosticCode, DiagnosticLevel, Group, IntermediateRepresentation, Language,
    Languages, ProjectMeta, Source, Spec, SpecStatus, Stats,
};
use spectoru::ports::template_engine::TemplateEngine;

fn spec(name: &str) -> Spec {
    Spec {
        name: name.to_string(),
        file: PathBuf::from("tests/artwork.rs"),
        line: 5,
        language: Language::Rust,
        status: SpecStatus::Active,
    }
}

fn group(name: &str, specs: Vec<Spec>, children: Vec<Group>) -> Group {
    Group {
        name: name.to_string(),
        file: PathBuf::from("tests/artwork.rs"),
        line: None,
        children,
        specs,
    }
}

fn project(name: &str, groups: Vec<Group>) -> IntermediateRepresentation {
    let total = groups.iter().map(|g| g.specs.len()).sum();
    IntermediateRepresentation {
        project: ProjectMeta {
            name: name.to_string(),
            repository: None,
            revision: Some("abc1234".to_string()),
            extracted_at: "2026-04-13T12:00:00Z".to_string(),
        },
        sources: vec![Source {
            name: "Backend".to_string(),
            groups,
        }],
        diagnostics: vec![],
        stats: Stats {
            total_specs: total,
            warnings: 0,
            languages: Languages {
                rust: total,
                typescript: 0,
            },
        },
    }
}

fn render(projects: &[IntermediateRepresentation]) -> String {
    InlineHtmlTemplate.render_site(projects).expect("render")
}

fn render_one(name: &str) -> String {
    render(&[project(
        "Astralys",
        vec![group("tests/artwork.rs", vec![spec(name)], vec![])],
    )])
}

// --- エスケープ ---

#[test]
fn テスト名のスクリプトタグはタグとして出力されない() {
    let html = render_one("<script>alert(1)</script>");

    assert!(
        !html.contains("<script>alert(1)</script>"),
        "スクリプトがそのまま埋め込まれている"
    );
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
}

#[test]
fn テスト名の引用符はエスケープされる() {
    let html = render_one("\"onmouseover=\"alert(1)");

    assert!(html.contains("&quot;onmouseover=&quot;alert(1)"));
    assert!(!html.contains("\"onmouseover=\""));
}

#[test]
fn テスト名のアンパサンドは二重エスケープを避けて正しく符号化される() {
    let html = render_one("a &amp; b");

    assert!(html.contains("a &amp;amp; b"));
}

#[test]
fn グループ名もエスケープされる() {
    let html = render(&[project(
        "Astralys",
        vec![group(
            "<img src=x onerror=alert(1)>",
            vec![spec("テスト")],
            vec![],
        )],
    )]);

    assert!(!html.contains("<img src=x"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
}

#[test]
fn プロジェクト名とソース名もエスケープされる() {
    let mut ir = project("<b>Astralys</b>", vec![group("g", vec![spec("t")], vec![])]);
    ir.sources[0].name = "<i>Backend</i>".to_string();

    let html = render(&[ir]);

    assert!(!html.contains("<b>Astralys</b>"));
    assert!(!html.contains("<i>Backend</i>"));
    assert!(html.contains("&lt;b&gt;Astralys&lt;/b&gt;"));
    assert!(html.contains("&lt;i&gt;Backend&lt;/i&gt;"));
}

#[test]
fn 診断メッセージもエスケープされる() {
    let mut ir = project("Astralys", vec![group("g", vec![spec("t")], vec![])]);
    ir.diagnostics.push(Diagnostic {
        level: DiagnosticLevel::Warning,
        code: DiagnosticCode::EmptyName,
        message: "<script>alert(2)</script>".to_string(),
        file: Some(PathBuf::from("<x>.rs")),
        line: Some(3),
    });

    let html = render(&[ir]);

    assert!(!html.contains("<script>alert(2)</script>"));
    assert!(html.contains("&lt;script&gt;alert(2)&lt;/script&gt;"));
    assert!(html.contains("&lt;x&gt;.rs:3"));
}

#[test]
fn 危険なスキームのrepositoryはリンクにしない() {
    // href に流すとクリックだけで任意コードが走る。
    let mut ir = project("Astralys", vec![group("g", vec![spec("t")], vec![])]);
    ir.project.repository = Some("javascript:alert(1)".to_string());

    let html = render(&[ir]);

    assert!(!html.contains("href=\"javascript:"));
    assert!(
        html.contains("javascript:alert(1)"),
        "文字列としては表示する"
    );
}

#[test]
fn httpsのrepositoryはリンクになる() {
    let mut ir = project("Astralys", vec![group("g", vec![spec("t")], vec![])]);
    ir.project.repository = Some("https://github.com/HermitianHQ/astralys".to_string());

    let html = render(&[ir]);

    assert!(html.contains("href=\"https://github.com/HermitianHQ/astralys\""));
}

// --- 外部依存ゼロ ---

#[test]
fn 外部ホストからリソースを読み込まない() {
    let mut ir = project("Astralys", vec![group("g", vec![spec("t")], vec![])]);
    ir.project.repository = Some("https://github.com/HermitianHQ/astralys".to_string());
    let html = render(&[ir]);

    // ナビゲーション用の <a href> は外部でよい。リソースの取得だけを禁じる。
    for forbidden in ["<script src", "<link ", "<img ", "<iframe ", "@import"] {
        assert!(
            !html.contains(forbidden),
            "外部リソース読み込みの疑い: {forbidden}"
        );
    }
}

#[test]
fn ネットワークを叩くJSのAPIを使わない() {
    let html = render(&[project(
        "Astralys",
        vec![group("g", vec![spec("t")], vec![])],
    )]);

    for forbidden in ["fetch(", "XMLHttpRequest", "WebSocket", "importScripts"] {
        assert!(!html.contains(forbidden), "外部通信の疑い: {forbidden}");
    }
}

#[test]
fn CSSとJSはインライン化される() {
    let html = render(&[project(
        "Astralys",
        vec![group("g", vec![spec("t")], vec![])],
    )]);

    assert!(html.contains("<style>"));
    assert!(html.contains("<script>"));
}

// --- 内容 ---

#[test]
fn 単一プロジェクトならタイトルはプロジェクト名になる() {
    let html = render(&[project(
        "Astralys",
        vec![group("g", vec![spec("t")], vec![])],
    )]);

    assert!(html.contains("<title>Astralys</title>"));
}

#[test]
fn 仕様文とグループ名が本文に現れる() {
    let html = render(&[project(
        "Astralys",
        vec![group(
            "tests/artwork.rs",
            vec![spec("作品が公開状態で作成される")],
            vec![],
        )],
    )]);

    assert!(html.contains("作品が公開状態で作成される"));
    assert!(html.contains("tests/artwork.rs"));
}

#[test]
fn ネストしたグループも出力される() {
    let inner = group("有効な画像のとき", vec![spec("作品が作成される")], vec![]);
    let html = render(&[project(
        "Astralys",
        vec![group("tests/artwork.rs", vec![], vec![inner])],
    )]);

    assert!(html.contains("有効な画像のとき"));
    assert!(html.contains("作品が作成される"));
}

#[test]
fn skippedな仕様は区別できる形で出力される() {
    let mut skipped = spec("未実装のテスト");
    skipped.status = SpecStatus::Skipped;
    let html = render(&[project("Astralys", vec![group("g", vec![skipped], vec![])])]);

    assert!(html.contains("spec--skipped"));
    assert!(html.contains("skipped"));
}

#[test]
fn 統計ヘッダーに合計と言語別内訳と警告数が出る() {
    let mut ir = project(
        "Astralys",
        vec![group("g", vec![spec("a"), spec("b")], vec![])],
    );
    ir.stats.warnings = 3;

    let html = render(&[ir]);

    assert!(html.contains("<dt>仕様</dt><dd>2</dd>"));
    assert!(html.contains("<dt>Rust</dt><dd>2</dd>"));
    assert!(html.contains("<dt>警告</dt><dd>3</dd>"));
}

#[test]
fn 複数プロジェクトの統計は合算される() {
    let html = render(&[
        project("Backend", vec![group("g", vec![spec("a")], vec![])]),
        project("Frontend", vec![group("g", vec![spec("b")], vec![])]),
    ]);

    assert!(html.contains("<dt>仕様</dt><dd>2</dd>"));
    assert!(html.contains("Backend"));
    assert!(html.contains("Frontend"));
}

#[test]
fn 複数プロジェクトならタイトルは固定名になる() {
    let html = render(&[
        project("Backend", vec![group("g", vec![spec("a")], vec![])]),
        project("Frontend", vec![group("g", vec![spec("b")], vec![])]),
    ]);

    assert!(html.contains("<title>Spectoru</title>"));
}

#[test]
fn 診断が無ければ診断セクションを出さない() {
    let html = render(&[project(
        "Astralys",
        vec![group("g", vec![spec("t")], vec![])],
    )]);

    // CSS には常に .diagnostics の規則が載るので、節そのものの有無を見る。
    assert!(!html.contains("<section class=\"diagnostics\">"));
    assert!(!html.contains("<h2>診断</h2>"));
}

#[test]
fn 診断があればコードと位置つきで出力される() {
    let mut ir = project("Astralys", vec![group("g", vec![spec("t")], vec![])]);
    ir.diagnostics.push(Diagnostic {
        level: DiagnosticLevel::Error,
        code: DiagnosticCode::ParseError,
        message: "解釈できない".to_string(),
        file: Some(PathBuf::from("src/broken.rs")),
        line: None,
    });

    let html = render(&[ir]);

    assert!(html.contains("parse_error"));
    assert!(html.contains("diagnostic--error"));
    assert!(html.contains("src/broken.rs"));
}

#[test]
fn プロジェクトが0件でも壊れたHTMLにならない() {
    let html = render(&[]);

    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.trim_end().ends_with("</html>"));
    assert!(html.contains("<dt>仕様</dt><dd>0</dd>"));
}

#[test]
fn テストを含まないプロジェクトでも壊れたHTMLにならない() {
    let html = render(&[project("Astralys", vec![])]);

    assert!(html.trim_end().ends_with("</html>"));
    assert!(html.contains("Astralys"));
}

#[test]
fn 同じIRからは常に同じHTMLが出る() {
    let ir = project("Astralys", vec![group("g", vec![spec("t")], vec![])]);
    assert_eq!(render(std::slice::from_ref(&ir)), render(&[ir]));
}
