//! 設定で指定されたディレクトリ群を歩き、対象ファイルを発見するポート。
//!
//! 実装側で `.gitignore` / `target` / `node_modules` などの除外規則を担う。
//! ここで返す `relative` は `root` からの相対パスで、IR の `file` フィールドに
//! そのまま入れられる形にしておく。

use std::path::{Path, PathBuf};

use crate::error::SpectoruError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredFile {
    pub root: PathBuf,
    pub relative: PathBuf,
}

pub trait FileWalker: Send + Sync {
    fn walk(
        &self,
        roots: &[PathBuf],
        extensions: &[&str],
    ) -> Result<Vec<DiscoveredFile>, SpectoruError>;

    /// 単一ファイルの内容を読み込む。テスト容易性のため walker 経由で読む。
    fn read_to_string(&self, path: &Path) -> Result<String, SpectoruError>;
}
