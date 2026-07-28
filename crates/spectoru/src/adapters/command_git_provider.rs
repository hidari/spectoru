//! `git` コマンド実行による [`GitProvider`](crate::ports::git_provider::GitProvider) 実装。
//!
//! git2 のようなライブラリを使わないのは、必要なのが `rev-parse HEAD` 一つだけで、
//! そのために C ライブラリ依存を抱える理由がないため。
//!
//! セキュリティ上の注意点として、対象ディレクトリは `-C <path>` 引数ではなく
//! `current_dir` で渡している。`-C` 経由だとパスが `-` で始まる場合に git が
//! オプションとして解釈しうるため。シェルを介さないので引数の分割やグロブ展開も
//! 起きない。

use std::path::Path;
use std::process::Command;

use crate::ports::git_provider::GitProvider;

#[derive(Debug, Default, Clone, Copy)]
pub struct CommandGitProvider;

impl GitProvider for CommandGitProvider {
    fn current_revision(&self, repo_root: &Path) -> Option<String> {
        // 失敗はすべて None に畳む。git が無い / リポジトリ外 / コミットが 1 つも
        // 無いのいずれも「取得できなかった」という同じ事実でしかなく、
        // 診断として扱うかどうかは application 層の判断になる。
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let revision = String::from_utf8(output.stdout).ok()?.trim().to_string();

        if revision.is_empty() {
            return None;
        }
        Some(revision)
    }
}
