//! 単一 HTML を組み立てる [`TemplateEngine`](crate::ports::template_engine::TemplateEngine) 実装。
//!
//! 外部依存ゼロ。CDN も含め外部ホストへのリクエストを一切行わない。SRI なしの
//! 外部スクリプト読み込みは、配信元が侵害された時点で閲覧者のブラウザ上での
//! 任意コード実行を許す経路になるため。
//!
//! 検索用のインデックスを別途 JSON として埋め込むことはしない。そうすると
//! テスト名が HTML とデータの二重に載り、エスケープ経路も 2 つに増える。
//! 代わりに仕様ツリーを完全な HTML として描き、絞り込みは JS が DOM の
//! テキストを見て行う。データの出どころが 1 つになり、JS を無効にしても
//! 全仕様が読める。
//!
//! **テスト名は任意の文字列であり `<script>` を含みうる。** 利用者由来の値は
//! 例外なく [`escape_html`] を通す。この性質は契約テストで固定している。

use std::fmt::Write as _;

use crate::core::ir::{
    Group, IntermediateRepresentation, ProjectMeta, Source, Spec, SpecStatus, Stats,
};
use crate::core::tree::normalize_path;
use crate::error::SpectoruError;
use crate::ports::template_engine::TemplateEngine;

/// リンクとして出力してよい URL スキーム。
///
/// `javascript:` などを `href` にそのまま流すとクリックで任意コードが走る。
/// 設定ファイル由来の値であっても信頼できない入力として扱う。
const SAFE_URL_PREFIXES: &[&str] = &["https://", "http://"];

#[derive(Debug, Default, Clone, Copy)]
pub struct InlineHtmlTemplate;

impl TemplateEngine for InlineHtmlTemplate {
    fn render_site(
        &self,
        projects: &[IntermediateRepresentation],
    ) -> Result<String, SpectoruError> {
        Ok(render(projects))
    }
}

fn render(projects: &[IntermediateRepresentation]) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"ja\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    let _ = writeln!(html, "<title>{}</title>", escape_html(site_title(projects)));
    let _ = writeln!(html, "<style>{STYLE}</style>");
    html.push_str("</head>\n<body>\n");

    render_header(&mut html, projects);
    html.push_str("<div class=\"layout\">\n");
    render_sidebar(&mut html, projects);
    html.push_str("<main class=\"content\">\n");
    render_projects(&mut html, projects);
    render_diagnostics(&mut html, projects);
    html.push_str("<p class=\"empty-result\" hidden>一致する仕様がありません。</p>\n");
    html.push_str("</main>\n</div>\n");

    let _ = writeln!(html, "<script>{SCRIPT}</script>");
    html.push_str("</body>\n</html>\n");

    html
}

fn site_title(projects: &[IntermediateRepresentation]) -> &str {
    match projects {
        [only] => &only.project.name,
        _ => "Spectoru",
    }
}

fn render_header(html: &mut String, projects: &[IntermediateRepresentation]) {
    let totals = total_stats(projects);
    html.push_str("<header class=\"site-header\">\n");

    let _ = writeln!(html, "<h1>{}</h1>", escape_html(site_title(projects)));

    html.push_str("<dl class=\"stats\">\n");
    push_stat(html, "仕様", &totals.total_specs.to_string());
    push_stat(html, "Rust", &totals.languages.rust.to_string());
    push_stat(html, "TypeScript", &totals.languages.typescript.to_string());
    push_stat(html, "警告", &totals.warnings.to_string());
    html.push_str("</dl>\n");

    html.push_str(
        "<input type=\"search\" id=\"search\" class=\"search\" \
         placeholder=\"仕様を検索\" autocomplete=\"off\">\n",
    );
    html.push_str("</header>\n");
}

fn push_stat(html: &mut String, label: &str, value: &str) {
    let _ = writeln!(
        html,
        "<div><dt>{}</dt><dd>{}</dd></div>",
        escape_html(label),
        escape_html(value)
    );
}

/// 全プロジェクトの集計を合算する。
///
/// 各 IR が持つ `stats` は抽出時に確定した事実なので、ここでは足すだけにする。
fn total_stats(projects: &[IntermediateRepresentation]) -> Stats {
    let mut totals = Stats::default();
    for project in projects {
        totals.total_specs += project.stats.total_specs;
        totals.warnings += project.stats.warnings;
        totals.languages.rust += project.stats.languages.rust;
        totals.languages.typescript += project.stats.languages.typescript;
    }
    totals
}

