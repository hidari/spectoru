//! システム時刻による [`Clock`](crate::ports::clock::Clock) 実装。
//!
//! `time` crate はエポック秒から暦日への変換にだけ使い、文字列の組み立ては
//! 自前で行う。well-known な RFC 3339 フォーマッタは値によって小数秒を
//! 出し分けるため、「常に秒精度」という契約を守るには自分で組む方が確実になる。

use time::OffsetDateTime;

use crate::ports::clock::Clock;

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_iso8601(&self) -> String {
        format(OffsetDateTime::now_utc())
    }
}

fn format(now: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
    )
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use time::{Date, Month};

    use super::*;

    fn utc(
        year: i32,
        month: Month,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> OffsetDateTime {
        Date::from_calendar_date(year, month, day)
            .expect("valid date")
            .with_hms_nano(hour, minute, second, nanosecond)
            .expect("valid time")
            .assume_utc()
    }

    #[test]
    fn 既知の時刻を秒精度のISO8601に変換する() {
        assert_eq!(
            format(utc(2026, Month::April, 13, 12, 0, 0, 0)),
            "2026-04-13T12:00:00Z"
        );
    }

    #[test]
    fn 一桁の月日時分秒はゼロ埋めされる() {
        assert_eq!(
            format(utc(2026, Month::January, 2, 3, 4, 5, 0)),
            "2026-01-02T03:04:05Z"
        );
    }

    #[test]
    fn 小数秒は切り捨てられる() {
        assert_eq!(
            format(utc(2026, Month::April, 13, 12, 0, 0, 987_654_321)),
            "2026-04-13T12:00:00Z"
        );
    }

    #[test]
    fn 閏日も正しく変換される() {
        assert_eq!(
            format(utc(2028, Month::February, 29, 23, 59, 59, 0)),
            "2028-02-29T23:59:59Z"
        );
    }
}
