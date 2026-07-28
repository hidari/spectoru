//! アーキテクチャ境界の実行可能な仕様。
//!
//! `core/` と `ports/` は外部 crate に依存してはならない、という library-contract
//! パターンの根幹をソース走査で機械的に担保する。この規律が破れると、外部
//! ライブラリの破壊的変更やサプライチェーン侵害がドメイン層まで到達しうる。
//!
//! 走査対象はプロダクションコードのみ（各ファイル末尾の `#[cfg(test)] mod tests`
//! より前）。テストコードが dev-dependency を使うことは境界違反ではない。

#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};

/// 標準ライブラリと自クレートを指す `use` のルート。これ以外は外部 crate とみなす。
const ALLOWED_USE_ROOTS: &[&str] = &["crate", "std", "core", "alloc", "self", "super"];

/// derive マクロのうち std が提供するもの。これ以外は外部 crate 由来。
const BUILTIN_DERIVES: &[&str] = &[
    "Debug",
    "Clone",
    "Copy",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Default",
];

const TEST_MODULE_MARKER: &str = "#[cfg(test)]";

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// `dir` 配下の `.rs` ファイルをパス順（決定的）に列挙する。
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// ファイルからプロダクションコード部分だけを取り出す。
///
/// `#[cfg(test)]` はファイル末尾のテストモジュールにのみ現れるという本プロジェクトの
/// 規約に依存するため、複数回出現したら（＝規約が崩れたら）明示的に失敗させる。
fn production_source(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let occurrences = text.matches(TEST_MODULE_MARKER).count();
    assert!(
        occurrences <= 1,
        "{}: {TEST_MODULE_MARKER} が {occurrences} 回出現している。\
         このテストは「テストモジュールはファイル末尾に1つだけ」という規約に依存する。",
        path.display()
    );
    match text.find(TEST_MODULE_MARKER) {
        Some(index) => text[..index].to_string(),
        None => text,
    }
}

/// `use` 宣言のルート識別子のうち、外部 crate を指すものを返す。
fn external_use_roots(source: &str) -> Vec<String> {
    source
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            line.strip_prefix("pub use ")
                .or_else(|| line.strip_prefix("use "))
        })
        .map(|rest| {
            rest.trim_start_matches("::")
                .split([':', ';', '{', ' '])
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .filter(|root| !root.is_empty() && !ALLOWED_USE_ROOTS.contains(&root.as_str()))
        .collect()
}

/// `#[derive(..)]` に列挙された derive 名をすべて返す。
fn derive_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("#[derive(") {
        let after = &rest[start + "#[derive(".len()..];
        let Some(end) = after.find(')') else { break };
        names.extend(
            after[..end]
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string),
        );
        rest = &after[end..];
    }
    names
}

/// 外部 crate を `use` してはならない層。
///
/// `app/` を含むのは、ユースケースが具体的なライブラリを直接掴んだ時点で
/// library-contract パターンが形骸化するため。I/O はすべてポート越しに行う。
const DOMAIN_DIRECTORIES: &[&str] = &["core", "ports", "app"];

fn domain_files() -> Vec<PathBuf> {
    DOMAIN_DIRECTORIES
        .iter()
        .flat_map(|directory| rust_files(&src_dir().join(directory)))
        .collect()
}

#[test]
fn coreとportsのプロダクションコードは外部crateをuseしない() {
    let files = domain_files();
    assert!(!files.is_empty(), "走査対象が見つからない");

    for path in files {
        let externals = external_use_roots(&production_source(&path));
        assert!(
            externals.is_empty(),
            "{} が外部 crate を use している: {externals:?}\n\
             外部 crate を use してよいのは adapters/ 配下だけ。",
            path.display()
        );
    }
}

#[test]
fn coreとportsのプロダクションコードは組み込みderiveしか使わない() {
    for path in domain_files() {
        for name in derive_names(&production_source(&path)) {
            assert!(
                BUILTIN_DERIVES.contains(&name.as_str()),
                "{} が外部 crate 由来の derive を使っている: {name}\n\
                 serde などの派生は adapters/ の DTO 側に閉じ込めること。",
                path.display()
            );
        }
    }
}

#[test]
fn error_rsが依存する外部crateはthiserrorだけ() {
    // SpectoruError は ports のシグネチャに現れるため core 相当の位置づけだが、
    // thiserror は Display/Error impl を生成するだけで型の公開 API には現れない。
    // 例外を明示的にテストで固定し、なし崩しに外部依存が増えるのを防ぐ。
    let externals = external_use_roots(&production_source(&src_dir().join("error.rs")));
    assert_eq!(externals, vec!["thiserror".to_string()]);
}

#[test]
fn 外部crate検出が機能していることをadaptersで確認する() {
    // 検出器が壊れていても上のテストは空振りして通ってしまう。
    // 外部 crate を使っているとわかっているファイルで検出できることを確かめる。
    let path = src_dir().join("adapters/serde_json_codec.rs");
    let externals = external_use_roots(&production_source(&path));
    assert!(
        externals.contains(&"serde".to_string()),
        "検出器が adapters の外部 crate を見つけられていない: {externals:?}"
    );
}