fn render_sidebar(html: &mut String, projects: &[IntermediateRepresentation]) {
    html.push_str("<nav class=\"sidebar\" aria-label=\"仕様ツリー\">\n<ul>\n");

    for (project_index, project) in projects.iter().enumerate() {
        let _ = writeln!(
            html,
            "<li data-nav=\"{}\"><a href=\"#{}\">{}</a>",
            project_id(project_index),
            project_id(project_index),
            escape_html(&project.project.name)
        );
        html.push_str("<ul>\n");

        for (source_index, source) in project.sources.iter().enumerate() {
            let id = source_id(project_index, source_index);
            let _ = writeln!(
                html,
                "<li data-nav=\"{id}\"><a href=\"#{id}\">{}</a>",
                escape_html(&source.name)
            );
            html.push_str("<ul>\n");

            for (group_index, group) in source.groups.iter().enumerate() {
                let id = group_id(project_index, source_index, group_index);
                let _ = writeln!(
                    html,
                    "<li data-nav=\"{id}\"><a href=\"#{id}\">{}</a></li>",
                    escape_html(&group.name)
                );
            }

            html.push_str("</ul>\n</li>\n");
        }

        html.push_str("</ul>\n</li>\n");
    }

    html.push_str("</ul>\n</nav>\n");
}

fn render_projects(html: &mut String, projects: &[IntermediateRepresentation]) {
    for (project_index, project) in projects.iter().enumerate() {
        let _ = writeln!(
            html,
            "<section class=\"project\" id=\"{}\" data-container>",
            project_id(project_index)
        );
        let _ = writeln!(html, "<h2>{}</h2>", escape_html(&project.project.name));
        render_project_meta(html, &project.project);

        for (source_index, source) in project.sources.iter().enumerate() {
            render_source(html, project_index, source_index, source);
        }

        html.push_str("</section>\n");
    }
}

fn render_project_meta(html: &mut String, meta: &ProjectMeta) {
    html.push_str("<p class=\"meta\">\n");

    if let Some(repository) = &meta.repository {
        if is_safe_url(repository) {
            let _ = writeln!(
                html,
                "<a class=\"repository\" href=\"{0}\" rel=\"noreferrer noopener\">{0}</a>",
                escape_html(repository)
            );
        } else {
            // スキームが信用できない場合はリンクにせず文字列として見せる。
            let _ = writeln!(
                html,
                "<span class=\"repository\">{}</span>",
                escape_html(repository)
            );
        }
    }
    if let Some(revision) = &meta.revision {
        let _ = writeln!(
            html,
            "<span class=\"revision\">{}</span>",
            escape_html(revision)
        );
    }
    let _ = writeln!(
        html,
        "<span class=\"extracted-at\">{}</span>",
        escape_html(&meta.extracted_at)
    );

    html.push_str("</p>\n");
}

fn render_source(html: &mut String, project_index: usize, source_index: usize, source: &Source) {
    let _ = writeln!(
        html,
        "<section class=\"source\" id=\"{}\" data-container>",
        source_id(project_index, source_index)
    );
    let _ = writeln!(html, "<h3>{}</h3>", escape_html(&source.name));

    for (group_index, group) in source.groups.iter().enumerate() {
        render_group(
            html,
            group,
            Some(&group_id(project_index, source_index, group_index)),
        );
    }

    html.push_str("</section>\n");
}

fn render_group(html: &mut String, group: &Group, id: Option<&str>) {
    match id {
        Some(id) => {
            let _ = writeln!(html, "<section class=\"group\" id=\"{id}\" data-container>");
        }
        None => html.push_str("<section class=\"group\" data-container>\n"),
    }

    let _ = writeln!(html, "<h4>{}</h4>", escape_html(&group.name));

    if !group.specs.is_empty() {
        html.push_str("<ul class=\"specs\">\n");
        for spec in &group.specs {
            render_spec(html, spec);
        }
        html.push_str("</ul>\n");
    }

    for child in &group.children {
        render_group(html, child, None);
    }

    html.push_str("</section>\n");
}

