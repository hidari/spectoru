//! `spec-site.toml` を `core::config::SpectruConfig` に変換するポート。

use crate::core::config::SpectruConfig;
use crate::error::SpectoruError;

pub trait TomlCodec: Send + Sync {
    fn decode_config(&self, source: &str) -> Result<SpectruConfig, SpectoruError>;
}
