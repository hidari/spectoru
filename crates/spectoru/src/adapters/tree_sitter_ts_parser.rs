//! `tree-sitter-typescript` による [`TsParser`](crate::ports::ts_parser::TsParser) 実装。
//!
//! `describe` / `it` / `test` の呼び出しを構文木から拾う。tree-sitter の型は
//! このファイルの外に出ない。
//!
//! Rust 側と違い構文エラーでも `Err` を返さないのは、tree-sitter がエラー耐性を
//! 持ち壊れた箇所以外は正しい構文木を返すため。読めた分は仕様として活かし、
//! 壊れていた事実は診断として記録する方が利用者にとって有益になる。

use std::path::Path;

use tree_sitter::{Node, Parser};

use crate::core::ir::{
    Diagnostic, DiagnosticCode, DiagnosticLevel, Group, Language, Spec, SpecStatus,
};
use crate::core::tree::file_group;
use crate::error::SpectoruError;
use crate::ports::rust_parser::ParsedFile;
use crate::ports::ts_parser::TsParser;

/// グループを作る呼び出し。
const GROUP_FUNCTIONS: &[&str] = &["describe"];
/// spec を作る呼び出し。Vitest では `it` と `test` は同等。
const SPEC_FUNCTIONS: &[&str] = &["it", "test"];
/// 実行対象から外す修飾子。`only` は実行の絞り込みであって仕様の状態ではないため含めない。
const SKIP_MODIFIERS: &[&str] = &["skip", "todo"];
/// 名前が実行時にしか決まらない修飾子。v0.1 ではスコープ外。
const DYNAMIC_MODIFIERS: &[&str] = &["each", "for"];

#[derive(Debug, Default, Clone, Copy)]
pub struct TreeSitterTsParser;

impl TsParser for TreeSitterTsParser {
    fn parse_file(&self, path: &Path, source: &str) -> Result<ParsedFile, SpectoruError> {
        let mut parser = Parser::new();
        parser.set_language(&language_for(path)).map_err(|error| {
            SpectoruError::TypeScriptParse {
                path: path.to_path_buf(),
                message: error.to_string(),
            }
        })?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| SpectoruError::TypeScriptParse {
                path: path.to_path_buf(),
                message: "構文木を生成できなかった".to_string(),
            })?;

        let mut diagnostics = Vec::new();
        let root = tree.root_node();

        if root.has_error() {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: DiagnosticCode::ParseError,
                message: "構文エラーを含むため一部のテストを抽出できていない可能性がある"
                    .to_string(),
                file: Some(path.to_path_buf()),
                line: Some(line_of(root)),
            });
        }

        let collected = collect(root, &Context::new(path, source), &mut diagnostics);

        Ok(ParsedFile {
            group: file_group(path, collected.groups, collected.specs),
            diagnostics,
        })
    }
}

fn language_for(path: &Path) -> tree_sitter::Language {
    if path.extension().is_some_and(|ext| ext == "tsx") {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }
}

/// 走査中に持ち回る不変の文脈。
struct Context<'a> {
    path: &'a Path,
    source: &'a str,
    /// 祖先の `describe.skip` を引き継ぐ。スキップされた describe の中の
    /// テストは実行されないため、spec の状態にも反映する。
    inherited_status: SpecStatus,
}

impl<'a> Context<'a> {
    fn new(path: &'a Path, source: &'a str) -> Self {
        Self {
            path,
            source,
            inherited_status: SpecStatus::Active,
        }
    }

    fn with_status(&self, status: SpecStatus) -> Self {
        Self {
            path: self.path,
            source: self.source,
            inherited_status: status,
        }
    }
}

#[derive(Default)]
struct Collected {
    groups: Vec<Group>,
    specs: Vec<Spec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallKind {
    Group,
    Spec,
}

/// `node` の子孫から `describe` / `it` / `test` 呼び出しを集める。
///
/// `describe` に出会ったらそのコールバック本体だけを子として辿り、それ以外の
/// ノードは素通しして下位を探す。これにより `if` や即時実行関数で囲まれていても
/// 見つけられる一方、ネスト関係は呼び出しの入れ子どおりに保たれる。
fn collect(node: Node, context: &Context, diagnostics: &mut Vec<Diagnostic>) -> Collected {
    let mut collected = Collected::default();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let Some((kind, modifiers)) = classify(child, context.source) else {
            let inner = collect(child, context, diagnostics);
            collected.groups.extend(inner.groups);
            collected.specs.extend(inner.specs);
            continue;
        };

        let line = line_of(child);

        if modifiers
            .iter()
            .any(|modifier| DYNAMIC_MODIFIERS.contains(&modifier.as_str()))
        {
            diagnostics.push(dynamic_name_diagnostic(
                context.path,
                line,
                "パラメタライズドテストの名前は静的に決まらない",
            ));
            continue;
        }

        let status = if modifiers
            .iter()
            .any(|modifier| SKIP_MODIFIERS.contains(&modifier.as_str()))
        {
            SpecStatus::Skipped
        } else {
            context.inherited_status
        };

        let arguments = child.child_by_field_name("arguments");
        let Some(name) = first_argument_name(arguments, context, line, diagnostics) else {
            continue;
        };

        match kind {
            CallKind::Spec => collected.specs.push(Spec {
                name,
                file: context.path.to_path_buf(),
                line,
                language: Language::TypeScript,
                status,
            }),
            CallKind::Group => {
                let inner = arguments
                    .and_then(callback_body)
                    .map(|body| collect(body, &context.with_status(status), diagnostics))
                    .unwrap_or_default();
                collected.groups.push(Group {
                    name,
                    file: context.path.to_path_buf(),
                    line: Some(line),
                    children: inner.groups,
                    specs: inner.specs,
                });
            }
        }
    }

