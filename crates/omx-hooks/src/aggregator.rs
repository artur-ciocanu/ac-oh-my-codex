use crate::HookResult;

/// Aggregated summary of multiple hook execution results.
#[derive(Debug, Clone)]
pub struct AggregatedHookResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub combined_stdout: String,
    pub combined_stderr: String,
    pub total_duration_ms: u64,
}

/// Aggregate a slice of [`HookResult`]s into a single summary.
pub fn aggregate_results(results: &[HookResult]) -> AggregatedHookResult {
    let total = results.len();
    let succeeded = results.iter().filter(|r| r.success).count();
    let failed = total - succeeded;

    let combined_stdout = results
        .iter()
        .map(|r| r.stdout.as_str())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let combined_stderr = results
        .iter()
        .map(|r| r.stderr.as_str())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let total_duration_ms = results.iter().map(|r| r.duration_ms).sum();

    AggregatedHookResult {
        total,
        succeeded,
        failed,
        combined_stdout,
        combined_stderr,
        total_duration_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(
        hook: &str,
        success: bool,
        stdout: &str,
        stderr: &str,
        duration_ms: u64,
    ) -> HookResult {
        HookResult {
            hook: hook.to_string(),
            success,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            duration_ms,
        }
    }

    #[test]
    fn aggregate_collects_all_stdout() {
        let results = vec![
            make_result("h1", true, "output1", "", 10),
            make_result("h2", true, "output2", "", 20),
            make_result("h3", true, "", "", 5),
        ];

        let agg = aggregate_results(&results);
        assert_eq!(agg.combined_stdout, "output1\noutput2");
        assert_eq!(agg.combined_stderr, "");
        assert_eq!(agg.total, 3);
        assert_eq!(agg.succeeded, 3);
        assert_eq!(agg.failed, 0);
    }

    #[test]
    fn aggregate_counts_failures() {
        let results = vec![
            make_result("h1", true, "ok", "", 10),
            make_result("h2", false, "", "error1", 20),
            make_result("h3", false, "", "error2", 30),
        ];

        let agg = aggregate_results(&results);
        assert_eq!(agg.succeeded, 1);
        assert_eq!(agg.failed, 2);
        assert_eq!(agg.total, 3);
        assert_eq!(agg.combined_stdout, "ok");
        assert_eq!(agg.combined_stderr, "error1\nerror2");
    }

    #[test]
    fn aggregate_empty_results() {
        let agg = aggregate_results(&[]);
        assert_eq!(agg.total, 0);
        assert_eq!(agg.succeeded, 0);
        assert_eq!(agg.failed, 0);
        assert_eq!(agg.combined_stdout, "");
        assert_eq!(agg.combined_stderr, "");
        assert_eq!(agg.total_duration_ms, 0);
    }

    #[test]
    fn aggregate_total_duration() {
        let results = vec![
            make_result("h1", true, "", "", 100),
            make_result("h2", true, "", "", 250),
            make_result("h3", false, "", "err", 50),
        ];

        let agg = aggregate_results(&results);
        assert_eq!(agg.total_duration_ms, 400);
    }
}
