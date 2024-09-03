use crate::{
    analog::AcCapture,
    fixpt::Fixpt,
    mutex::{unwrap_option, unwrap_result, CriticalSection},
    timer::{timer_get, Timestamp, TIMER_TICK_US},
};
use core::num::NonZeroU16;

/// 4 speedometer edges per motor revolution
const SPEEDO_FACT: u32 = 4;

pub struct Speedo {
    mot_hz: Option<NonZeroU16>,
    prev_stamp: Option<Timestamp>,
}

impl Speedo {
    pub const fn new() -> Self {
        Self {
            mot_hz: None,
            prev_stamp: None,
        }
    }

    pub fn reset(&mut self) {
        self.mot_hz = None;
        self.prev_stamp = None;
    }

    pub fn get_freq_hz(&mut self) -> Option<Fixpt> {
        self.mot_hz.map(|hz| {
            let hz: u16 = hz.into();
            Fixpt::new(unwrap_result(hz.try_into()))
        })
    }

    fn new_duration(&mut self, dur: u8) {
        let num = (1_000_000 / (TIMER_TICK_US as u32 * SPEEDO_FACT)) as u16;
        let denom = dur as u16;
        let mot_hz = num / denom;
        let mot_hz = unwrap_option(NonZeroU16::new(mot_hz));
        self.mot_hz = Some(mot_hz);
    }

    pub fn update(&mut self, cs: CriticalSection<'_>, ac: &AcCapture) {
        let now = timer_get(cs);
        if let Some(prev_stamp) = self.prev_stamp {
            if now < prev_stamp {
                // prev_stamp wrapped. Drop it.
                self.prev_stamp = None;
            }
        }
        if ac.is_new() {
            let ac_stamp = ac.stamp();
            if let Some(prev_stamp) = self.prev_stamp {
                if ac_stamp >= prev_stamp {
                    let dur = ac_stamp - prev_stamp;
                    self.new_duration(dur);
                } else {
                    // prev_stamp wrapped.
                }
            }
            self.prev_stamp = Some(ac_stamp);
        }
    }
}

// vim: ts=4 sw=4 expandtab
