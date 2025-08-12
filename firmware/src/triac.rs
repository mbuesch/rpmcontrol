use crate::{
    fixpt::{Fixpt, fixpt},
    mains::{Phase, PhaseUpdate},
    mutex::{MainCtx, MutexCell},
    ports::PORTB,
    timer::{
        LargeTimestamp, RelLargeTimestamp, RelTimestamp, TIMER_TICK_US, Timestamp, timer_get_large,
    },
};

/// Triac trigger pulse length set-duration or clear-duration.
const HALF_PULSE_LEN: RelTimestamp = RelTimestamp::from_micros(64);

/// Mains sine wave period (50 Hz).
const MAINS_PERIOD: RelLargeTimestamp = RelLargeTimestamp::from_micros(20_000);

/// Mains sine wave half-wave length.
const HALFWAVE_LEN: RelLargeTimestamp = MAINS_PERIOD.div(2);

/// The last point a trigger can happen.
/// Relative to the halfwave start.
const MAX_TRIG_OFFS: RelLargeTimestamp = RelLargeTimestamp::from_micros(9_850);

fn ms_to_reltimestamp(ms: Fixpt) -> RelLargeTimestamp {
    // We must convert `ms` milliseconds to a corresponding number of ticks.
    //
    // Basically, we want to do:
    //  let ticks = (ms * 1000) / TIMER_TICK_US;
    //
    // But we must avoid overflows and minimize rounding errors.
    //
    // assumptions:
    //  1000 / TIMER_TICK_US = 62.5
    //  We use a bias of 32 = 1 << 5.
    //
    // Therefore, we calculate:
    //
    //         ms * 62.5 * 32
    // ticks = --------------
    //              32
    //
    // But we split it up into a Fixpt calculation and the final bias shift.
    //
    // Fixpt calculation:
    //
    //         ms * 62.5
    // ticks = ---------
    //              32

    // The microseconds per tick value is embedded in the constants below.
    // See comment above.
    assert_eq!(TIMER_TICK_US, 16);

    // First part: Fixpt multiplication.
    let fac = fixpt!(125 / 64); // 62.5 / 32
    let scaled = ms * fac;

    // Second part: Bias multiplication.
    // Get the raw fixpt value and shift by 5.
    let ticks = scaled.to_q() >> (Fixpt::SHIFT - 5);

    // Limit to last possible moment.
    RelLargeTimestamp::from_ticks(ticks).min(MAX_TRIG_OFFS)
}

fn calc_trig_count(trig_offs: RelLargeTimestamp) -> u8 {
    let retrig_thres = HALFWAVE_LEN.div(4);

    let retrig_dur = if trig_offs < retrig_thres {
        retrig_thres - trig_offs
    } else if trig_offs > HALFWAVE_LEN - retrig_thres {
        HALFWAVE_LEN - trig_offs
    } else {
        RelLargeTimestamp::from_micros(0)
    };

    let retrig_dur: i16 = retrig_dur.into();
    let half_pulse_len: i8 = HALF_PULSE_LEN.into();
    let pulse_len = half_pulse_len as i16 * 2;

    ((retrig_dur / pulse_len) as u8).max(1)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TriacState {
    WaitForTrig,
    TriggeringSet,
    TriggeringClr,
    Triggered,
}

pub struct Triac {
    phi_offs: MutexCell<RelLargeTimestamp>,
    state: MutexCell<TriacState>,
    pulse_end_time: MutexCell<Timestamp>,
    trig_count: MutexCell<u8>,
}

impl Triac {
    pub const fn new() -> Self {
        Self {
            phi_offs: MutexCell::new(RelLargeTimestamp::new()),
            state: MutexCell::new(TriacState::WaitForTrig),
            pulse_end_time: MutexCell::new(Timestamp::new()),
            trig_count: MutexCell::new(0),
        }
    }

    pub fn set_phi_offs_ms(&self, m: &MainCtx<'_>, ms: Fixpt) {
        self.phi_offs.set(m, ms_to_reltimestamp(ms));
    }

    pub fn shutoff(&self, m: &MainCtx<'_>) {
        self.phi_offs.set(m, MAINS_PERIOD);
    }

    fn set_trigger(&self, _m: &MainCtx<'_>, trigger: bool) {
        let trigger = !trigger; // negative logic at triac gate.
        PORTB.set(3, trigger);
    }

    pub fn run(
        &self,
        m: &MainCtx<'_>,
        phase_update: PhaseUpdate,
        phase: Phase,
        phaseref: LargeTimestamp,
    ) {
        if phase == Phase::Notsync {
            self.set_trigger(m, false);
            return;
        }

        if phase_update == PhaseUpdate::Changed {
            self.set_trigger(m, false);
            self.state.set(m, TriacState::WaitForTrig);
        }

        let now = timer_get_large();

        match self.state.get(m) {
            TriacState::WaitForTrig => {
                let trig_offs = self.phi_offs.get(m);
                let trig_time = phaseref + trig_offs;

                if now >= trig_time {
                    self.set_trigger(m, true);
                    self.state.set(m, TriacState::TriggeringSet);

                    let now: Timestamp = now.into();
                    self.pulse_end_time.set(m, now + HALF_PULSE_LEN);
                    self.trig_count.set(m, calc_trig_count(trig_offs));
                }
            }
            TriacState::TriggeringSet => {
                let now: Timestamp = now.into();
                if now >= self.pulse_end_time.get(m) {
                    self.set_trigger(m, false);

                    let trig_count = self.trig_count.get(m) - 1;
                    self.trig_count.set(m, trig_count);

                    if trig_count == 0 {
                        self.state.set(m, TriacState::Triggered);
                    } else {
                        self.pulse_end_time
                            .set(m, self.pulse_end_time.get(m) + HALF_PULSE_LEN);
                        self.state.set(m, TriacState::TriggeringClr);
                    }
                }
            }
            TriacState::TriggeringClr => {
                let now: Timestamp = now.into();
                if now >= self.pulse_end_time.get(m) {
                    self.set_trigger(m, true);

                    self.pulse_end_time
                        .set(m, self.pulse_end_time.get(m) + HALF_PULSE_LEN);
                    self.state.set(m, TriacState::TriggeringSet);
                }
            }
            TriacState::Triggered => {
                // Waiting for PhaseUpdate::Changed (handled above).
            }
        }
    }
}

// vim: ts=4 sw=4 expandtab