fn render_spec(html: &mut String, spec: &Spec) {
    let skipped = if spec.status == SpecStatus::Skipped {
        " spec--skipped"
    } else {
        ""
    };

    let _ = writeln!(html, "<li class=\"spec{skipped}\" data-spec>");
    let _ = writeln!(
        html,
        "<span class=\"spec-name\">{}</span>",
        escape_html(&spec.name)
    );
    let _ = writeln!(
        html,
        "<span class=\"spec-meta\">{} · {}:{}</span>",
        escape_html(spec.language.as_str()),
        escape_html(&normalize_path(&spec.file)),
        spec.line
    );
    if spec.status == SpecStatus::Skipped {
        html.push_str("<span class=\"badge\">skipped</span>\n");
    }
    html.push_str("</li>\n");
}

fn render_diagnostics(html: &mut String, projects: &[IntermediateRepresentation]) {
    if projects
        .iter()
        .all(|project| project.diagnostics.is_empty())
    {
        return;
    }

    html.push_str("<section class=\"diagnostics\">\n<h2>診断</h2>\n<ul>\n");
    for diagnostic in projects.iter().flat_map(|project| &project.diagnostics) {
        let _ = writeln!(
            html,
            "<li class=\"diagnostic diagnostic--{}\">",
            diagnostic.level.as_str()
        );
        let _ = writeln!(
            html,
            "<span class=\"code\">{}</span>",
            escape_html(diagnostic.code.as_str())
        );
        let _ = writeln!(
            html,
            "<span class=\"message\">{}</span>",
            escape_html(&diagnostic.message)
        );
        if let Some(file) = &diagnostic.file {
            let location = match diagnostic.line {
                Some(line) => format!("{}:{line}", normalize_path(file)),
                None => normalize_path(file),
            };
            let _ = writeln!(
                html,
                "<span class=\"location\">{}</span>",
                escape_html(&location)
            );
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</section>\n");
}

fn project_id(project_index: usize) -> String {
    format!("p{project_index}")
}

fn source_id(project_index: usize, source_index: usize) -> String {
    format!("p{project_index}-s{source_index}")
}

fn group_id(project_index: usize, source_index: usize, group_index: usize) -> String {
    format!("p{project_index}-s{source_index}-g{group_index}")
}

fn is_safe_url(url: &str) -> bool {
    SAFE_URL_PREFIXES
        .iter()
        .any(|prefix| url.starts_with(prefix))
}

/// HTML のテキスト / 属性値コンテキストで安全な形にエスケープする。
///
/// テスト名は利用者のリポジトリ由来の任意文字列であり、`<script>` や引用符を
/// 含みうる。属性値は必ず `"` で囲むため、両コンテキストをこの 1 つの関数で
/// 賄える。
fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

const STYLE: &str = "
:root { color-scheme: light dark; --bg: #fff; --fg: #1a1a1a; --muted: #666;
  --line: #e2e2e2; --accent: #3b6ea5; --skip: #8a6d3b; --warn: #8a6d3b; --err: #a33; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #16181c; --fg: #e6e6e6; --muted: #9aa0a6; --line: #2c3038;
    --accent: #7aa7d8; --skip: #d0b070; --warn: #d0b070; --err: #e08585; }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg); line-height: 1.7;
  font-family: system-ui, -apple-system, 'Hiragino Sans', 'Noto Sans JP', sans-serif; }
[hidden] { display: none !important; }
.site-header { padding: 1.5rem 2rem; border-bottom: 1px solid var(--line); }
.site-header h1 { margin: 0 0 .75rem; font-size: 1.35rem; }
.stats { display: flex; flex-wrap: wrap; gap: 1.25rem; margin: 0 0 1rem; }
.stats div { display: flex; align-items: baseline; gap: .4rem; }
.stats dt { color: var(--muted); font-size: .8rem; margin: 0; }
.stats dd { margin: 0; font-variant-numeric: tabular-nums; font-weight: 600; }
.search { width: 100%; max-width: 32rem; padding: .5rem .75rem; font-size: 1rem;
  color: inherit; background: transparent; border: 1px solid var(--line); border-radius: .4rem; }
.layout { display: flex; align-items: flex-start; }
.sidebar { flex: 0 0 20rem; max-height: calc(100vh - 9rem); overflow-y: auto;
  position: sticky; top: 0; padding: 1.5rem 1rem; border-right: 1px solid var(--line); }
.sidebar ul { list-style: none; margin: 0; padding-left: .9rem; }
.sidebar > ul { padding-left: 0; }
.sidebar a { color: inherit; text-decoration: none; font-size: .9rem; }
.sidebar a:hover { color: var(--accent); text-decoration: underline; }
.content { flex: 1 1 auto; min-width: 0; padding: 1.5rem 2rem 6rem; }
.project > h2 { margin-top: 0; }
.meta { display: flex; flex-wrap: wrap; gap: 1rem; margin: 0 0 1.5rem;
  color: var(--muted); font-size: .8rem; }
.meta a { color: var(--accent); }
.source { margin-bottom: 2.5rem; }
.source > h3 { font-size: 1.05rem; padding-bottom: .3rem; border-bottom: 1px solid var(--line); }
.group { margin: 1.25rem 0 1.25rem .25rem; padding-left: .9rem; border-left: 2px solid var(--line); }
.group h4 { margin: 0 0 .4rem; font-size: .92rem; color: var(--muted); font-weight: 600; }
.specs { list-style: none; margin: 0; padding: 0; }
.spec { display: flex; flex-wrap: wrap; align-items: baseline; gap: .6rem; padding: .2rem 0; }
.spec-name { font-size: .98rem; }
.spec-meta { color: var(--muted); font-size: .75rem; font-variant-numeric: tabular-nums; }
.spec--skipped .spec-name { color: var(--skip); text-decoration: line-through; }
.badge { font-size: .68rem; padding: 0 .4rem; border-radius: .6rem;
  border: 1px solid var(--skip); color: var(--skip); }
.diagnostics { margin-top: 3rem; padding-top: 1rem; border-top: 1px solid var(--line); }
.diagnostics ul { list-style: none; margin: 0; padding: 0; }
.diagnostic { display: flex; flex-wrap: wrap; gap: .6rem; padding: .3rem 0; font-size: .85rem; }
.diagnostic .code { font-family: ui-monospace, monospace; font-size: .78rem; }
.diagnostic--warning .code { color: var(--warn); }
.diagnostic--error .code { color: var(--err); }
.diagnostic .location { color: var(--muted); font-size: .75rem; }
.empty-result { color: var(--muted); }
@media (max-width: 50rem) {
  .layout { display: block; }
  .sidebar { position: static; max-height: none; border-right: 0;
    border-bottom: 1px solid var(--line); flex-basis: auto; }
  .content { padding: 1.5rem 1rem 4rem; }
}
";

const SCRIPT: &str = "
(function () {
  var input = document.getElementById('search');
  if (!input) { return; }

  // 全角英数を半角に畳んでから小文字化する。索引はここで一度だけ作るので、
  // 正規化の規則がクエリ側と索引側で食い違うことがない。
  function normalize(text) {
    return text.replace(/[\\uFF01-\\uFF5E]/g, function (character) {
      return String.fromCharCode(character.charCodeAt(0) - 0xFEE0);
    }).toLowerCase();
  }

  var specs = [];
  document.querySelectorAll('[data-spec]').forEach(function (element) {
    specs.push({ element: element, text: normalize(element.textContent) });
  });
  var containers = document.querySelectorAll('[data-container]');
  var navItems = document.querySelectorAll('[data-nav]');
  var emptyResult = document.querySelector('.empty-result');

  function apply() {
    var query = normalize(input.value.trim());
    var visible = 0;

    specs.forEach(function (spec) {
      var matched = query === '' || spec.text.indexOf(query) !== -1;
      spec.element.hidden = !matched;
      if (matched) { visible += 1; }
    });

    containers.forEach(function (container) {
      container.hidden = !container.querySelector('[data-spec]:not([hidden])');
    });

    navItems.forEach(function (item) {
      var target = document.getElementById(item.getAttribute('data-nav'));
      item.hidden = !target || target.hidden;
    });

    if (emptyResult) {
      emptyResult.hidden = !(visible === 0 && specs.length > 0);
    }
  }

  input.addEventListener('input', apply);
})();
";
