//! `spec-site.toml` を表現するドメイン値型。
//!
//! TOML 由来の生表現を `adapters::toml_codec_impl` がここで定義された純値型に
//! 変換する。これ以降の application 層は TOML 形式に依存しない。

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpectoruConfig {
    pub project: ProjectConfig,
    pub sources: Vec<SourceConfig>,
    pub lint: LintConfig,
}

/// `[project]` セクション。
///
/// `repository` が source ではなくここにあるのは、1 つの設定ファイルが 1 つの
/// 作業ディレクトリを指し、その中の全 source が同じリポジトリに属するため。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub name: String,
    pub repository: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceConfig {
    pub name: String,
    pub kind: SourceKind,
    pub paths: Vec<PathBuf>,
    /// 探索対象から外すパス。設定ファイルからの相対で、前方一致で判定する。
    ///
    /// テストのフィクスチャや生成コードのように、ソースツリーには存在するが
    /// 仕様ではないものを外すために使う。glob ではなくパスの前方一致にしている
    /// のは、書いたとおりに効くことを最優先したため。
    pub exclude: Vec<PathBuf>,
}

impl SourceConfig {
    /// 設定ファイルからの相対パスが除外対象かを判定する。
    ///
    /// 比較はパスの構成要素単位で行う。`tests/fix` という指定が
    /// `tests/fixtures/a.rs` に誤って一致しないようにするため。
    #[must_use]
    pub fn excludes(&self, relative: &Path) -> bool {
        self.exclude
            .iter()
            .any(|prefix| relative.starts_with(prefix))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Rust,
    Vitest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LintConfig {
    pub max_depth: usize,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self { max_depth: 4 }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    fn source(exclude: &[&str]) -> SourceConfig {
        SourceConfig {
            name: "Tests".to_string(),
            kind: SourceKind::Rust,
            paths: vec![PathBuf::from("tests/")],
            exclude: exclude.iter().map(PathBuf::from).collect(),
        }
    }

    #[test]
    fn 除外指定が無ければ何も除外しない() {
        assert!(!source(&[]).excludes(Path::new("tests/foo.rs")));
    }

    #[test]
    fn 指定したディレクトリ配下を除外する() {
        let config = source(&["tests/fixtures"]);
        assert!(config.excludes(Path::new("tests/fixtures/rust/flat.rs")));
        assert!(!config.excludes(Path::new("tests/contract_foo.rs")));
    }

    #[test]
    fn 末尾のスラッシュは有無どちらでも同じに効く() {
        assert!(source(&["tests/fixtures/"]).excludes(Path::new("tests/fixtures/a.rs")));
    }

    #[test]
    fn ファイルを直接指定して除外できる() {
        let config = source(&["tests/broken.rs"]);
        assert!(config.excludes(Path::new("tests/broken.rs")));
        assert!(!config.excludes(Path::new("tests/broken_helper.rs")));
    }

    #[test]
    fn 途中で切れた名前には一致しない() {
        // 文字列の前方一致だと `tests/fix` が `tests/fixtures/` に誤って当たる。
        assert!(!source(&["tests/fix"]).excludes(Path::new("tests/fixtures/a.rs")));
    }

    #[test]
    fn 複数の除外指定はいずれかに当たれば除外する() {
        let config = source(&["tests/fixtures", "tests/support"]);
        assert!(config.excludes(Path::new("tests/fixtures/a.rs")));
        assert!(config.excludes(Path::new("tests/support/mod.rs")));
        assert!(!config.excludes(Path::new("tests/app.rs")));
    }
}
