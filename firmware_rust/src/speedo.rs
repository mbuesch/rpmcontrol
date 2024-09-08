use crate::{
    analog::AcCapture,
    fixpt::Fixpt,
    mutex::CriticalSection,
    timer::{timer_get, Timestamp},
};

/// 2 edge (rising falling) in AC capture.
/// 4 speedometer edges per motor revolution
const SPEEDO_FACT: u32 = 4 * 2;

pub struct Speedo {
    mot_hz: Fixpt,
    ok_count: u8,
    prev_stamp: Timestamp,
    prev_dur: u8, // small mov avg
}

impl Speedo {
    pub const fn new() -> Self {
        Self {
            mot_hz: Fixpt::new(0),
            ok_count: 0,
            prev_stamp: Timestamp::new(),
            prev_dur: 0,
        }
    }

    pub fn reset(&mut self) {
        self.ok_count = 0;
    }

    pub fn get_freq_hz(&mut self) -> Option<Fixpt> {
        None
        /*
        self.mot_hz.map(|hz| {
            let hz: u16 = hz.into();
            Fixpt::new(unwrap_result(hz.try_into()))
        })
        */
    }

    pub fn get_dur(&self) -> u8 {
        self.prev_dur
    }

    fn new_duration(&mut self, dur: u8) {
        self.prev_dur = dur;
        /*
        let num = (1_000_000 / (TIMER_TICK_US as u32 * SPEEDO_FACT)) as u16;
        let denom = dur as u16;
        let mot_hz = num / denom;
        let mot_hz = unwrap_option(NonZeroU16::new(mot_hz));
        self.mot_hz = Some(mot_hz);
        */
    }

    pub fn update(&mut self, cs: CriticalSection<'_>, ac: &AcCapture) {
        let now = timer_get(cs);
        if now < self.prev_stamp {
            // prev_stamp wrapped. Drop it.
            self.ok_count = 0;
        }
        if ac.is_new() {
            let ac_stamp = ac.stamp();
            if ac_stamp >= self.prev_stamp {
                let dur = ac_stamp - self.prev_stamp;
                self.new_duration(dur);
            } else {
                // prev_stamp wrapped.
            }
            self.prev_stamp = ac_stamp;
        }
    }
}

// vim: ts=4 sw=4 expandtab
