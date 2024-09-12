use crate::{
    fixpt::{fixpt, Fixpt},
    mains::{Phase, PhaseUpdate},
    mutex::{MainCtx, MutexCell},
    system::SysPeriph,
    timer::{timer_get_large, LargeTimestamp, RelLargeTimestamp, Timestamp, TIMER_TICK_US},
};

fn time_plus_ms(t: LargeTimestamp, ms: Fixpt) -> LargeTimestamp {
    const TICK_US: i16 = TIMER_TICK_US as i16;
    let ticks = (ms * fixpt!(1000 / TICK_US)).to_int();
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
            phi_offs_ms: MutexCell::new(Fixpt::from_int(20)),
            state: MutexCell::new(TriacState::Idle),
            trig_time: MutexCell::new(Timestamp::new()),
        }
    }

    pub fn set_phi_offs_ms(&self, m: &MainCtx<'_>, ms: Fixpt) {
        self.phi_offs_ms.set(m, ms);
    }

    fn set_trigger(&self, _m: &MainCtx<'_>, sp: &SysPeriph, trigger: bool) {
        let trigger = !trigger; // negative logic at triac gate.
        sp.PORTB.portb.modify(|_, w| w.pb3().bit(trigger));
    }

    pub fn run(&self, m: &MainCtx<'_>, sp: &SysPeriph, phase_update: PhaseUpdate, phase: &Phase) {
        let now = timer_get_large(m);
        let phi_offs_ms = self.phi_offs_ms.get(m);

        let phaseref = match phase {
            Phase::Notsync => {
                return;
            }
            Phase::PosHalfwave(phaseref) => phaseref,
            Phase::NegHalfwave(phaseref) => phaseref,
        };

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
                if self.trig_time.get(m) >= now.into() {
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