    collected
}

/// 呼び出しノードを spectoru が関心を持つ種類に分類する。
///
/// `it.skip(...)` や `describe.concurrent.skip(...)` のようなメソッドチェーンは
/// 基底の識別子と修飾子の列に分解する。
fn classify(node: Node, source: &str) -> Option<(CallKind, Vec<String>)> {
    if node.kind() != "call_expression" {
        return None;
    }

    let (base, modifiers) = flatten_callee(node.child_by_field_name("function")?, source)?;

    if GROUP_FUNCTIONS.contains(&base.as_str()) {
        return Some((CallKind::Group, modifiers));
    }
    if SPEC_FUNCTIONS.contains(&base.as_str()) {
        return Some((CallKind::Spec, modifiers));
    }
    None
}

/// 呼び出し対象を「基底の識別子 + 修飾子の列」に分解する。
fn flatten_callee(node: Node, source: &str) -> Option<(String, Vec<String>)> {
    match node.kind() {
        "identifier" => Some((text_of(node, source)?, Vec::new())),
        "member_expression" => {
            let (base, mut modifiers) =
                flatten_callee(node.child_by_field_name("object")?, source)?;
            modifiers.push(text_of(node.child_by_field_name("property")?, source)?);
            Some((base, modifiers))
        }
        _ => None,
    }
}

/// 第 1 引数からテスト名を取り出す。静的に決まらない場合は診断を積んで `None`。
fn first_argument_name(
    arguments: Option<Node>,
    context: &Context,
    line: u32,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    let argument = arguments.and_then(|node| node.named_child(0));

    let Some(argument) = argument else {
        diagnostics.push(dynamic_name_diagnostic(
            context.path,
            line,
            "テスト名の引数が無い",
        ));
        return None;
    };

    match argument.kind() {
        "string" => Some(decode_string(argument, context.source)),
        "template_string" if !has_substitution(argument) => {
            Some(decode_string(argument, context.source))
        }
        "template_string" => {
            diagnostics.push(dynamic_name_diagnostic(
                context.path,
                line,
                "テンプレートリテラルに補間が含まれる",
            ));
            None
        }
        _ => {
            diagnostics.push(dynamic_name_diagnostic(
                context.path,
                line,
                "テスト名が文字列リテラルではない",
            ));
            None
        }
    }
}

/// 引数列からコールバック関数の本体を探す。
///
/// `it(name, fn)` だけでなく `it(name, { timeout }, fn)` のように
/// オプション引数が挟まる形にも対応するため、位置ではなく種類で探す。
fn callback_body(arguments: Node) -> Option<Node> {
    let mut cursor = arguments.walk();
    arguments
        .named_children(&mut cursor)
        .find(|node| matches!(node.kind(), "arrow_function" | "function_expression"))
        .and_then(|function| function.child_by_field_name("body"))
}

fn has_substitution(node: Node) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| child.kind() == "template_substitution")
}

/// 文字列 / テンプレートリテラルのノードから中身の文字列を組み立てる。
fn decode_string(node: Node, source: &str) -> String {
    let mut cursor = node.walk();
    let mut out = String::new();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "string_fragment" => out.push_str(text_of(child, source).unwrap_or_default().as_str()),
            "escape_sequence" => {
                out.push_str(&unescape(
                    text_of(child, source).unwrap_or_default().as_str(),
                ));
            }
            _ => {}
        }
    }
    out
}

/// JavaScript のエスケープシーケンス 1 つを実際の文字に変換する。
///
/// 解釈できない並びは、元の見た目を壊さないようそのまま返す。テスト名は
/// 仕様文として表示されるため、黙って文字を落とす方が害が大きい。
fn unescape(sequence: &str) -> String {
    let mut chars = sequence.chars();
    if chars.next() != Some('\\') {
        return sequence.to_string();
    }
    let Some(marker) = chars.next() else {
        return sequence.to_string();
    };
    let rest: String = chars.collect();

    match marker {
        'n' => "\n".to_string(),
        'r' => "\r".to_string(),
        't' => "\t".to_string(),
        'b' => "\u{8}".to_string(),
        'f' => "\u{c}".to_string(),
        'v' => "\u{b}".to_string(),
        '0' if rest.is_empty() => "\0".to_string(),
        'u' | 'x' => decode_code_point(&rest).map_or_else(|| sequence.to_string(), String::from),
        other => other.to_string(),
    }
}

fn decode_code_point(digits: &str) -> Option<char> {
    let hex = digits.strip_prefix('{').and_then(|d| d.strip_suffix('}'));
    char::from_u32(u32::from_str_radix(hex.unwrap_or(digits), 16).ok()?)
}

fn text_of(node: Node, source: &str) -> Option<String> {
    node.utf8_text(source.as_bytes()).ok().map(str::to_string)
}

fn line_of(node: Node) -> u32 {
    u32::try_from(node.start_position().row).unwrap_or(0) + 1
}

fn dynamic_name_diagnostic(path: &Path, line: u32, reason: &str) -> Diagnostic {
    Diagnostic {
        level: DiagnosticLevel::Warning,
        code: DiagnosticCode::DynamicTestName,
        message: format!("テスト名を静的に決定できないため除外した: {reason}"),
        file: Some(path.to_path_buf()),
        line: Some(line),
    }
}
