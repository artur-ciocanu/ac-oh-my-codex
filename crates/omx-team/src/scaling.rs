/// Recommend the optimal worker count based on pending tasks.
/// Returns a value between 1 and max_workers.
/// Heuristic: 1 worker per 2 pending tasks, clamped to [current, max].
pub fn recommend_scale(current_workers: u8, pending_tasks: u32, max_workers: u8) -> u8 {
    if max_workers == 0 {
        return 0;
    }

    let ideal = ((pending_tasks as f64 / 2.0).ceil() as u8).max(1);
    ideal.clamp(current_workers.min(max_workers), max_workers)
}

/// Compute how many workers to add (positive) or remove (negative) to reach target.
pub fn scale_delta(current: u8, target: u8) -> i16 {
    target as i16 - current as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommend_scale_basic() {
        assert_eq!(recommend_scale(2, 4, 5), 2);
    }

    #[test]
    fn recommend_scale_many_tasks_caps_at_max() {
        assert_eq!(recommend_scale(2, 20, 5), 5);
    }

    #[test]
    fn recommend_scale_zero_tasks_keeps_current() {
        assert_eq!(recommend_scale(3, 0, 5), 3);
    }

    #[test]
    fn recommend_scale_zero_max_returns_zero() {
        assert_eq!(recommend_scale(0, 10, 0), 0);
    }

    #[test]
    fn recommend_scale_never_below_one() {
        assert_eq!(recommend_scale(0, 1, 5), 1);
    }

    #[test]
    fn scale_delta_positive() {
        assert_eq!(scale_delta(2, 5), 3);
    }

    #[test]
    fn scale_delta_negative() {
        assert_eq!(scale_delta(5, 2), -3);
    }

    #[test]
    fn scale_delta_zero() {
        assert_eq!(scale_delta(3, 3), 0);
    }
}
