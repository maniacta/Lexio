/// SM-2 间隔复习算法
/// 输入：当前 mastery_record 和本次作答是否正确
/// 输出：更新后的 ease_factor, interval_days, repetitions, next_review_at

pub struct Sm2Input {
    pub ease_factor: f64,
    pub interval_days: i32,
    pub repetitions: i32,
    pub is_correct: bool,
    pub response_quality: i32, // 0-5, 仅 is_correct=false 时使用
}

pub struct Sm2Output {
    pub ease_factor: f64,
    pub interval_days: i32,
    pub repetitions: i32,
    pub next_review_at: String,
}

pub fn calculate(input: Sm2Input) -> Sm2Output {
    let Sm2Input { ease_factor, interval_days, repetitions, is_correct, response_quality } = input;

    if is_correct {
        let reps = repetitions + 1;
        let interval = match reps {
            1 => 1,
            2 => 6,
            _ => (interval_days as f64 * ease_factor).round() as i32,
        };
        // ease factor increases when correct repeatedly
        let ef = (ease_factor + 0.1).max(1.3).min(3.0);
        let next = chrono::Utc::now() + chrono::Duration::days(interval as i64);
        Sm2Output {
            ease_factor: ef,
            interval_days: interval,
            repetitions: reps,
            next_review_at: next.to_rfc3339(),
        }
    } else {
        // Reset: review again tomorrow, ease factor drops
        let ef = (ease_factor - 0.2 - (0.02 * (5 - response_quality) as f64)).max(1.3);
        Sm2Output {
            ease_factor: ef,
            interval_days: 1,
            repetitions: 0,
            next_review_at: (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339(),
        }
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
            response_quality: 5,
        });
        assert_eq!(output.repetitions, 1);
        assert_eq!(output.interval_days, 1);
        assert!(output.ease_factor > 2.5);
    }

    #[test]
    fn test_second_correct() {
        let output = calculate(Sm2Input {
            ease_factor: 2.5,
            interval_days: 1,
            repetitions: 1,
            is_correct: true,
            response_quality: 5,
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
            response_quality: 2,
        });
        assert_eq!(output.repetitions, 0);
        assert_eq!(output.interval_days, 1);
        assert!(output.ease_factor < 2.5);
    }
}
