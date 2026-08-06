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
}
