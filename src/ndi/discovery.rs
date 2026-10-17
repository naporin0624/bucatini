/// How long the source list must stay unchanged before discovery is considered settled.
pub const DISCOVERY_SETTLE_MS: u32 = 1000;

/// Repeatedly calls `wait` until the source list settles or `timeout_ms` elapses.
///
/// `wait(budget_ms)` must block for at most `budget_ms` and return `true` if the
/// source list changed. `elapsed_ms` reports milliseconds since discovery started.
/// Before the first change is seen it keeps waiting until the deadline; after that
/// it returns as soon as one full quiet window passes without a change.
pub fn wait_until_settled(
    timeout_ms: u32,
    settle_ms: u32,
    mut elapsed_ms: impl FnMut() -> u32,
    mut wait: impl FnMut(u32) -> bool,
) {
    let mut seen_change = false;
    loop {
        let elapsed = elapsed_ms();
        if elapsed >= timeout_ms {
            return;
        }
        let changed = wait(settle_ms.min(timeout_ms - elapsed));
        if seen_change && !changed {
            return;
        }
        seen_change |= changed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    /// Runs `wait_until_settled` against a simulated clock. `change_at` lists the
    /// absolute times (ms) at which the source list changes; a wait returns early
    /// at the first pending change inside its budget. Returns each wait's budget
    /// and the final simulated time.
    fn simulate(timeout_ms: u32, settle_ms: u32, change_at: &[u32]) -> (Vec<u32>, u32) {
        let now = Cell::new(0u32);
        let pending = RefCell::new(change_at.to_vec());
        let budgets = RefCell::new(Vec::new());
        wait_until_settled(
            timeout_ms,
            settle_ms,
            || now.get(),
            |budget| {
                budgets.borrow_mut().push(budget);
                let start = now.get();
                let mut pending = pending.borrow_mut();
                match pending
                    .iter()
                    .position(|&t| t >= start && t <= start + budget)
                {
                    Some(i) => {
                        now.set(pending.remove(i));
                        true
                    }
                    None => {
                        now.set(start + budget);
                        false
                    }
                }
            },
        );
        (budgets.into_inner(), now.get())
    }

    #[test]
    fn stops_after_quiet_window_following_changes() {
        let (_, end) = simulate(5000, 1000, &[10, 300]);
        assert_eq!(end, 1300);
    }

    #[test]
    fn keeps_waiting_until_deadline_when_nothing_changes() {
        let (budgets, end) = simulate(5000, 1000, &[]);
        assert_eq!(end, 5000);
        assert_eq!(budgets, vec![1000; 5]);
    }

    #[test]
    fn waits_past_quiet_windows_until_first_change() {
        let (_, end) = simulate(5000, 1000, &[2500]);
        assert_eq!(end, 3500);
    }

    #[test]
    fn never_exceeds_deadline_when_changes_keep_arriving() {
        let changes: Vec<u32> = (1..100).map(|i| i * 500).collect();
        let (_, end) = simulate(2000, 1000, &changes);
        assert_eq!(end, 2000);
    }

    #[test]
    fn budget_is_clamped_to_remaining_time() {
        let (budgets, end) = simulate(2500, 1000, &[]);
        assert_eq!(budgets, vec![1000, 1000, 500]);
        assert_eq!(end, 2500);
    }

    #[test]
    fn zero_timeout_never_waits() {
        let (budgets, end) = simulate(0, 1000, &[]);
        assert!(budgets.is_empty());
        assert_eq!(end, 0);
    }
}
