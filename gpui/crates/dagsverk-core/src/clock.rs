use chrono::{DateTime, NaiveDate, Utc};

pub trait Clock: Send + Sync {
    fn today(&self) -> NaiveDate;
    fn now_utc(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> NaiveDate {
        Utc::now().date_naive()
    }

    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    now: DateTime<Utc>,
}

impl FixedClock {
    pub const fn new(now: DateTime<Utc>) -> Self {
        Self { now }
    }
}

impl Clock for FixedClock {
    fn today(&self) -> NaiveDate {
        self.now.date_naive()
    }

    fn now_utc(&self) -> DateTime<Utc> {
        self.now
    }
}
