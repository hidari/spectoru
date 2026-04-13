//! IR を 1 つの HTML 文字列に変換するポート。
//!
//! 複数プロジェクト集約も同じ trait で扱えるよう、入力は `&[IntermediateRepresentation]`
//! とする。実装側の責務として CSS / JS / 検索インデックスを単一 HTML に
//! インライン化する。

use crate::core::ir::IntermediateRepresentation;
use crate::error::SpectoruError;

pub trait TemplateEngine: Send + Sync {
    fn render_site(&self, projects: &[IntermediateRepresentation])
    -> Result<String, SpectoruError>;
}
