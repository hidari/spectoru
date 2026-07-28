//! Spectoru の中間表現 (IR)。
//!
//! ここで定義される型は外部 crate に一切依存しない。serde 派生は
//! `adapters::serde_json_codec` 側の DTO 型で行い、ドメイン型は serialization
//! フォーマットに縛られない純粋な値型として保つ。
//!
//! 階層は `project → source → group → spec` の 4 段。1 回の extract
//! （= 1 つの `spec-site.toml`）が 1 つの [`IntermediateRepresentation`] を生み、
//! それが JSON フラグメント 1 ファイルに対応する。複数リポジトリの集約は
//! `render` が `&[IntermediateRepresentation]` を受け取ることで表現される。

use std::path::PathBuf;

/// 1 回の extract 結果のルート。JSON フラグメント 1 ファイルと 1:1 に対応する。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IntermediateRepresentation {
    pub project: ProjectMeta,
    pub sources: Vec<Source>,
    pub diagnostics: Vec<Diagnostic>,
    pub stats: Stats,
}

/// extract を行ったプロジェクトのメタ情報。
///
/// `repository` / `revision` / `extracted_at` が source ではなくここに載るのは、
/// 1 つの `spec-site.toml` が 1 つの作業ディレクトリ（= 1 つの git リポジトリ）を
/// 指すため。同一リポジトリ内の複数 source は必ず同じ revision を共有する。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMeta {
    pub name: String,
    pub repository: Option<String>,
    /// git SHA。取得できなかった場合は `None`。
    pub revision: Option<String>,
    /// ISO 8601 UTC 文字列。`chrono` 等を core に持ち込まないため、生成は
    /// [`Clock`](crate::ports::clock::Clock) ポートに委ね、ここでは文字列で保持する。
    pub extracted_at: String,
}

/// `[[sources]]` 1 件分の抽出結果。
///
/// 言語は spec ごとの [`Language`] が事実として保持するため、ここには複製しない。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Source {
    pub name: String,
    pub groups: Vec<Group>,
}

/// テストのグルーピング。ファイルパスや `mod` / `describe` を表現する。
///
/// ファイル直下のグループは `line == None`、`mod` / `describe` 由来のグループは
/// `line == Some(行番号)` を持つ。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Group {
    pub name: String,
    pub file: PathBuf,
    pub line: Option<u32>,
    pub children: Vec<Group>,
    pub specs: Vec<Spec>,
}

/// 個々のテスト = 仕様文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub name: String,
    pub file: PathBuf,
    pub line: u32,
    pub language: Language,
    pub status: SpecStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    TypeScript,
}

impl Language {
    /// 機械可読な識別子。JSON・HTML・CLI で同じ表記を使う。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecStatus {
    Active,
    Skipped,
}

/// extract 中に検出された警告 / エラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub code: DiagnosticCode,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticLevel {
    Warning,
    Error,
}

impl DiagnosticLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// diagnostics の機械可読な分類コード。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// ネスト深さが設定上限を超えている。
    NestingTooDeep,
    /// グループ名 / spec 名が空文字列。
    EmptyName,
    /// テスト名が静的に決定できない（テンプレートリテラル補間など）。
    DynamicTestName,
    /// git revision の取得に失敗した。
    GitRevisionUnavailable,
    /// パーサがソースファイルを解釈できなかった。
    ParseError,
    /// 探索で見つかったファイルを読み出せなかった。
    FileUnreadable,
}

impl DiagnosticCode {
    /// 機械可読な分類コード。
    ///
    /// メッセージ文言の変更が CI の判定を壊さないよう、細かい制御はこの
    /// 文字列を使う。JSON・HTML・CLI のどこでも同じ表記になるよう、
    /// 出どころをここ 1 箇所に保つ。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NestingTooDeep => "nesting_too_deep",
            Self::EmptyName => "empty_name",
            Self::DynamicTestName => "dynamic_test_name",
            Self::GitRevisionUnavailable => "git_revision_unavailable",
            Self::ParseError => "parse_error",
            Self::FileUnreadable => "file_unreadable",
        }
    }
}

/// extract 結果の集計値。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    pub total_specs: usize,
    pub warnings: usize,
    pub languages: Languages,
}

/// 言語別 spec 数の内訳。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Languages {
    pub rust: usize,
    pub typescript: usize,
}
