use crate::{
    mutex::MainCtx,
    system::SysPeriph,
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
    prev_vsense: bool,
    phase: Phase,
    phaseref: LargeTimestamp,
}

impl Mains {
    pub const fn new() -> Self {
        Self {
            prev_vsense: false,
            phase: Phase::Notsync,
            phaseref: LargeTimestamp::new(),
        }
    }

    fn read_vsense(&self, _m: &MainCtx<'_>, sp: &SysPeriph) -> bool {
        sp.PORTA.pina.read().pa1().bit()
    }

    pub fn run(&mut self, m: &MainCtx<'_>, sp: &SysPeriph) -> PhaseUpdate {
        let vsense = self.read_vsense(m, sp);
        let now = timer_get_large(m);
        let mut ret = PhaseUpdate::NotChanged;
        match self.phase {
            Phase::Notsync | Phase::NegHalfwave => {
                if !self.prev_vsense && vsense {
                    self.phaseref = now;
                    self.phase = Phase::PosHalfwave;
                    ret = PhaseUpdate::Changed;
                }
            }
            Phase::PosHalfwave => {
                let nextref = self.phaseref + HALFWAVE_DUR;
                if now >= nextref {
                    self.phaseref = nextref;
                    self.phase = Phase::NegHalfwave;
                    ret = PhaseUpdate::Changed;
                }
            }
        }
        self.prev_vsense = vsense;
        ret
    }

    pub fn get_phase(&self) -> Phase {
        self.phase
    }

    pub fn get_phaseref(&self) -> LargeTimestamp {
        self.phaseref
    }
}

// vim: ts=4 sw=4 expandtab
