//! `syn` による [`RustParser`](crate::ports::rust_parser::RustParser) 実装。
//!
//! `syn` の型はこのファイルの外に出ない。抽出結果は core のドメイン型だけで
//! 表現されるため、パーサライブラリを差し替えても `tests/contract_syn_rust_parser.rs`
//! を通れば application 層は影響を受けない。

use std::path::Path;

use syn::{Attribute, Item, ItemFn, ItemMod};

use crate::core::ir::{Group, Language, Spec, SpecStatus};
use crate::core::tree::file_group;
use crate::error::SpectoruError;
use crate::ports::rust_parser::{ParsedFile, RustParser};

/// テストを入れるためだけの慣習的なモジュール名。仕様文として意味を持たないため、
/// グループ階層からは取り除き、中身を親に引き上げる。
const CONTAINER_MODULE_NAMES: &[&str] = &["tests", "test"];

#[derive(Debug, Default, Clone, Copy)]
pub struct SynRustParser;

impl RustParser for SynRustParser {
    fn parse_file(&self, path: &Path, source: &str) -> Result<ParsedFile, SpectoruError> {
        // syn は構文エラーで部分的な AST を返さないため、ここは Err にするしかない。
        // 「1 ファイルの失敗で全体を止めるか」の判断は application 層が行う。
        let ast = syn::parse_file(source).map_err(|error| SpectoruError::RustParse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        let collected = collect(&ast.items, path);

        Ok(ParsedFile {
            group: file_group(path, collected.groups, collected.specs),
            diagnostics: Vec::new(),
        })
    }
}

/// 1 階層分の走査結果。
#[derive(Default)]
struct Collected {
    groups: Vec<Group>,
    specs: Vec<Spec>,
}

impl Collected {
    fn absorb(&mut self, other: Self) {
        self.groups.extend(other.groups);
        self.specs.extend(other.specs);
    }
}

fn collect(items: &[Item], path: &Path) -> Collected {
    let mut collected = Collected::default();

    for item in items {
        match item {
            Item::Fn(function) if is_test_fn(function) => {
                collected.specs.push(to_spec(function, path));
            }
            Item::Mod(module) => {
                let Some((_, inner_items)) = &module.content else {
                    // `mod foo;` は別ファイルを指す。そのファイル自体が
                    // FileWalker によって独立した最上位グループとして扱われる。
                    continue;
                };
                let inner = collect(inner_items, path);
                if is_container_module(module) {
                    collected.absorb(inner);
                } else {
                    collected.groups.push(to_group(module, path, inner));
                }
            }
            _ => {}
        }
    }

    collected
}

fn to_group(module: &ItemMod, path: &Path, inner: Collected) -> Group {
    Group {
        name: module.ident.to_string(),
        file: path.to_path_buf(),
        line: Some(line_of(&module.ident)),
        children: inner.groups,
        specs: inner.specs,
    }
}

fn to_spec(function: &ItemFn, path: &Path) -> Spec {
    Spec {
        name: function.sig.ident.to_string(),
        file: path.to_path_buf(),
        line: line_of(&function.sig.ident),
        language: Language::Rust,
        status: if has_attribute(&function.attrs, "ignore") {
            SpecStatus::Skipped
        } else {
            SpecStatus::Active
        },
    }
}

fn is_container_module(module: &ItemMod) -> bool {
    let name = module.ident.to_string();
    CONTAINER_MODULE_NAMES.contains(&name.as_str())
}

/// `#[test]` / `#[tokio::test]` / `#[async_std::test]` などを一様に判定する。
///
/// パスの最終セグメントが `test` であることだけを見る。テストフレームワークごとに
/// 属性の完全パスを列挙すると、新しいランタイムが出るたびに spectoru の更新が
/// 必要になってしまう。
fn is_test_fn(function: &ItemFn) -> bool {
    function
        .attrs
        .iter()
        .any(|attr| last_segment_is(attr, "test"))
}

fn last_segment_is(attr: &Attribute, name: &str) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

/// 識別子の開始行（1 始まり）。
///
/// 行番号が取れるのは `proc-macro2` の `span-locations` feature が有効なとき
/// だけで、Cargo.toml でその理由と併せて明示的に指定している。
fn line_of(ident: &syn::Ident) -> u32 {
    u32::try_from(ident.span().start().line).unwrap_or(0)
}
