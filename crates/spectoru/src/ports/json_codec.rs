//! IR と JSON 文字列の相互変換ポート。
//!
//! Spectoru の中間表現はファイルとして永続化・転送されるアーティファクトでも
//! あるため、人間可読な JSON を選択している。実装は `serde_json` を使うが、
//! その派生は adapter 側に閉じ込めてドメイン型 (`core::ir`) は serde 非依存に保つ。

use crate::core::ir::IntermediateRepresentation;
use crate::error::SpectoruError;

pub trait JsonCodec: Send + Sync {
    fn encode(&self, ir: &IntermediateRepresentation) -> Result<String, SpectoruError>;
    fn decode(&self, json: &str) -> Result<IntermediateRepresentation, SpectoruError>;
}
