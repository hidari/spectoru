// プロダクションコードに同居するユニットテスト。
// `mod tests` は仕様文としての意味を持たない容器なので階層から取り除かれる。

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 正の数どうしを足せる() {}

    mod 負の数を含むとき {
        #[test]
        fn 符号を保って計算する() {}
    }
}
