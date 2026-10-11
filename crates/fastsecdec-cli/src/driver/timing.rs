//! CLI-owned scheduling deadline. Native tasks and statistics stay unchanged.
use crate::CliResult;
use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct TimedStopReport {
    pub requested_seconds: f64,
    pub deadline_observed_seconds: Option<f64>,
    pub drain_seconds: f64,
    /// Reserved work still awaiting complete caller admission/acknowledgement.
    pub in_flight_at_deadline: usize,
    pub counts_complete: bool,
}
impl TimedStopReport {
    pub fn deadline_reached(&self) -> bool {
        self.deadline_observed_seconds.is_some()
    }
}

pub(super) struct TimedStop {
    requested: Option<f64>,
    limit: Option<Duration>,
    deadline: Option<Duration>,
    drain_started: Option<Duration>,
    drained: Option<Duration>,
    in_flight_at_deadline: usize,
}
impl TimedStop {
    pub fn new(seconds: Option<f64>) -> CliResult<Self> {
        let limit = seconds
            .map(|s| -> CliResult<Duration> {
                if !s.is_finite() || s <= 0.0 {
                    return Err("max-seconds must be finite and positive".into());
                }
                let d = Duration::try_from_secs_f64(s)?;
                if d.is_zero() {
                    return Err("max-seconds is below the clock duration resolution".into());
                }
                Ok(d)
            })
            .transpose()?;
        Ok(Self {
            requested: seconds,
            limit,
            deadline: None,
            drain_started: None,
            drained: None,
            in_flight_at_deadline: 0,
        })
    }
    pub fn enabled(&self) -> bool {
        self.limit.is_some()
    }
    pub fn stopping(&self) -> bool {
        self.drain_started.is_some()
    }
    /// Use the driver's existing elapsed clock. A deadline is latched once,
    /// including when an earlier target stop is still draining.
    pub fn observe(&mut self, elapsed: Duration, in_flight: usize) -> bool {
        if self.limit.is_some_and(|limit| elapsed >= limit) && self.deadline.is_none() {
            self.deadline = Some(elapsed);
            self.in_flight_at_deadline = in_flight;
            self.begin_drain(elapsed);
        }
        self.stopping()
    }
    pub fn begin_drain(&mut self, elapsed: Duration) {
        if self.enabled() {
            self.drain_started.get_or_insert(elapsed);
        }
    }
    pub fn drained(&mut self, elapsed: Duration) {
        if self.stopping() {
            self.drained.get_or_insert(elapsed);
        }
    }
    pub fn report(&self, elapsed: Duration, complete: bool) -> Option<TimedStopReport> {
        self.requested.map(|requested_seconds| TimedStopReport {
            requested_seconds,
            deadline_observed_seconds: self.deadline.map(|d| d.as_secs_f64()),
            drain_seconds: self.drain_started.map_or(0.0, |start| {
                self.drained
                    .unwrap_or(elapsed)
                    .saturating_sub(start)
                    .as_secs_f64()
            }),
            in_flight_at_deadline: self.in_flight_at_deadline,
            counts_complete: complete && (!self.stopping() || self.drained.is_some()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seconds(n: u64) -> Duration {
        Duration::from_secs(n)
    }
    #[test]
    fn deadline_latches_and_counts_require_successful_drain() {
        let mut limit = TimedStop::new(Some(10.0)).unwrap();
        assert!(!limit.observe(seconds(9), 3));
        assert!(limit.observe(seconds(10), 3));
        assert!(limit.observe(seconds(12), 1));
        assert!(!limit.report(seconds(12), true).unwrap().counts_complete);
        limit.drained(seconds(14));
        let report = limit.report(seconds(15), true).unwrap();
        assert!(report.deadline_reached());
        assert_eq!(report.deadline_observed_seconds, Some(10.0));
        assert_eq!(report.in_flight_at_deadline, 3);
        assert_eq!(report.drain_seconds, 4.0);
        assert!(report.counts_complete);
        assert!(!limit.report(seconds(15), false).unwrap().counts_complete);
    }
    #[test]
    fn optional_limit_preserves_early_and_disabled_stops() {
        let mut absent = TimedStop::new(None).unwrap();
        absent.begin_drain(seconds(2));
        assert!(!absent.observe(seconds(1000), 1));
        assert!(absent.report(seconds(1000), true).is_none());
        let mut early = TimedStop::new(Some(10.0)).unwrap();
        early.begin_drain(seconds(2));
        early.drained(seconds(3));
        let report = early.report(seconds(4), true).unwrap();
        assert!(!report.deadline_reached());
        assert_eq!(report.drain_seconds, 1.0);
        assert!(report.counts_complete);
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MAX] {
            assert!(TimedStop::new(Some(value)).is_err());
        }
    }
}
