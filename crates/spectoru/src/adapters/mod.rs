//! 外部 crate を実際に `use` する唯一のレイヤ。
//!
//! ここで定義される型は `ports/` の trait を実装する具象 adapter であり、
//! application 層（`extract/` `render/` `cli/`）からは trait オブジェクト経由で
//! 注入される。各 adapter は、対応する `tests/contract_*.rs` で「ライブラリ
//! 差し替え時に守るべき契約」を検証する。

pub mod serde_json_codec;
