//! Spectoru 全体で共有されるエラー型。
//!
//! 一つの enum に集約することで、`Result` 型に「あり得る失敗集合」が型として
//! 現れる。anyhow を使わない理由はここにあり、関数シグネチャがそのまま
//! ドキュメントになる。

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpectoruError {
    #[error("I/O 失敗: {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Rust ソースのパース失敗: {path}: {message}")]
    RustParse { path: PathBuf, message: String },

    #[error("TypeScript ソースのパース失敗: {path}: {message}")]
    TypeScriptParse { path: PathBuf, message: String },

    #[error("TOML 設定のパース失敗: {path}: {message}")]
    TomlParse { path: PathBuf, message: String },

    #[error("JSON エンコード失敗: {message}")]
    JsonEncode { message: String },

    #[error("JSON デコード失敗: {message}")]
    JsonDecode { message: String },

    #[error("テンプレート描画失敗: {message}")]
    TemplateRender { message: String },

    #[error("ファイル探索失敗 (root: {root}): {message}")]
    FileWalk { root: PathBuf, message: String },

    #[error("CLI 引数エラー: {message}")]
    CliArgs { message: String },
}
