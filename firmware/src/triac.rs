use crate::{
    fixpt::{fixpt, Fixpt},
    mains::{Phase, PhaseUpdate},
    mutex::{MainCtx, MutexCell},
    ports::PORTB,
    timer::{
        timer_get_large, LargeTimestamp, RelLargeTimestamp, RelTimestamp, Timestamp, TIMER_TICK_US,
    },
};

/// Triac trigger pulse length.
const PULSE_LEN: RelTimestamp = RelTimestamp::from_micros(64);

/// The last point a trigger can happen.
/// Relative to the halfwave start.
const MAX_TRIG_OFFS: RelLargeTimestamp = RelLargeTimestamp::from_micros(9_850);

fn t_plus_trig_offs(t: LargeTimestamp, ms: Fixpt) -> LargeTimestamp {
    // We must convert `ms` to a corresponding number of ticks.
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
    let trig_offs = RelLargeTimestamp::from_ticks(ticks).min(MAX_TRIG_OFFS);

    // Halfwave start + offset is trigger start.
    t + trig_offs
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TriacState {
    Idle,
    Triggering,
    Triggered,
}

pub struct Triac {
    phi_offs_ms: MutexCell<Fixpt>,
    state: MutexCell<TriacState>,
    trig_time: MutexCell<Timestamp>,
}

impl Triac {
    pub const fn new() -> Self {
        Self {
            phi_offs_ms: MutexCell::new(Fixpt::from_int(0)),
            state: MutexCell::new(TriacState::Idle),
            trig_time: MutexCell::new(Timestamp::new()),
        }
    }

    pub fn set_phi_offs_ms(&self, m: &MainCtx<'_>, ms: Fixpt) {
        self.phi_offs_ms.set(m, ms);
    }

    pub fn shutoff(&self, m: &MainCtx<'_>) {
        self.phi_offs_ms.set(m, Fixpt::from_int(20));
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

        let now = timer_get_large(m);
        let phi_offs_ms = self.phi_offs_ms.get(m);

        match self.state.get(m) {
            TriacState::Idle => {
                let must_trigger = now >= t_plus_trig_offs(phaseref, phi_offs_ms);
                if must_trigger {
                    self.state.set(m, TriacState::Triggering);
                    self.set_trigger(m, true);
                    self.trig_time.set(m, now.into());
                }
            }
            TriacState::Triggering => {
                let now: Timestamp = now.into();
                if now >= self.trig_time.get(m) + PULSE_LEN {
                    self.state.set(m, TriacState::Triggered);
                    self.set_trigger(m, false);
                }
                if phase_update == PhaseUpdate::Changed {
                    self.state.set(m, TriacState::Idle);
                    self.set_trigger(m, false);
                }
            }
            TriacState::Triggered => {
                if phase_update == PhaseUpdate::Changed {
                    self.state.set(m, TriacState::Idle);
                    self.set_trigger(m, false);
                }
                //TODO re-trigger if:
                // - the triac lost trigger (measure voltage) and
                // - it's still earlier than MAX_TRIG_OFFS from beginning.
            }
        }
    }
}

// vim: ts=4 sw=4 expandtab
