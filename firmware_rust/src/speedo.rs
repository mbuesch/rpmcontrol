use crate::{
    analog::AcCapture,
    fixpt::Fixpt,
    mutex::CriticalSection,
    timer::{timer_get, RelTimestamp, Timestamp, TIMER_TICK_US},
};

/// 2 edge (rising falling) in AC capture.
/// 4 speedometer edges per motor revolution
const SPEEDO_FACT: u32 = 4 * 2;

const OK_THRES: u8 = 4;

pub struct Speedo {
    mot_hz: Fixpt,
    ok_count: u8,
    prev_stamp: Timestamp,
    dur: [u8; 4],
}

impl Speedo {
    pub const fn new() -> Self {
        Self {
            mot_hz: Fixpt::new(0),
            ok_count: 0,
            prev_stamp: Timestamp::new(),
            dur: [0; 4],
        }
    }

    pub fn reset(&mut self) {
        self.ok_count = 0;
    }

    pub fn get_freq_hz(&mut self) -> Option<Fixpt> {
        if self.ok_count < OK_THRES {
            None
        } else {
            let dur: i8 = self.get_dur().into();
            let dur: u8 = dur as _;

            let num = (1_000_000 / (TIMER_TICK_US as u32 * SPEEDO_FACT)) as u16;
            let denom = dur as u16;
            let mot_hz = num / denom;

            Some(Fixpt::new(mot_hz as _))
        }
    }

    pub fn get_dur(&self) -> RelTimestamp {
        let a = self.dur[0] as u16;
        let b = self.dur[1] as u16;
        let c = self.dur[2] as u16;
        let d = self.dur[3] as u16;
        let dur: u8 = ((a + b + c + d) / 4) as _;
        let dur: i8 = dur as _;
        dur.into()
    }

    fn new_duration(&mut self, dur: RelTimestamp) {
        let dur: i8 = dur.into();
        self.dur[0] = self.dur[1];
        self.dur[1] = self.dur[2];
        self.dur[2] = self.dur[3];
        self.dur[3] = dur as _;
        self.ok_count = self.ok_count.saturating_add(1);
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
                self.ok_count = 0;
            }
            self.prev_stamp = ac_stamp;
        }
    }
}

// vim: ts=4 sw=4 expandtab
