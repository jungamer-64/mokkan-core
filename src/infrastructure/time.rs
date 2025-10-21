use crate::application::ports::time::Clock;
use chrono::{DateTime, Utc};

#[derive(Default, Clone)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
