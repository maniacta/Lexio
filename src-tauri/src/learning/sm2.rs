/// SM-2 spaced repetition (classic SuperMemo-2 ease-factor update).
/// quality uses 0–5; values < 3 are treated as a failed recall.

pub struct Sm2Input {
    pub ease_factor: f64,
    pub interval_days: i32,
    pub repetitions: i32,
    pub is_correct: bool,
    /// 0–5 response quality (preferred). When omitted by callers, derive from is_correct.
    pub response_quality: i32,
}

pub struct Sm2Output {
    pub ease_factor: f64,
    pub interval_days: i32,
    pub repetitions: i32,
    pub next_review_at: String,
}

pub fn calculate(input: Sm2Input) -> Sm2Output {
    let Sm2Input {
        ease_factor,
        interval_days,
        repetitions,
        is_correct,
        response_quality,
    } = input;

    let q = response_quality.clamp(0, 5);
    // Prefer explicit quality; fall back to binary correctness
    let q = if !is_correct && q >= 3 { 1 } else if is_correct && q < 3 { 4 } else { q };

    let (reps, interval) = if q >= 3 {
        let reps = repetitions + 1;
        let interval = match reps {
            1 => 1,
            2 => 6,
            _ => ((interval_days as f64) * ease_factor).round().max(1.0) as i32,
        };
        (reps, interval)
    } else {
        (0, 1)
    };

    // Classic SM-2: EF' = EF + (0.1 - (5-q) * (0.08 + (5-q) * 0.02))
    let delta = 0.1 - (5 - q) as f64 * (0.08 + (5 - q) as f64 * 0.02);
    let ef = (ease_factor + delta).max(1.3).min(3.0);

    let next = chrono::Utc::now() + chrono::Duration::days(interval as i64);
    Sm2Output {
        ease_factor: ef,
        interval_days: interval,
        repetitions: reps,
        next_review_at: next.to_rfc3339(),
    }
}

/// SM-2 advances at most once per local calendar day per KP.
///
/// The day is the machine's local calendar, not UTC. A UTC+8 user reviewing at
/// 07:00 and 09:00 local would otherwise land on two UTC dates (23:00Z / 01:00Z)
/// and get two SM-2 steps in one morning; the reverse, two reviews that straddle
/// local midnight but stay on the same UTC date, would fail to advance at all.
pub fn should_advance_sm2(
    last_reviewed_at: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    should_advance_sm2_on(last_reviewed_at, now, &chrono::Local)
}

pub fn should_advance_sm2_on<Tz: chrono::TimeZone>(
    last_reviewed_at: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
    tz: &Tz,
) -> bool {
    let Some(last) = last_reviewed_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    else {
        return true;
    };
    last.with_timezone(tz).date_naive() != now.with_timezone(tz).date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_correct() {
        let output = calculate(Sm2Input {
            ease_factor: 2.5,
            interval_days: 0,
            repetitions: 0,
            is_correct: true,
            response_quality: 4,
        });
        assert_eq!(output.repetitions, 1);
        assert_eq!(output.interval_days, 1);
        assert!((output.ease_factor - 2.5).abs() < 0.001 || output.ease_factor > 2.4);
    }

    #[test]
    fn test_second_correct() {
        let output = calculate(Sm2Input {
            ease_factor: 2.5,
            interval_days: 1,
            repetitions: 1,
            is_correct: true,
            response_quality: 4,
        });
        assert_eq!(output.repetitions, 2);
        assert_eq!(output.interval_days, 6);
    }

    #[test]
    fn test_incorrect_resets() {
        let output = calculate(Sm2Input {
            ease_factor: 2.5,
            interval_days: 30,
            repetitions: 5,
            is_correct: false,
            response_quality: 1,
        });
        assert_eq!(output.repetitions, 0);
        assert_eq!(output.interval_days, 1);
        assert!(output.ease_factor < 2.5);
    }

    fn utc8() -> chrono::FixedOffset {
        chrono::FixedOffset::east_opt(8 * 3600).expect("UTC+8")
    }

    fn utc_at(rfc3339: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn sm2_advances_when_no_previous_review() {
        assert!(should_advance_sm2(None, chrono::Utc::now()));
    }

    #[test]
    fn sm2_does_not_advance_twice_same_day() {
        let now = chrono::Utc::now();
        let last = now.to_rfc3339();
        assert!(!should_advance_sm2(Some(&last), now));
    }

    #[test]
    fn sm2_advances_next_day() {
        let now = chrono::Utc::now();
        let yesterday = (now - chrono::Duration::days(1)).to_rfc3339();
        assert!(should_advance_sm2(Some(&yesterday), now));
    }

    #[test]
    fn sm2_does_not_advance_twice_on_the_same_local_day() {
        let last = "2026-09-17T23:00:00Z";
        let now = utc_at("2026-09-18T01:00:00Z");
        assert!(
            !should_advance_sm2_on(Some(last), now, &utc8()),
            "two morning reviews on the same local date must share one SM-2 step"
        );
        assert_ne!(
            utc_at(last).date_naive(),
            now.date_naive(),
            "the fixture really does straddle a UTC date"
        );
    }

    #[test]
    fn sm2_advances_when_the_local_date_changes_inside_one_utc_day() {
        let last = "2026-09-17T15:00:00Z";
        let now = utc_at("2026-09-17T17:00:00Z");
        assert!(
            should_advance_sm2_on(Some(last), now, &utc8()),
            "crossing local midnight must still advance, even on the same UTC date"
        );
        assert_eq!(
            utc_at(last).date_naive(),
            now.date_naive(),
            "the fixture really does stay on one UTC date"
        );
    }
}
