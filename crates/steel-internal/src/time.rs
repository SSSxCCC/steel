use shipyard::{Unique, UniqueViewMut};
use std::time::Instant;

/// Time unique manages current running times of game,
/// you can use Time::now() to get the time at the start of this frame since game start,
/// and use Time::delta() to get the time spent in last frame.
#[derive(Unique)]
pub struct Time {
    now: f32,
    delta: f32,
    start_instant: Instant,
    last_instant: Instant,
}

impl Time {
    /// Create a new Time.
    pub fn new() -> Time {
        let now_instant = Instant::now();
        Time {
            now: 0.0,
            delta: 0.0,
            start_instant: now_instant,
            last_instant: now_instant,
        }
    }

    /// Get the number of seconds at the start of this frame since game start.
    pub fn now(&self) -> f32 {
        self.now
    }

    /// Get the number of seconds spent in last frame.
    pub fn delta(&self) -> f32 {
        self.delta
    }

    /// Reset time so that now is the game start time.
    pub fn reset(&mut self) {
        let now_instant = Instant::now();
        self.now = 0.0;
        self.delta = 0.0;
        self.start_instant = now_instant;
        self.last_instant = now_instant;
    }
}

/// Update Time::now and Time::delta.
pub fn time_maintain_system(mut time: UniqueViewMut<Time>) {
    let now_instant = Instant::now();
    time.now = (now_instant - time.start_instant).as_secs_f32();
    time.delta = (now_instant - time.last_instant).as_secs_f32();
    time.last_instant = now_instant;
}

/// The execution order of [time_maintain_system].
pub const TIME_MAINTAIN_SYSTEM_ORDER: i32 = -6000;
