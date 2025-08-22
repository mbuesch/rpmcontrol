use crate::{
    hw::interrupt,
    mutex::{MainCtx, MutexCell},
    ports::PORTA,
    timer::{LargeTimestamp, RelLargeTimestamp, timer_get_large},
};

/// Mains sine wave period (50 Hz).
pub const MAINS_PERIOD: RelLargeTimestamp = RelLargeTimestamp::from_millis(20); // 50 Hz

/// Mains sine wave half-wave length.
pub const MAINS_HALFWAVE_DUR: RelLargeTimestamp = MAINS_PERIOD.div(2);

/// Mains sine wave quarter-wave length.
pub const MAINS_QUARTERWAVE_DUR: RelLargeTimestamp = MAINS_PERIOD.div(4);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Notsync,
    NegHalfwave,
    PosHalfwave,
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
    next_run: MutexCell<LargeTimestamp>,
}

impl Mains {
    pub const fn new() -> Self {
        Self {
            prev_vsense: MutexCell::new(false),
            phase: MutexCell::new(Phase::Notsync),
            phaseref: MutexCell::new(LargeTimestamp::new()),
            next_run: MutexCell::new(LargeTimestamp::new()),
        }
    }

    fn read_vsense(&self, _m: &MainCtx<'_>) -> bool {
        PORTA.get(1)
    }

    /// Run mains vsense pin reading and evaluation.
    pub fn run(&self, m: &MainCtx<'_>) -> PhaseUpdate {
        // Read vsense pin and timer with IRQs disabled
        // to not be interrupted and therefore to not skew the timestamp
        // in unpredictable ways.
        let (vsense, now) = interrupt::free(|_| {
            let vsense = self.read_vsense(m);
            let now = timer_get_large();
            (vsense, now)
        });

        let mut ret = PhaseUpdate::NotChanged;
        if now >= self.next_run.get(m) {
            match self.phase.get(m) {
                Phase::Notsync | Phase::NegHalfwave => {
                    if !self.prev_vsense.get(m) && vsense {
                        self.phaseref.set(m, now);
                        self.phase.set(m, Phase::PosHalfwave);
                        ret = PhaseUpdate::Changed;
                    }
                }
                Phase::PosHalfwave => {
                    let nextref = self.phaseref.get(m) + MAINS_HALFWAVE_DUR;
                    if now >= nextref {
                        self.phaseref.set(m, nextref);
                        self.phase.set(m, Phase::NegHalfwave);
                        // Mute vsense reading for another quarter wave.
                        self.next_run.set(m, nextref + MAINS_QUARTERWAVE_DUR);
                        ret = PhaseUpdate::Changed;
                    }
                }
            }
            self.prev_vsense.set(m, vsense);
        }
        ret
    }

    pub fn get_phase(&self, m: &MainCtx<'_>) -> Phase {
        self.phase.get(m)
    }

    pub fn get_phaseref(&self, m: &MainCtx<'_>) -> LargeTimestamp {
        self.phaseref.get(m)
    }

    pub fn get_time_since_zerocrossing(&self, m: &MainCtx<'_>) -> Option<RelLargeTimestamp> {
        if self.phase.get(m) == Phase::Notsync {
            None
        } else {
            Some(timer_get_large() - self.phaseref.get(m))
        }
    }
}

// vim: ts=4 sw=4 expandtab
