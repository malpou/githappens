use crate::github::pr::CheckSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowCounts {
    pub completed: usize,
    pub total: usize,
}

pub fn count_checks(checks: &[CheckSnapshot]) -> WorkflowCounts {
    let total = checks.len();
    let completed = checks.iter().filter(|c| c.completed).count();
    WorkflowCounts { completed, total }
}

pub fn render_counts(counts: &WorkflowCounts) -> String {
    if counts.total == 0 {
        return "–/–".to_string();
    }
    let cap = |n: usize| -> String {
        if n > 99 {
            "99+".to_string()
        } else {
            n.to_string()
        }
    };
    format!("{}/{}", cap(counts.completed), cap(counts.total))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::github::pr::CheckKind;
    use rstest::rstest;

    fn check(completed: bool, failed: bool) -> CheckSnapshot {
        CheckSnapshot {
            name: "CI".to_string(),
            kind: CheckKind::CheckRun,
            completed,
            failed,
        }
    }

    #[rstest]
    fn all_completed() {
        let checks = vec![check(true, false), check(true, false)];
        let counts = count_checks(&checks);
        assert_eq!(
            counts,
            WorkflowCounts {
                completed: 2,
                total: 2
            }
        );
        assert_eq!(render_counts(&counts), "2/2");
    }

    #[rstest]
    fn some_pending() {
        let checks = vec![check(true, false), check(false, false), check(false, false)];
        let counts = count_checks(&checks);
        assert_eq!(
            counts,
            WorkflowCounts {
                completed: 1,
                total: 3
            }
        );
        assert_eq!(render_counts(&counts), "1/3");
    }

    #[rstest]
    fn zero_checks_renders_dash() {
        let counts = count_checks(&[]);
        assert_eq!(
            counts,
            WorkflowCounts {
                completed: 0,
                total: 0
            }
        );
        assert_eq!(render_counts(&counts), "–/–");
    }

    #[rstest]
    fn capped_at_99() {
        let checks: Vec<CheckSnapshot> = (0..100).map(|_| check(true, false)).collect();
        let counts = count_checks(&checks);
        assert_eq!(
            counts,
            WorkflowCounts {
                completed: 100,
                total: 100
            }
        );
        assert_eq!(render_counts(&counts), "99+/99+");
    }

    #[rstest]
    fn mixed_checkrun_and_status_context() {
        let checks = vec![
            CheckSnapshot {
                name: "CI".to_string(),
                kind: CheckKind::CheckRun,
                completed: true,
                failed: false,
            },
            CheckSnapshot {
                name: "travis".to_string(),
                kind: CheckKind::StatusContext,
                completed: true,
                failed: false,
            },
        ];
        let counts = count_checks(&checks);
        assert_eq!(
            counts,
            WorkflowCounts {
                completed: 2,
                total: 2
            }
        );
    }

    #[rstest]
    fn status_context_pending_not_completed() {
        let checks = vec![CheckSnapshot {
            name: "travis".to_string(),
            kind: CheckKind::StatusContext,
            completed: false,
            failed: false,
        }];
        let counts = count_checks(&checks);
        assert_eq!(
            counts,
            WorkflowCounts {
                completed: 0,
                total: 1
            }
        );
    }

    #[rstest]
    fn checkrun_completed_null_conclusion_not_completed() {
        let checks = vec![CheckSnapshot {
            name: "CI".to_string(),
            kind: CheckKind::CheckRun,
            completed: false,
            failed: false,
        }];
        let counts = count_checks(&checks);
        assert_eq!(
            counts,
            WorkflowCounts {
                completed: 0,
                total: 1
            }
        );
    }
}
