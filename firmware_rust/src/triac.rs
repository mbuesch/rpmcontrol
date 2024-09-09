use crate::{
    fixpt::{fixpt, Fixpt},
    mains::{Phase, PhaseUpdate},
    mutex::{CriticalSection, MutexCell},
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

    pub fn set_phi_offs_ms(&self, cs: CriticalSection<'_>, ms: Fixpt) {
        self.phi_offs_ms.set(cs, ms);
    }

    fn set_trigger(&self, _cs: CriticalSection<'_>, sp: &SysPeriph, trigger: bool) {
        let trigger = !trigger; // negative logic at triac gate.
        sp.PORTB.portb.modify(|_, w| w.pb3().bit(trigger));
    }

    pub fn run(
        &self,
        cs: CriticalSection<'_>,
        sp: &SysPeriph,
        phase_update: PhaseUpdate,
        phase: &Phase,
    ) {
        let now = timer_get_large(cs);
        let phi_offs_ms = self.phi_offs_ms.get(cs);

        let phaseref = match phase {
            Phase::Notsync => {
                return;
            }
            Phase::PosHalfwave(phaseref) => phaseref,
            Phase::NegHalfwave(phaseref) => phaseref,
        };

        match self.state.get(cs) {
            TriacState::Idle => {
                let must_trigger = now >= time_plus_ms(*phaseref, phi_offs_ms);
                if must_trigger {
                    self.set_trigger(cs, sp, true);
                    self.state.set(cs, TriacState::Triggering);
                    self.trig_time.set(cs, now.into());
                } else {
                    self.set_trigger(cs, sp, false);
                }
            }
            TriacState::Triggering => {
                if self.trig_time.get(cs) >= now.into() {
                    self.set_trigger(cs, sp, false);
                    self.state.set(cs, TriacState::Triggered);
                }
                if phase_update == PhaseUpdate::Changed {
                    self.state.set(cs, TriacState::Idle);
                }
            }
            TriacState::Triggered => {
                if phase_update == PhaseUpdate::Changed {
                    self.state.set(cs, TriacState::Idle);
                }
            }
        }
    }
}

// vim: ts=4 sw=4 expandtab
