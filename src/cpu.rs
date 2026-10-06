//! CPU usage from `/proc/stat`.
//!
//! A usage percentage needs two samples. Rather than sleeping between them,
//! each run compares the current counters with the ones the previous run saved
//! to a small state file, so the percentage covers the interval between two
//! status-bar refreshes.

use std::fs;
use std::path::Path;

/// Minimum jiffies per CPU between two samples (half a second at the usual
/// USER_HZ of 100). Several tmux clients refresh at nearly the same moment;
/// the later ones would otherwise compute a percentage over a few
/// milliseconds, which is noise. They reuse the previous percentage instead.
const MIN_JIFFIES_PER_CPU: u64 = 50;

/// Aggregate counters from the `cpu` line of `/proc/stat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub total: u64,
    pub idle: u64,
    pub cpus: u64,
}

/// What a run saves for the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub total: u64,
    pub idle: u64,
    pub percent: u8,
}

/// Parses `/proc/stat`. Total time is user + nice + system + idle + iowait +
/// irq + softirq + steal (guest time is already counted in user and nice);
/// idle time is idle + iowait.
pub fn parse_stat(stat: &str) -> Option<Sample> {
    let mut lines = stat.lines();
    let first = lines.next()?;
    let mut fields = first.split_whitespace();
    if fields.next()? != "cpu" {
        return None;
    }
    let values: Vec<u64> = fields
        .take(8)
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if values.len() < 4 {
        return None;
    }
    let total = values.iter().sum();
    let idle = values[3] + values.get(4).copied().unwrap_or(0);
    let cpus = lines
        .filter(|line| {
            line.strip_prefix("cpu")
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        })
        .count() as u64;
    Some(Sample {
        total,
        idle,
        cpus: cpus.max(1),
    })
}

pub fn parse_state(state: &str) -> Option<State> {
    let mut fields = state.split_whitespace().map(str::parse::<u64>);
    let total = fields.next()?.ok()?;
    let idle = fields.next()?.ok()?;
    let percent = fields.next()?.ok()?.min(100) as u8;
    Some(State {
        total,
        idle,
        percent,
    })
}

pub fn format_state(state: &State) -> String {
    format!("{} {} {}\n", state.total, state.idle, state.percent)
}

/// Returns the percentage to show and, when the interval was long enough to
/// measure, the state to save. Without a usable previous state (first run,
/// or counters that went backwards after a reboot) it shows 0%.
pub fn percent(previous: Option<State>, current: Sample) -> (u8, Option<State>) {
    let fresh = |percent| State {
        total: current.total,
        idle: current.idle,
        percent,
    };
    let Some(previous) = previous.filter(|p| p.total <= current.total && p.idle <= current.idle)
    else {
        return (0, Some(fresh(0)));
    };
    let elapsed = current.total - previous.total;
    if elapsed < current.cpus * MIN_JIFFIES_PER_CPU {
        return (previous.percent, None);
    }
    let idle = (current.idle - previous.idle).min(elapsed);
    let busy = elapsed - idle;
    let percent = ((busy * 100 + elapsed / 2) / elapsed).min(100) as u8;
    (percent, Some(fresh(percent)))
}

/// Reads the counters, updates the state file, and returns the percentage.
pub fn read(state_path: &Path) -> Option<u8> {
    let current = parse_stat(&fs::read_to_string("/proc/stat").ok()?)?;
    let previous = fs::read_to_string(state_path)
        .ok()
        .and_then(|s| parse_state(&s));
    let (percent, save) = percent(previous, current);
    if let Some(state) = save {
        // Write-then-rename so a concurrent reader never sees a partial file.
        let tmp = state_path.with_extension(format!("tmp.{}", std::process::id()));
        if fs::write(&tmp, format_state(&state)).is_ok() && fs::rename(&tmp, state_path).is_err() {
            let _ = fs::remove_file(&tmp);
        }
    }
    Some(percent)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAT: &str = "cpu  100 10 50 800 40 0 0 0 0 0\n\
                        cpu0 50 5 25 400 20 0 0 0 0 0\n\
                        cpu1 50 5 25 400 20 0 0 0 0 0\n\
                        intr 12345\n\
                        ctxt 678\n";

    #[test]
    fn parses_aggregate_line_and_counts_cpus() {
        assert_eq!(
            parse_stat(STAT),
            Some(Sample {
                total: 1000,
                idle: 840,
                cpus: 2
            })
        );
    }

    #[test]
    fn ignores_guest_columns() {
        let stat = "cpu 1 1 1 1 1 1 1 1 99 99\ncpu0 1 1 1 1 1 1 1 1 99 99\n";
        assert_eq!(parse_stat(stat).unwrap().total, 8);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_stat(""), None);
        assert_eq!(parse_stat("intr 1 2 3\n"), None);
        assert_eq!(parse_stat("cpu a b c d\n"), None);
    }

    #[test]
    fn state_round_trips() {
        let state = State {
            total: 123,
            idle: 45,
            percent: 67,
        };
        assert_eq!(parse_state(&format_state(&state)), Some(state));
        assert_eq!(parse_state("1 2"), None);
    }

    fn sample(total: u64, idle: u64) -> Sample {
        Sample {
            total,
            idle,
            cpus: 2,
        }
    }

    #[test]
    fn first_run_shows_zero_and_saves() {
        let (p, save) = percent(None, sample(1000, 800));
        assert_eq!(p, 0);
        assert_eq!(save.unwrap().total, 1000);
    }

    #[test]
    fn computes_rounded_busy_share() {
        let prev = State {
            total: 1000,
            idle: 800,
            percent: 0,
        };
        // 200 jiffies elapsed, 50 idle: 75% busy.
        let (p, save) = percent(Some(prev), sample(1200, 850));
        assert_eq!(p, 75);
        assert_eq!(save.unwrap().percent, 75);
    }

    #[test]
    fn short_interval_reuses_previous_percentage() {
        let prev = State {
            total: 1000,
            idle: 800,
            percent: 42,
        };
        // 99 jiffies < 2 cpus * 50.
        assert_eq!(percent(Some(prev), sample(1099, 850)), (42, None));
    }

    #[test]
    fn counters_going_backwards_reset() {
        let prev = State {
            total: 5000,
            idle: 4000,
            percent: 42,
        };
        let (p, save) = percent(Some(prev), sample(1000, 800));
        assert_eq!(p, 0);
        assert_eq!(save.unwrap().total, 1000);
    }
}
