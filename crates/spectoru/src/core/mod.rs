//! ドメイン層: 外部 crate に依存しない純粋なロジック。
//!
//! library-contract パターンの中心であり、`adapters/` 側のどの実装にも縛られない
//! データ型と純関数だけで構成される。`extract` と `render` の application 層は
//! ここで定義された型を入出力に持つ。

pub mod config;
pub mod ir;
pub mod lint;
pub mod stats;
