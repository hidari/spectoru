//! IR ツリーを組み立てる際の純粋な補助関数。
//!
//! Rust パーサと Vitest パーサは対象言語こそ違うが、「ファイル 1 つを最上位
//! グループにする」「テストを含まない枝を落とす」という後処理は完全に同じ。
//! ここに置くことで両アダプタが同じ規則を共有し、規則そのものを core の
//! ユニットテストで固定できる。

use std::path::{Component, Path};

use crate::core::ir::{Group, Spec};

/// ファイル直下の最上位グループを組み立てる。
///
/// グループ名はパス表記（区切りは `/` に正規化）、`line` は `None`。
/// テストを含まないサブグループは [`prune_empty_groups`] で取り除く。
#[must_use]
pub fn file_group(path: &Path, children: Vec<Group>, specs: Vec<Spec>) -> Group {
    Group {
        name: normalize_path(path),
        file: path.to_path_buf(),
        line: None,
        children: prune_empty_groups(children),
        specs,
    }
}

/// パスを `/` 区切りの文字列にする。
///
/// 生成環境（Windows / Unix）によってフラグメントの内容が変わらないよう、
/// 区切り文字をここで正規化する。
#[must_use]
pub fn normalize_path(path: &Path) -> String {
    let mut out = String::new();
    for component in path.components() {
        match component {
            Component::RootDir => out.push('/'),
            Component::Prefix(prefix) => out.push_str(&prefix.as_os_str().to_string_lossy()),
            other => {
                if !out.is_empty() && !out.ends_with('/') {
                    out.push('/');
                }
                out.push_str(&other.as_os_str().to_string_lossy());
            }
        }
    }
    out
}

/// spec を 1 つも含まない（子孫まで見て空の）グループを取り除く純関数。
///
/// パーサは `mod` や `describe` を機械的にグループ化するため、テストを含まない
/// ヘルパーモジュールもそのままではツリーに残ってしまう。仕様サイトに意味の
/// ない枝を出さないよう、ここで刈り取る。残るグループの相対順序は保たれる。
#[must_use]
pub fn prune_empty_groups(groups: Vec<Group>) -> Vec<Group> {
    groups
        .into_iter()
        .filter_map(|mut group| {
            group.children = prune_empty_groups(group.children);
            if group.specs.is_empty() && group.children.is_empty() {
                return None;
            }
            Some(group)
        })
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::core::ir::{Language, SpecStatus};

    fn group(name: &str, children: Vec<Group>, specs: Vec<Spec>) -> Group {
        Group {
            name: name.to_string(),
            file: PathBuf::from("foo.rs"),
            line: Some(1),
            children,
            specs,
        }
    }

    fn spec(name: &str) -> Spec {
        Spec {
            name: name.to_string(),
            file: PathBuf::from("foo.rs"),
            line: 1,
            language: Language::Rust,
            status: SpecStatus::Active,
        }
    }

    #[test]
    fn specを持たないグループは取り除かれる() {
        assert_eq!(prune_empty_groups(vec![group("empty", vec![], vec![])]), []);
    }

    #[test]
    fn specを持つグループは残る() {
        let groups = prune_empty_groups(vec![group("kept", vec![], vec![spec("テスト")])]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "kept");
    }

    #[test]
    fn 子孫にspecがあれば中間グループも残る() {
        let leaf = group("leaf", vec![], vec![spec("テスト")]);
        let groups = prune_empty_groups(vec![group("middle", vec![leaf], vec![])]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].children.len(), 1);
    }

    #[test]
    fn 空の子グループだけを持つグループも取り除かれる() {
        let empty_leaf = group("leaf", vec![], vec![]);
        assert_eq!(
            prune_empty_groups(vec![group("middle", vec![empty_leaf], vec![])]),
            []
        );
    }

    #[test]
    fn 空の枝だけが取り除かれ他は残る() {
        let kept = group("kept", vec![], vec![spec("テスト")]);
        let dropped = group("dropped", vec![], vec![]);
        let groups = prune_empty_groups(vec![dropped.clone(), kept, dropped]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "kept");
    }

    #[test]
    fn 残るグループの順序は保たれる() {
        let groups = prune_empty_groups(vec![
            group("a", vec![], vec![spec("1")]),
            group("empty", vec![], vec![]),
            group("b", vec![], vec![spec("2")]),
        ]);
        let names: Vec<&str> = groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn file_groupはパスを名前にしlineを持たない() {
        let g = file_group(Path::new("tests/foo.rs"), vec![], vec![spec("テスト")]);
        assert_eq!(g.name, "tests/foo.rs");
        assert_eq!(g.file, PathBuf::from("tests/foo.rs"));
        assert_eq!(g.line, None);
    }

    #[test]
    fn file_groupは空のサブグループを取り除く() {
        let g = file_group(
            Path::new("tests/foo.rs"),
            vec![group("empty", vec![], vec![])],
            vec![spec("テスト")],
        );
        assert_eq!(g.children, []);
    }

    #[test]
    fn パス区切りはスラッシュに正規化される() {
        assert_eq!(normalize_path(Path::new("a/b/c.rs")), "a/b/c.rs");
        assert_eq!(normalize_path(Path::new("foo.rs")), "foo.rs");
    }

    #[test]
    fn 絶対パスは先頭のスラッシュを重複させない() {
        assert_eq!(normalize_path(Path::new("/a/b.rs")), "/a/b.rs");
    }

    #[test]
    fn 空のパスは空文字になる() {
        assert_eq!(normalize_path(Path::new("")), "");
    }
}
