//! 生成物（JSON フラグメント / HTML サイト）を書き出すポート。
//!
//! [`FileWalker`](crate::ports::file_walker::FileWalker) が読み取り専用なのに対し、
//! こちらは書き込み専用。責務を分けることで、`lint` のように書き込みを一切
//! 行わないユースケースが「書き込む能力を持たない」ことを型で表現できる。
//!
//! テストでは書き込み内容をメモリに溜める Fake を注入し、実ファイルシステムに
//! 触れずに `extract` / `render` の出力を検証する。

use std::path::Path;

use crate::error::SpectoruError;

pub trait FileWriter: Send + Sync {
    /// `path` にテキストを書き出す。
    ///
    /// 親ディレクトリが存在しない場合は実装側で再帰的に作成する（`--out dist/`
    /// のように未作成のディレクトリを指定できる必要があるため）。既存ファイルは
    /// 上書きする。
    fn write(&self, path: &Path, contents: &str) -> Result<(), SpectoruError>;
}
