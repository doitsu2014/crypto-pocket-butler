//! The local period window a brief covers.
//!
//! WF1: a daily brief covers the most recent local day ending at 07:00 local, and
//! a weekly brief the most recent ISO week ending Monday at 07:00 local. C3 tells
//! Reporting which users are due and in which timezone; Reporting derives the
//! window here so the boundary is measured once, in one place.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use reporting_domain::Period;

/// The hour, local time, at which a period closes.
pub const BOUNDARY_HOUR: u32 = 7;

/// The instant the period most recently closed at or before `at`, and the
/// instant the next period closes.
///
/// An unparseable timezone falls back to UTC, which keeps the user on a defined
/// boundary instead of skipping their brief. A window that cannot be resolved
/// (a nonexistent local instant at a daylight-saving gap) returns `None` and the
/// caller records the brief as failed.
#[must_use]
pub fn window(
    period: Period,
    timezone: &str,
    at: DateTime<Utc>,
) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let zone: Tz = timezone.trim().parse().unwrap_or(chrono_tz::UTC);
    let local = at.with_timezone(&zone);
    let local_date = local.date_naive();
    let local_time = NaiveTime::from_hms_opt(BOUNDARY_HOUR, 0, 0)?;

    let closing_date = match period {
        Period::Daily => {
            let today = local_date.and_time(local_time);
            if local.naive_local() >= today {
                local_date
            } else {
                previous_day(local_date)?
            }
        }
        Period::Weekly => {
            let boundary = local_date
                .and_time(local_time)
                .checked_sub_signed(Duration::days(i64::from(
                    local_date.weekday().num_days_from_monday(),
                )))?;
            if local.naive_local() >= boundary {
                boundary.date()
            } else {
                // Only Monday before 07:00 local reaches here; the boundary that
                // just closed is the one a week earlier.
                boundary.date() - Duration::days(7)
            }
        }
    };
    let start_local = NaiveDateTime::new(closing_date, local_time);
    let end_local = start_local.checked_add_signed(match period {
        Period::Daily => Duration::days(1),
        Period::Weekly => Duration::days(7),
    })?;
    Some((resolve(&zone, start_local)?, resolve(&zone, end_local)?))
}

fn previous_day(date: NaiveDate) -> Option<NaiveDate> {
    date.checked_sub_signed(Duration::days(1))
}

fn resolve(zone: &Tz, local: NaiveDateTime) -> Option<DateTime<Utc>> {
    match zone.from_local_datetime(&local) {
        chrono::LocalResult::Single(instant) => Some(instant.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(earliest, _) => Some(earliest.with_timezone(&Utc)),
        chrono::LocalResult::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    fn at(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw)
            .expect("a valid instant")
            .with_timezone(&Utc)
    }

    #[test]
    fn a_daily_brief_after_the_boundary_covers_the_day_that_just_closed() {
        let (start, end) =
            window(Period::Daily, "UTC", at("2026-11-08T08:00:00Z")).expect("a resolvable window");
        assert_eq!(start, at("2026-11-08T07:00:00Z"));
        assert_eq!(end, at("2026-11-09T07:00:00Z"));
    }

    #[test]
    fn a_daily_brief_before_the_boundary_covers_the_previous_day() {
        let (start, end) =
            window(Period::Daily, "UTC", at("2026-11-08T06:59:00Z")).expect("a resolvable window");
        assert_eq!(start, at("2026-11-07T07:00:00Z"));
        assert_eq!(end, at("2026-11-08T07:00:00Z"));
    }

    #[test]
    fn the_window_is_measured_in_the_users_timezone() {
        // 2026-11-08T08:00Z is 15:00 in Ho_Chi_Minh, so the local day closed at
        // 00:00Z on the same date.
        let (start, end) = window(
            Period::Daily,
            "Asia/Ho_Chi_Minh",
            at("2026-11-08T08:00:00Z"),
        )
        .expect("a resolvable window");
        assert_eq!(start, at("2026-11-08T00:00:00Z"));
        assert_eq!(end, at("2026-11-09T00:00:00Z"));
    }

    #[test]
    fn a_timezone_behind_utc_still_closes_at_seven_local() {
        // 2026-11-08T08:00Z is 03:00 in New York (EST), before the 07:00
        // boundary, so the window is the prior local day: 07:00 EST = 12:00Z.
        let (start, end) = window(
            Period::Daily,
            "America/New_York",
            at("2026-11-08T08:00:00Z"),
        )
        .expect("a resolvable window");
        assert_eq!(start, at("2026-11-07T12:00:00Z"));
        assert_eq!(end, at("2026-11-08T12:00:00Z"));
    }

    #[test]
    fn a_weekly_brief_ends_on_monday_at_seven_local() {
        let (start, end) =
            window(Period::Weekly, "UTC", at("2026-11-09T08:00:00Z")).expect("a resolvable window");
        assert_eq!(start, at("2026-11-09T07:00:00Z"));
        assert_eq!(end, at("2026-11-16T07:00:00Z"));
    }

    #[test]
    fn a_weekly_brief_before_monday_closes_the_prior_week() {
        let (start, end) =
            window(Period::Weekly, "UTC", at("2026-11-09T06:00:00Z")).expect("a resolvable window");
        assert_eq!(start, at("2026-11-02T07:00:00Z"));
        assert_eq!(end, at("2026-11-09T07:00:00Z"));
    }

    #[test]
    fn a_weekly_window_mid_week_closes_the_monday_that_just_passed() {
        let (start, _end) =
            window(Period::Weekly, "UTC", at("2026-11-11T06:00:00Z")).expect("a resolvable window");
        assert_eq!(start, at("2026-11-09T07:00:00Z"));
    }

    #[test]
    fn an_unknown_timezone_falls_back_to_utc() {
        let (start, _end) = window(Period::Daily, "Mars/Olympus", at("2026-11-08T08:00:00Z"))
            .expect("the fallback window resolves");
        assert_eq!(start, at("2026-11-08T07:00:00Z"));
    }

    #[test]
    fn a_daily_window_that_spans_spring_forward_is_twenty_three_hours() {
        // US spring forward: 2026-03-08. At 01:00 local (06:00Z) the window is
        // the prior day, 07:00 EST (12:00Z) to 07:00 EDT (11:00Z): 23 hours.
        let (start, end) = window(
            Period::Daily,
            "America/New_York",
            at("2026-03-08T06:00:00Z"),
        )
        .expect("a resolvable window");
        assert_eq!(end - start, Duration::hours(23));
        assert_eq!(end.with_timezone(&chrono_tz::America::New_York).hour(), 7);
    }

    #[test]
    fn a_daily_window_that_spans_fall_back_is_twenty_five_hours() {
        // US fall back: 2026-11-01. At 01:00 local (06:00Z) the window is the
        // prior day, 07:00 EDT (11:00Z) to 07:00 EST (12:00Z): 25 hours.
        let (start, end) = window(
            Period::Daily,
            "America/New_York",
            at("2026-11-01T06:00:00Z"),
        )
        .expect("a resolvable window");
        assert_eq!(end - start, Duration::hours(25));
    }

    #[test]
    fn a_boundary_day_after_the_shift_is_twenty_four_hours() {
        // 2026-03-08T13:00Z is 09:00 EDT; the window starts after the shift and
        // is an ordinary 24 hours.
        let (start, end) = window(
            Period::Daily,
            "America/New_York",
            at("2026-03-08T13:00:00Z"),
        )
        .expect("a resolvable window");
        assert_eq!(end - start, Duration::hours(24));
        assert_eq!(start, at("2026-03-08T11:00:00Z"));
    }
}
