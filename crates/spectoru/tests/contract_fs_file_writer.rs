//! `FsFileWriter` の契約テスト。

#![allow(non_snake_case)]

use std::fs;

use spectoru::adapters::fs_file_writer::FsFileWriter;
use spectoru::error::SpectoruError;
use spectoru::ports::file_writer::FileWriter;
use tempfile::TempDir;

#[test]
fn 書き込んだ内容をそのまま読み戻せる() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("fragment.json");

    FsFileWriter
        .write(&path, "{\"project\":{}}")
        .expect("write");

    assert_eq!(fs::read_to_string(&path).expect("read"), "{\"project\":{}}");
}

#[test]
fn 親ディレクトリが無ければ再帰的に作成する() {
    // `--out dist/` のように未作成のディレクトリを指定できる必要がある。
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("dist/site/index.html");

    FsFileWriter.write(&path, "<!doctype html>").expect("write");

    assert_eq!(fs::read_to_string(&path).expect("read"), "<!doctype html>");
}

#[test]
fn 既存ファイルは上書きされる() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("out.html");

    FsFileWriter.write(&path, "古い内容").expect("write");
    FsFileWriter.write(&path, "新しい").expect("overwrite");

    // 前の内容が残らないこと（切り詰められること）。
    assert_eq!(fs::read_to_string(&path).expect("read"), "新しい");
}

#[test]
fn 日本語を含む内容をUTF8で書き出す() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("out.html");

    FsFileWriter
        .write(&path, "作品が公開状態で作成される")
        .expect("write");

    assert_eq!(
        fs::read(&path).expect("read"),
        "作品が公開状態で作成される".as_bytes()
    );
}

#[test]
fn 書き込めない場所を指定するとIoエラーになる() {
    let dir = TempDir::new().expect("tempdir");
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "").expect("write");

    // ファイルをディレクトリとして扱おうとするため親の作成に失敗する。
    let result = FsFileWriter.write(&blocker.join("child/out.html"), "x");

    assert!(matches!(result, Err(SpectoruError::Io { .. })));
}
