use crate::{
    mutex::CriticalSection,
    system::SysPeriph,
    timer::{timer_get_large, LargeTimestamp},
};

const HALFWAVE_DUR: LargeTimestamp = LargeTimestamp::from_millis(10);

#[derive(Clone)]
pub enum Phase {
    Notsync,
    PosHalfwave(LargeTimestamp),
    NegHalfwave(LargeTimestamp),
}

pub struct Mains {
    prev_vsense: bool,
    phase: Phase,
}

impl Mains {
    pub const fn new() -> Self {
        Self {
            prev_vsense: false,
            phase: Phase::Notsync,
        }
    }

    fn read_vsense(&self, _cs: CriticalSection<'_>, sp: &SysPeriph) -> bool {
        sp.PORTA.pina.read().pa1().bit()
    }

    pub fn run(&mut self, cs: CriticalSection<'_>, sp: &SysPeriph) {
        let vsense = self.read_vsense(cs, sp);
        let now = timer_get_large(cs);
        match self.phase {
            Phase::Notsync | Phase::NegHalfwave(_) => {
                if !self.prev_vsense && vsense {
                    self.phase = Phase::PosHalfwave(now);
                }
            }
            Phase::PosHalfwave(refstamp) => {
                let nextref = refstamp + HALFWAVE_DUR.into();
                if now >= nextref {
                    self.phase = Phase::NegHalfwave(nextref);
                }
            }
        }
        self.prev_vsense = vsense;
    }

    pub fn get_phase(&self) -> Phase {
        self.phase.clone()
    }
}

// vim: ts=4 sw=4 expandtab
