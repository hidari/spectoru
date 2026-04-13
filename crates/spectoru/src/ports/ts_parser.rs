//! TypeScript (Vitest) テストソースをパースするポート。
//!
//! Rust 側と API を揃えて `ParsedFile` を返す。動的テスト名（テンプレート
//! リテラル補間など）は IR には含めず、`diagnostics` に warning として記録する。

use std::path::Path;

use crate::error::SpectoruError;
use crate::ports::rust_parser::ParsedFile;

pub trait TsParser: Send + Sync {
    fn parse_file(&self, path: &Path, source: &str) -> Result<ParsedFile, SpectoruError>;
}
