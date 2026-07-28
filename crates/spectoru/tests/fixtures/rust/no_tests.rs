// テストを 1 つも含まないプロダクションコード。

pub struct Builder;

impl Builder {
    pub fn build(&self) -> u32 {
        0
    }
}

mod helpers {
    pub fn noop() {}
}
