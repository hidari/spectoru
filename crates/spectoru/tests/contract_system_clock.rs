//! `SystemClock` の契約テスト。
//!
//! 実時刻を返す以上、値そのものは検証できない。検証するのは形式の契約
//! （秒精度・UTC・ISO 8601）であり、これが崩れるとフラグメントの
//! `extracted_at` が生成環境によって別物になる。

#![allow(non_snake_case)]

use spectoru::adapters::system_clock::SystemClock;
use spectoru::ports::clock::Clock;

#[test]
fn 秒精度のISO8601形式で返す() {
    let now = SystemClock.now_iso8601();
    let bytes = now.as_bytes();

    assert_eq!(now.len(), 20, "got {now}");
    assert_eq!(bytes[4], b'-', "got {now}");
    assert_eq!(bytes[7], b'-', "got {now}");
    assert_eq!(bytes[10], b'T', "got {now}");
    assert_eq!(bytes[13], b':', "got {now}");
    assert_eq!(bytes[16], b':', "got {now}");
}

#[test]
fn 常にUTCとして返す() {
    // ローカルタイムゾーンを含む表現だと、生成環境によってフラグメントの
    // 内容が変わってしまう。
    let now = SystemClock.now_iso8601();
    assert!(now.ends_with('Z'), "got {now}");
    assert!(!now.contains('+'), "got {now}");
}

#[test]
fn 小数秒を含まない() {
    let now = SystemClock.now_iso8601();
    assert!(!now.contains('.'), "got {now}");
}

#[test]
fn 日付部分と時刻部分が妥当な範囲に収まる() {
    let now = SystemClock.now_iso8601();
    let number = |range: std::ops::Range<usize>| {
        now[range]
            .parse::<u32>()
            .unwrap_or_else(|e| panic!("parse {now}: {e}"))
    };

    assert!(number(0..4) >= 2024, "got {now}");
    assert!((1..=12).contains(&number(5..7)), "got {now}");
    assert!((1..=31).contains(&number(8..10)), "got {now}");
    assert!(number(11..13) <= 23, "got {now}");
    assert!(number(14..16) <= 59, "got {now}");
    assert!(number(17..19) <= 60, "got {now}");
}
