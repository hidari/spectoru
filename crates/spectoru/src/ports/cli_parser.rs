//! 生の引数列からドメイン値型 `CliCommand` への変換ポート。
//!
//! clap のような外部 crate を入れ替えても application 層が壊れないよう、
//! 戻り値は core が知る純値型のみで構成する。

use std::path::PathBuf;

use crate::error::SpectoruError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Build {
        config: PathBuf,
        out: PathBuf,
        strict: bool,
        revision: Option<String>,
    },
    Extract {
        config: PathBuf,
        out: PathBuf,
        strict: bool,
        revision: Option<String>,
    },
    Render {
        fragments: Vec<PathBuf>,
        out: PathBuf,
    },
    Lint {
        config: PathBuf,
        strict: bool,
    },
}

pub trait CliParser: Send + Sync {
    fn parse(&self, args: &[String]) -> Result<CliCommand, SpectoruError>;
}
