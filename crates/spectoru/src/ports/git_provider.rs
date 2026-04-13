//! 作業ディレクトリの git revision を解決するポート。
//!
//! 失敗（git 不在 / リポジトリ外）は `None` で表現し、診断レベルの判断は
//! application 層に委ねる。`Result` ではなく `Option` なのはそのためで、
//! 「取得不能」はエラーではなく中立的な事実として扱う。

use std::path::Path;

pub trait GitProvider: Send + Sync {
    fn current_revision(&self, repo_root: &Path) -> Option<String>;
}
