//! `CommandGitProvider` の契約テスト。
//!
//! 契約の中心は「取得できないときに例外を投げず `None` を返す」こと。
//! git の有無やリポジトリの状態は spectoru が制御できない環境要因であり、
//! そこで処理を止めると仕様サイトの生成そのものが失敗してしまう。

#![allow(non_snake_case)]

use std::path::Path;
use std::process::Command;

use spectoru::adapters::command_git_provider::CommandGitProvider;
use spectoru::ports::git_provider::GitProvider;
use tempfile::TempDir;

/// コミットを 1 つ持つ git リポジトリを作る。
///
/// 利用者のグローバル設定に依存しないよう、identity は `-c` で明示的に与える。
fn repository_with_commit() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    run(dir.path(), &["init", "--quiet"]);
    run(
        dir.path(),
        &[
            "-c",
            "user.email=spectoru@example.com",
            "-c",
            "user.name=spectoru",
            "commit",
            "--allow-empty",
            "--quiet",
            "-m",
            "initial",
        ],
    );
    dir
}

fn run(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .status()
        .expect("git available");
    assert!(status.success(), "git {args:?} failed");
}

#[test]
fn コミットのあるリポジトリではSHAを返す() {
    let repository = repository_with_commit();

    let revision = CommandGitProvider
        .current_revision(repository.path())
        .expect("revision");

    assert_eq!(revision.len(), 40, "got {revision}");
    assert!(
        revision.chars().all(|c| c.is_ascii_hexdigit()),
        "got {revision}"
    );
}

#[test]
fn 返り値に改行や空白が混ざらない() {
    // そのまま JSON フラグメントに載るため、トリムされている必要がある。
    let repository = repository_with_commit();

    let revision = CommandGitProvider
        .current_revision(repository.path())
        .expect("revision");

    assert_eq!(revision, revision.trim());
}

#[test]
fn 同じリポジトリからは同じSHAが返る() {
    let repository = repository_with_commit();
    assert_eq!(
        CommandGitProvider.current_revision(repository.path()),
        CommandGitProvider.current_revision(repository.path())
    );
}

#[test]
fn コミットが1つも無いリポジトリではNoneを返す() {
    let dir = TempDir::new().expect("tempdir");
    run(dir.path(), &["init", "--quiet"]);

    assert_eq!(CommandGitProvider.current_revision(dir.path()), None);
}

#[test]
fn 存在しないディレクトリではNoneを返す() {
    let dir = TempDir::new().expect("tempdir");
    assert_eq!(
        CommandGitProvider.current_revision(&dir.path().join("missing")),
        None
    );
}
