//! `spec-site.toml` を `core::config::SpectoruConfig` に変換するポート。

use crate::core::config::SpectoruConfig;
use crate::error::SpectoruError;

pub trait TomlCodec: Send + Sync {
    fn decode_config(&self, source: &str) -> Result<SpectoruConfig, SpectoruError>;
}
