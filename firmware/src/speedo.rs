use crate::{
    analog::AcCapture,
    fixpt::Fixpt,
    mutex::{MainCtx, MutexCell},
    system::SysPeriph,
    timer::{timer_get, RelTimestamp, Timestamp, TIMER_TICK_US},
};

/// 2 edge (rising falling) in AC capture.
/// 4 speedometer edges per motor revolution
const SPEEDO_FACT: u32 = 4 * 2;

const OK_THRES: u8 = 4;

#[derive(Copy, Clone)]
pub struct MotorSpeed(Fixpt);

impl MotorSpeed {
    const FACT_16HZ: u16 = 16;

    pub const fn zero() -> Self {
        Self(Fixpt::from_int(0))
    }

    fn from_period_dur(dur: RelTimestamp) -> Self {
        let dur: i8 = dur.into();
        let dur: u8 = dur as _;

        // fact 2 to avoid rounding error.
        let num = (1_000_000 / (TIMER_TICK_US as u32 * (SPEEDO_FACT / 2))) as u16;
        let denom = dur as u16 * Self::FACT_16HZ * 2;

        Self(Fixpt::from_fraction(num as i16, denom as i16))
    }

    pub const fn as_16hz(&self) -> Fixpt {
        self.0
    }
}

pub struct Speedo {
    ok_count: MutexCell<u8>,
    prev_stamp: MutexCell<Timestamp>,
    dur: [MutexCell<u8>; 4],
}

impl Speedo {
    pub const fn new() -> Self {
        Self {
            ok_count: MutexCell::new(0),
            prev_stamp: MutexCell::new(Timestamp::new()),
            dur: [
                MutexCell::new(0),
                MutexCell::new(0),
                MutexCell::new(0),
                MutexCell::new(0),
            ],
        }
    }

    pub fn get_speed(&self, m: &MainCtx<'_>) -> Option<MotorSpeed> {
        if self.ok_count.get(m) >= OK_THRES {
            Some(MotorSpeed::from_period_dur(self.get_dur(m)))
        } else {
            None
        }
    }

    pub fn get_dur(&self, m: &MainCtx<'_>) -> RelTimestamp {
        let a = self.dur[0].get(m) as u16;
        let b = self.dur[1].get(m) as u16;
        let c = self.dur[2].get(m) as u16;
        let d = self.dur[3].get(m) as u16;
        let dur: u8 = ((a + b + c + d) / 4) as _;
        let dur: i8 = dur as _;
        dur.into()
    }

    fn new_duration(&self, m: &MainCtx<'_>, dur: RelTimestamp) {
        let dur: i8 = dur.into();
        self.dur[0].set(m, self.dur[1].get(m));
        self.dur[1].set(m, self.dur[2].get(m));
        self.dur[2].set(m, self.dur[3].get(m));
        self.dur[3].set(m, dur as _);
        self.ok_count.set(m, self.ok_count.get(m).saturating_add(1));
    }

    pub fn update(&self, m: &MainCtx<'_>, _sp: &SysPeriph, ac: &AcCapture) {
        let now = timer_get(&m.to_any());
        let prev_stamp = self.prev_stamp.get(m);
        if now < prev_stamp {
            // prev_stamp wrapped. Drop it.
            self.ok_count.set(m, 0);
        }
        if ac.is_new() && ac.is_rising() {
            let ac_stamp = ac.stamp();
            if ac_stamp >= prev_stamp {
                let dur = ac_stamp - prev_stamp;
                self.new_duration(m, dur);
            } else {
                // prev_stamp wrapped.
                self.ok_count.set(m, 0);
            }
            self.prev_stamp.set(m, ac_stamp);
        }
    }
}

// vim: ts=4 sw=4 expandtab
