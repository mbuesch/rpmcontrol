use crate::{
    analog::ac_capture_get,
    debug::Debug,
    timer::{LargeTimestamp, RelLargeTimestamp, TIMER_TICK_US, timer_get_large},
};
use avr_context::{MainCtx, MainCtxCell};
use avr_int24::I24;
use avr_q::{Q7p8, q7p8};

/// 4 speedometer edges per motor revolution
const SPEEDO_FACT: u32 = 4;

const OK_THRES: u8 = 4;

#[derive(Copy, Clone)]
pub struct MotorSpeed(Q7p8);

impl MotorSpeed {
    const FACT_16HZ: i16 = 16;

    pub const fn as_16hz(&self) -> Q7p8 {
        self.0
    }

    pub fn from_16hz(value: Q7p8) -> Self {
        Self(value)
    }

    pub fn from_period_dur(dur: RelLargeTimestamp) -> Self {
        let dur: i16 = dur.into();
        let dur = dur.min(i16::MAX / (Self::FACT_16HZ * 2)); // avoid mul overflow.
        let dur = dur.max(1); // avoid div by zero.

        // fact 2 to avoid rounding error.
        let num = (1_000_000 / (TIMER_TICK_US as u32 * (SPEEDO_FACT / 2))) as i16;
        let denom = dur * Self::FACT_16HZ * 2;

        Self::from_16hz(q7p8!(num / denom))
    }
}

pub struct Speedo {
    ok_count: MainCtxCell<u8>,
    prev_stamp: MainCtxCell<LargeTimestamp>,
    dur: [MainCtxCell<i16>; 4],
}

impl Speedo {
    pub const fn new() -> Self {
        Self {
            ok_count: MainCtxCell::new(0),
            prev_stamp: MainCtxCell::new(LargeTimestamp::new()),
            dur: [
                MainCtxCell::new(0),
                MainCtxCell::new(0),
                MainCtxCell::new(0),
                MainCtxCell::new(0),
            ],
        }
    }

    fn get_speed(&self, m: &MainCtx<'_>) -> Option<MotorSpeed> {
        if self.ok_count.get(m) >= OK_THRES {
            Some(MotorSpeed::from_period_dur(self.get_dur(m)))
        } else {
            None
        }
    }

    fn get_dur(&self, m: &MainCtx<'_>) -> RelLargeTimestamp {
        let a = I24::from_i16(self.dur[0].get(m));
        let b = I24::from_i16(self.dur[1].get(m));
        let c = I24::from_i16(self.dur[2].get(m));
        let d = I24::from_i16(self.dur[3].get(m));
        let dur = ((a + b + c + d) >> 2).to_i16();
        dur.into()
    }

    fn new_duration(&self, m: &MainCtx<'_>, dur: RelLargeTimestamp) {
        let dur: i16 = dur.into();
        self.dur[0].set(m, self.dur[1].get(m));
        self.dur[1].set(m, self.dur[2].get(m));
        self.dur[2].set(m, self.dur[3].get(m));
        self.dur[3].set(m, dur);
        self.ok_count.set(m, self.ok_count.get(m).saturating_add(1));
    }

    pub fn run(&self, m: &MainCtx<'_>) -> Option<MotorSpeed> {
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

        self.get_speed(m)
    }
}

// vim: ts=4 sw=4 expandtab
