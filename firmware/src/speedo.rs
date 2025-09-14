use crate::{
    analog::ac_capture_get,
    debug::Debug,
    filter::Filter,
    fixpt::{Fixpt, fixpt},
    mutex::{MainCtx, MutexCell},
    system::SysPeriph,
    timer::{LargeTimestamp, RelLargeTimestamp, TIMER_TICK_US, timer_get_large},
};

/// 4 speedometer edges per motor revolution
const SPEEDO_FACT: u32 = 4;

const OK_THRES: u8 = 4;

const FILTER_DIV: Fixpt = fixpt!(3 / 1);

#[derive(Copy, Clone)]
pub struct MotorSpeed(Fixpt);

impl MotorSpeed {
    const FACT_16HZ: i16 = 16;

    pub const fn zero() -> Self {
        Self(Fixpt::from_int(0))
    }

    pub const fn as_16hz(&self) -> Fixpt {
        self.0
    }

    pub fn from_16hz(value: Fixpt) -> Self {
        Self(value)
    }

    pub fn from_period_dur(dur: RelLargeTimestamp) -> Self {
        let dur: i16 = dur.into();
        let dur = dur.min(i16::MAX / (Self::FACT_16HZ * 2)); // avoid mul overflow.

        // fact 2 to avoid rounding error.
        let num = (1_000_000 / (TIMER_TICK_US as u32 * (SPEEDO_FACT / 2))) as u16;
        let denom = dur * Self::FACT_16HZ * 2;

        Self::from_16hz(Fixpt::from_fraction(num as i16, denom))
    }
}

pub struct Speedo {
    ok_count: MutexCell<u8>,
    prev_stamp: MutexCell<LargeTimestamp>,
    dur: [MutexCell<i16>; 4],
    filter: Filter,
    speed_filtered: MutexCell<MotorSpeed>,
}

impl Speedo {
    pub const fn new() -> Self {
        Self {
            ok_count: MutexCell::new(0),
            prev_stamp: MutexCell::new(LargeTimestamp::new()),
            dur: [
                MutexCell::new(0),
                MutexCell::new(0),
                MutexCell::new(0),
                MutexCell::new(0),
            ],
            filter: Filter::new(),
            speed_filtered: MutexCell::new(MotorSpeed::zero()),
        }
    }

    pub fn get_speed(&self, m: &MainCtx<'_>) -> Option<MotorSpeed> {
        if self.ok_count.get(m) >= OK_THRES {
            Some(self.speed_filtered.get(m))
        } else {
            None
        }
    }

    fn get_dur(&self, m: &MainCtx<'_>) -> RelLargeTimestamp {
        let a = self.dur[0].get(m) as i32;
        let b = self.dur[1].get(m) as i32;
        let c = self.dur[2].get(m) as i32;
        let d = self.dur[3].get(m) as i32;
        let dur: i16 = ((a + b + c + d) / 4) as _;
        dur.into()
    }

    fn new_duration(&self, m: &MainCtx<'_>, dur: RelLargeTimestamp) {
        let dur: i16 = dur.into();
        self.dur[0].set(m, self.dur[1].get(m));
        self.dur[1].set(m, self.dur[2].get(m));
        self.dur[2].set(m, self.dur[3].get(m));
        self.dur[3].set(m, dur);
        self.ok_count.set(m, self.ok_count.get(m).saturating_add(1));
        self.update_filtered(m);
    }

    fn update_filtered(&self, m: &MainCtx<'_>) {
        let speed = MotorSpeed::from_period_dur(self.get_dur(m));
        let speed = self.filter.run(m, speed.as_16hz(), FILTER_DIV);
        self.speed_filtered.set(m, MotorSpeed::from_16hz(speed));
    }

    pub fn update(&self, m: &MainCtx<'_>, _sp: &SysPeriph) {
        let now = timer_get_large();
        let prev_stamp = self.prev_stamp.get(m);
        if now < prev_stamp {
            // prev_stamp wrapped. Drop it.
            self.ok_count.set(m, 0);
        }

        while let Some(ac) = ac_capture_get() {
            if ac >= prev_stamp {
                let dur = ac - prev_stamp;
                self.new_duration(m, dur);
            } else {
                // prev_stamp wrapped.
                self.ok_count.set(m, 0);
            }
            self.prev_stamp.set(m, ac);
        }

        Debug::SpeedoStatus.log_u16(self.ok_count.get(m) as u16);
    }
}

// vim: ts=4 sw=4 expandtab
