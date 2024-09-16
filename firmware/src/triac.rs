use crate::{
    fixpt::{fixpt, Fixpt},
    mains::{Phase, PhaseUpdate},
    mutex::{MainCtx, MutexCell},
    system::SysPeriph,
    timer::{timer_get_large, LargeTimestamp, RelLargeTimestamp, Timestamp, RelTimestamp, TIMER_TICK_US},
};

const PULSE_LEN: RelTimestamp = RelTimestamp::from_micros(64);

fn time_plus_ms(t: LargeTimestamp, ms: Fixpt) -> LargeTimestamp {
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

    assert_eq!(TIMER_TICK_US, 16);

    // First part: Fixpt multiplication.
    let fac = fixpt!(125 / 64); // 62.5 / 32
    let scaled = ms * fac;

    // Second part: Bias multiplication.
    // Get the raw fixpt value and shift by 5.
    let ticks = scaled.to_q() >> (Fixpt::SHIFT - 5);

    t + RelLargeTimestamp::from_ticks(ticks)
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

    fn set_trigger(&self, _m: &MainCtx<'_>, sp: &SysPeriph, trigger: bool) {
        let trigger = !trigger; // negative logic at triac gate.
        sp.PORTB.portb.modify(|_, w| w.pb3().bit(trigger));
    }

    pub fn run(&self, m: &MainCtx<'_>, sp: &SysPeriph, phase_update: PhaseUpdate, phase: &Phase) {
        let phaseref = match phase {
            Phase::Notsync => {
                self.set_trigger(m, sp, false);
                return;
            }
            Phase::PosHalfwave(phaseref) => phaseref,
            Phase::NegHalfwave(phaseref) => phaseref,
        };

        let now = timer_get_large(m);
        let phi_offs_ms = self.phi_offs_ms.get(m);

        match self.state.get(m) {
            TriacState::Idle => {
                let must_trigger = now >= time_plus_ms(*phaseref, phi_offs_ms);
                if must_trigger {
                    self.set_trigger(m, sp, true);
                    self.state.set(m, TriacState::Triggering);
                    self.trig_time.set(m, now.into());
                } else {
                    self.set_trigger(m, sp, false);
                }
            }
            TriacState::Triggering => {
                let now: Timestamp = now.into();
                if now >= self.trig_time.get(m) + PULSE_LEN {
                    self.set_trigger(m, sp, false);
                    self.state.set(m, TriacState::Triggered);
                }
                if phase_update == PhaseUpdate::Changed {
                    self.state.set(m, TriacState::Idle);
                }
            }
            TriacState::Triggered => {
                if phase_update == PhaseUpdate::Changed {
                    self.state.set(m, TriacState::Idle);
                }
            }
        }
    }
}

// vim: ts=4 sw=4 expandtab
