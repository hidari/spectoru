//! `IgnoreFileWalker` の契約テスト。
//!
//! ファイル探索に求められるのは「同じ木からは常に同じ集合が同じ順序で返る」
//! ことと、「ビルド生成物や無視指定されたファイルを拾わない」こと。
//! ウォーカーライブラリを差し替えてもこの性質は守られなければならない。

#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};

use spectoru::adapters::ignore_file_walker::IgnoreFileWalker;
use spectoru::error::SpectoruError;
use spectoru::ports::file_walker::FileWalker;
use tempfile::TempDir;

/// 相対パスの一覧を作る。ファイルは親ディレクトリごと作成する。
fn tree(files: &[(&str, &str)]) -> TempDir {
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

fn walk(roots: &[&Path], extensions: &[&str]) -> Vec<String> {
    let roots: Vec<PathBuf> = roots.iter().map(|r| r.to_path_buf()).collect();
    IgnoreFileWalker
        .walk(&roots, extensions)
        .expect("walk")
        .iter()
        .map(|file| file.relative.to_string_lossy().replace('\\', "/"))
        .collect()
}

#[test]
fn 指定した拡張子のファイルだけを返す() {
    let dir = tree(&[
        ("a.rs", ""),
        ("b.ts", ""),
        ("c.md", ""),
        ("d", ""),
        ("e.rs.bak", ""),
    ]);
    assert_eq!(walk(&[dir.path()], &["rs"]), ["a.rs"]);
}

#[test]
fn 複数の拡張子を同時に指定できる() {
    let dir = tree(&[("a.ts", ""), ("b.tsx", ""), ("c.rs", "")]);
    assert_eq!(walk(&[dir.path()], &["ts", "tsx"]), ["a.ts", "b.tsx"]);
}

#[test]
fn サブディレクトリを再帰的に探索する() {
    let dir = tree(&[("src/core/ir.rs", ""), ("tests/e2e.rs", "")]);
    assert_eq!(
        walk(&[dir.path()], &["rs"]),
        ["src/core/ir.rs", "tests/e2e.rs"]
    );
}

#[test]
fn relativeはrootからの相対パスになる() {
    // IR の `file` フィールドにそのまま入る形であること。
    let dir = tree(&[("src/core/ir.rs", "")]);
    let found = IgnoreFileWalker
        .walk(&[dir.path().to_path_buf()], &["rs"])
        .expect("walk");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].root, dir.path());
    assert_eq!(found[0].relative, PathBuf::from("src/core/ir.rs"));
}

#[test]
fn gitignoreで無視されたファイルを除外する() {
    let dir = tree(&[
        (".gitignore", "generated.rs\nbuild/\n"),
        ("kept.rs", ""),
        ("generated.rs", ""),
        ("build/output.rs", ""),
    ]);
    assert_eq!(walk(&[dir.path()], &["rs"]), ["kept.rs"]);
}

#[test]
fn targetとnode_modulesはgitignoreに無くても除外する() {
    let dir = tree(&[
        ("src/main.rs", ""),
        ("target/debug/build.rs", ""),
        ("app/node_modules/pkg/index.ts", ""),
        ("app/main.ts", ""),
    ]);
    assert_eq!(
        walk(&[dir.path()], &["rs", "ts"]),
        ["app/main.ts", "src/main.rs"]
    );
}

#[test]
fn 隠しディレクトリを探索しない() {
    let dir = tree(&[("src/main.rs", ""), (".hidden/secret.rs", "")]);
    assert_eq!(walk(&[dir.path()], &["rs"]), ["src/main.rs"]);
}

#[test]
fn 列挙順はパス順で決定的になる() {
    // 同じ入力から常に同じフラグメントが出ることが JSON 出力の前提。
    let dir = tree(&[
        ("z.rs", ""),
        ("a.rs", ""),
        ("m/b.rs", ""),
        ("m/a.rs", ""),
        ("b.rs", ""),
    ]);
    let expected = ["a.rs", "b.rs", "m/a.rs", "m/b.rs", "z.rs"];
    assert_eq!(walk(&[dir.path()], &["rs"]), expected);
    assert_eq!(walk(&[dir.path()], &["rs"]), expected);
}

#[test]
fn 複数のrootをまとめて探索する() {
    let dir = tree(&[("src/a.rs", ""), ("tests/b.rs", ""), ("docs/c.rs", "")]);
    let found = walk(
        &[&dir.path().join("src"), &dir.path().join("tests")],
        &["rs"],
    );
    assert_eq!(found, ["a.rs", "b.rs"]);
}

#[test]
fn rootが重なっていても同じファイルは一度だけ返る() {
    let dir = tree(&[("src/a.rs", "")]);
    let found = IgnoreFileWalker
        .walk(&[dir.path().to_path_buf(), dir.path().join("src")], &["rs"])
        .expect("walk");
    assert_eq!(found.len(), 1);
}

#[test]
fn rootがファイルそのものでも探索できる() {
    let dir = tree(&[("a.rs", "")]);
    let found = IgnoreFileWalker
        .walk(&[dir.path().join("a.rs")], &["rs"])
        .expect("walk");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].relative, PathBuf::from("a.rs"));
}

#[test]
fn テスト対象が無ければ空の結果を返す() {
    let dir = tree(&[("a.md", "")]);
    assert_eq!(walk(&[dir.path()], &["rs"]), Vec::<String>::new());
}

#[test]
fn 存在しないrootはFileWalkエラーになる() {
    // 設定に書いたパスが存在しないのは打ち間違いの可能性が高く、
    // 黙って空を返すと「なぜか何も出ない」という失敗になる。
    let dir = TempDir::new().expect("tempdir");
    let result = IgnoreFileWalker.walk(&[dir.path().join("missing")], &["rs"]);
    assert!(matches!(result, Err(SpectoruError::FileWalk { .. })));
}

#[test]
fn read_to_stringはファイル内容をそのまま返す() {
    let dir = tree(&[("a.rs", "fn 日本語のテスト() {}")]);
    let contents = IgnoreFileWalker
        .read_to_string(&dir.path().join("a.rs"))
        .expect("read");
    assert_eq!(contents, "fn 日本語のテスト() {}");
}

#[test]
fn 存在しないファイルの読み込みはIoエラーになる() {
    let dir = TempDir::new().expect("tempdir");
    let result = IgnoreFileWalker.read_to_string(&dir.path().join("missing.rs"));
    assert!(matches!(result, Err(SpectoruError::Io { .. })));
}
