use crate::{
    fixpt::{Fixpt, fixpt},
    hw::interrupt,
    mutex::{IrqCtx, MainCtx, Mutex, MutexCell},
    ports::PORTA,
    timer::{LargeTimestamp, RelLargeTimestamp, timer_get_large, timer_get_large_cs},
};
use core::cell::Cell;

/// Mains sine wave period (50 Hz).
pub const MAINS_PERIOD_MS: Fixpt = fixpt!(20);
/// Mains sine wave period (50 Hz).
pub const MAINS_PERIOD: RelLargeTimestamp = RelLargeTimestamp::from_millis(20);

/// Mains sine wave half-wave length.
pub const MAINS_HALFWAVE_DUR_MS: Fixpt = MAINS_PERIOD_MS.const_div(fixpt!(2));
/// Mains sine wave half-wave length.
pub const MAINS_HALFWAVE_DUR: RelLargeTimestamp = MAINS_PERIOD.div(2);

/// Mains sine wave quarter-wave length.
pub const MAINS_QUARTERWAVE_DUR: RelLargeTimestamp = MAINS_PERIOD.div(4);

fn read_vsense() -> bool {
    PORTA.get(1)
}

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
}

impl Mains {
    pub const fn new() -> Self {
        Self {
            prev_vsense: MutexCell::new(false),
            phase: MutexCell::new(Phase::Notsync),
            phaseref: MutexCell::new(LargeTimestamp::new()),
        }
    }

    /// Run mains vsense pin reading and evaluation.
    pub fn run(&self, m: &MainCtx<'_>) -> PhaseUpdate {
        let mut ret = PhaseUpdate::NotChanged;

        let (vsense, vsense_stamp) =
            interrupt::free(|cs| (VSENSE.borrow(cs).get(), VSENSE_STAMP.borrow(cs).get()));

        match self.phase.get(m) {
            Phase::Notsync | Phase::NegHalfwave => {
                if !self.prev_vsense.get(m) && vsense {
                    self.phaseref.set(m, vsense_stamp);
                    self.phase.set(m, Phase::PosHalfwave);
                    ret = PhaseUpdate::Changed;
                }
            }
            Phase::PosHalfwave => {
                let nextref = self.phaseref.get(m) + MAINS_HALFWAVE_DUR;
                let now = timer_get_large();
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

    pub fn get_time_since_zerocrossing(&self, m: &MainCtx<'_>) -> Option<RelLargeTimestamp> {
        if self.phase.get(m) == Phase::Notsync {
            None
        } else {
            Some(timer_get_large() - self.phaseref.get(m))
        }
    }
}

static VSENSE: Mutex<Cell<bool>> = Mutex::new(Cell::new(false));
static VSENSE_STAMP: Mutex<Cell<LargeTimestamp>> = Mutex::new(Cell::new(LargeTimestamp::new()));

pub fn irq_handler_pcint(c: &IrqCtx) {
    let cs = c.cs();

    let now = timer_get_large_cs(cs);
    let vsense = read_vsense();

    let prev_vsense = VSENSE.borrow(cs).get();
    let prev_stamp = VSENSE_STAMP.borrow(cs).get();

    if vsense != prev_vsense && now >= prev_stamp + MAINS_QUARTERWAVE_DUR {
        VSENSE.borrow(cs).set(vsense);
        VSENSE_STAMP.borrow(cs).set(now);
    }
}

// vim: ts=4 sw=4 expandtab
