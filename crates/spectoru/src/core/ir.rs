//! Spectoru の中間表現 (IR)。
//!
//! ここで定義される型は外部 crate に一切依存しない。serde 派生は
//! `adapters::serde_json_codec` 側の DTO 型で行い、ドメイン型は serialization
//! フォーマットに縛られない純粋な値型として保つ。

use std::path::PathBuf;

/// 1 ソース（リポジトリ）に対する extract 結果のルート。
///
/// `render` はこの構造を入力として静的サイトを生成する。複数リポジトリの集約は
/// `Vec<IntermediateRepresentation>` を入力に取るレンダラ側で扱う。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IntermediateRepresentation {
    pub source: SourceMeta,
    pub groups: Vec<Group>,
    pub diagnostics: Vec<Diagnostic>,
    pub stats: Stats,
}

/// extract を行ったソースのメタ情報。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceMeta {
    pub name: String,
    pub repository: Option<String>,
    pub revision: Option<String>,
    /// ISO 8601 UTC 文字列。`chrono` 等を入れたくないため文字列で保持する。
    pub extracted_at: String,
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
    /// パーサがソースファイルを読めなかった。
    ParseError,
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
