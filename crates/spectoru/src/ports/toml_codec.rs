//! `spec-site.toml` を `core::config::SpectoruConfig` に変換するポート。

use std::path::Path;

use crate::core::config::SpectoruConfig;
use crate::error::SpectoruError;

pub trait TomlCodec: Send + Sync {
    /// `path` は表示用（エラーメッセージに入る）、`source` が解釈対象本体。
    ///
    /// 設定ファイルの誤りは利用者が最も頻繁に出会うエラーであり、どのファイルの
    /// どこが悪いのかを伝えられなければ意味がない。パーサ系ポートと同じく
    /// パスと内容を分けて受け取る。
    fn decode_config(&self, path: &Path, source: &str) -> Result<SpectoruConfig, SpectoruError>;
}
