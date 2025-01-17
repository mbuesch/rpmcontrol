use crate::{
    mutex::{MainCtx, MutexCell},
    ports::PORTA,
    timer::{timer_get_large, LargeTimestamp, RelLargeTimestamp},
};

const HALFWAVE_DUR: RelLargeTimestamp = RelLargeTimestamp::from_millis(10);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Notsync,
    PosHalfwave,
    NegHalfwave,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PhaseUpdate {
    NotChanged,
    Changed,
}

pub struct Mains {
    prev_vsense: MutexCell<bool>,
    phase: MutexCell<Phase>,
    phaseref: MutexCell<LargeTimestamp>,
}

impl Mains {
    pub const fn new() -> Self {
        Self {
            prev_vsense: MutexCell::new(false),
            phase: MutexCell::new(Phase::Notsync),
            phaseref: MutexCell::new(LargeTimestamp::new()),
        }
    }

    fn read_vsense(&self, m: &MainCtx<'_>) -> bool {
        PORTA.get_bit(&m.to_any(), 1)
    }

    pub fn run(&self, m: &MainCtx<'_>) -> PhaseUpdate {
        let vsense = self.read_vsense(m);
        let now = timer_get_large(m);
        let mut ret = PhaseUpdate::NotChanged;
        match self.phase.get(m) {
            Phase::Notsync | Phase::NegHalfwave => {
                if !self.prev_vsense.get(m) && vsense {
                    self.phaseref.set(m, now);
                    self.phase.set(m, Phase::PosHalfwave);
                    ret = PhaseUpdate::Changed;
                }
            }
            Phase::PosHalfwave => {
                let nextref = self.phaseref.get(m) + HALFWAVE_DUR;
                if now >= nextref {
                    self.phaseref.set(m, nextref);
                    self.phase.set(m, Phase::NegHalfwave);
                    ret = PhaseUpdate::Changed;
                }
            }
        }
        self.prev_vsense.set(m, vsense);
        ret
    }

    pub fn get_phase(&self, m: &MainCtx<'_>) -> Phase {
        self.phase.get(m)
    }

    pub fn get_phaseref(&self, m: &MainCtx<'_>) -> LargeTimestamp {
        self.phaseref.get(m)
    }
}

// vim: ts=4 sw=4 expandtab
