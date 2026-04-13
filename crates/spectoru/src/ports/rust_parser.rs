//! Rust テストソースファイルをパースして 1 ファイル分の `Group` を返すポート。

use std::path::Path;

use crate::core::ir::{Diagnostic, Group};
use crate::error::SpectoruError;

/// 1 ファイルのパース結果。グループ本体と、パース中に検出された警告。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedFile {
    pub group: Group,
    pub diagnostics: Vec<Diagnostic>,
}

pub trait RustParser: Send + Sync {
    /// `path` は表示用（IR の `file` フィールドに入る）、`source` がパース対象本体。
    fn parse_file(&self, path: &Path, source: &str) -> Result<ParsedFile, SpectoruError>;
}
