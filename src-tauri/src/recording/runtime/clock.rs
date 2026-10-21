use std::time::Duration;

pub(super) struct ActiveClock {
    started_at: std::time::Instant,
    accumulated_pause: Duration,
    paused_at: Option<std::time::Instant>,
}

impl ActiveClock {
    pub(super) fn start() -> Self {
        Self {
            started_at: std::time::Instant::now(),
            accumulated_pause: Duration::ZERO,
            paused_at: None,
        }
    }

    pub(super) fn is_paused(&self) -> bool {
        self.paused_at.is_some()
    }

    pub(super) fn pause(&mut self) -> u64 {
        self.paused_at.get_or_insert_with(std::time::Instant::now);
        self.elapsed_ms()
    }

    pub(super) fn resume(&mut self) {
        if let Some(paused_at) = self.paused_at.take() {
            self.accumulated_pause = self.accumulated_pause.saturating_add(paused_at.elapsed());
        }
    }

    pub(super) fn elapsed(&self) -> Duration {
        let total = self.started_at.elapsed();
        let pause = self.accumulated_pause.saturating_add(
            self.paused_at
                .map_or(Duration::ZERO, |paused_at| paused_at.elapsed()),
        );
        total.saturating_sub(pause)
    }

    pub(super) fn elapsed_ms(&self) -> u64 {
        self.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }

    pub(super) fn paused_duration_100ns(&self) -> i64 {
        let duration = self.accumulated_pause.saturating_add(
            self.paused_at
                .map_or(Duration::ZERO, |paused_at| paused_at.elapsed()),
        );
        (duration.as_nanos() / 100).min(i64::MAX as u128) as i64
    }
}

#[cfg(all(test, windows))]
mod tests {
    use std::{thread, time::Duration};

    use super::ActiveClock;

    #[test]
    fn paused_time_does_not_advance_the_recording_clock() {
        let mut clock = ActiveClock::start();
        thread::sleep(Duration::from_millis(25));
        let elapsed_at_pause = clock.pause();
        thread::sleep(Duration::from_millis(35));
        assert!(clock.elapsed_ms().abs_diff(elapsed_at_pause) <= 2);
        assert!(clock.paused_duration_100ns() >= 300_000);

        clock.resume();
        thread::sleep(Duration::from_millis(25));
        let resumed_elapsed = clock.elapsed_ms();
        assert!(resumed_elapsed >= elapsed_at_pause);
        assert!(resumed_elapsed - elapsed_at_pause < 100);
    }
}
