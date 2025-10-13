use crate::{
    fixpt::Fixpt,
    hw::interrupt,
    mains::{MAINS_HALFWAVE_DUR, Phase, PhaseUpdate},
    mutex::{IrqCtx, MainCtx, Mutex, MutexCell},
    ports::PORTB,
    shutoff::Shutoff,
    timer::{
        LargeTimestamp, RelLargeTimestamp, RelTimestamp, Timestamp, timer_get_large,
        timer_interrupt_a_arm, timer_interrupt_a_cancel,
    },
};
use core::{
    cell::Cell,
    sync::atomic::{Ordering::SeqCst, fence},
};

/// Triac trigger pulse length set-duration or clear-duration.
const HALF_PULSE_LEN: RelTimestamp = RelTimestamp::from_micros(64);

/// The last point a trigger can happen.
/// Relative to the halfwave start.
const MAX_TRIG_OFFS: RelLargeTimestamp =
    MAINS_HALFWAVE_DUR.sub(RelLargeTimestamp::from_micros(150));

fn set_trigger(trigger: bool) {
    let trigger = !trigger; // negative logic at triac gate.
    PORTB.set(3, trigger);
}

static TRIAC_TIMER_STATE: Mutex<Cell<TriacTimerState>> =
    Mutex::new(Cell::new(TriacTimerState::TrigSet));
static TRIAC_TIMER_COUNT: Mutex<Cell<u8>> = Mutex::new(Cell::new(0));

#[derive(Clone, Copy, PartialEq, Eq)]
enum TriacTimerState {
    TrigSet = 0,
    TrigClr,
}

fn triac_timer_do_arm(ts: Timestamp) {
    fence(SeqCst);
    timer_interrupt_a_arm(ts);
}

fn triac_timer_do_cancel() {
    fence(SeqCst);
    timer_interrupt_a_cancel();
}

pub fn triac_timer_interrupt(c: &IrqCtx<'_>, now: Timestamp) {
    let cs = c.cs();

    let mut state = TRIAC_TIMER_STATE.borrow(cs).get();
    let mut count = TRIAC_TIMER_COUNT.borrow(cs).get();

    if count > 0 {
        let arm;
        match state {
            TriacTimerState::TrigSet => {
                set_trigger(true);
                arm = true;
                state = TriacTimerState::TrigClr;
            }
            TriacTimerState::TrigClr => {
                set_trigger(false);
                count -= 1;
                arm = count != 0;
                state = TriacTimerState::TrigSet;
            }
        }

        TRIAC_TIMER_STATE.borrow(cs).set(state);
        TRIAC_TIMER_COUNT.borrow(cs).set(count);

        if arm {
            triac_timer_do_arm(now + HALF_PULSE_LEN);
        }
    }
}

fn triac_timer_arm(begin_time: Timestamp, count: u8) {
    interrupt::free(|cs| {
        TRIAC_TIMER_STATE.borrow(cs).set(TriacTimerState::TrigSet);
        TRIAC_TIMER_COUNT.borrow(cs).set(count);
        triac_timer_do_arm(begin_time);
    });
}

fn triac_timer_cancel() {
    interrupt::free(|cs| {
        TRIAC_TIMER_COUNT.borrow(cs).set(0);
        triac_timer_do_cancel();
    });
}

fn calc_trig_count(trig_offs: RelLargeTimestamp) -> u8 {
    let retrig_thres =
        MAINS_HALFWAVE_DUR.div(4) + MAINS_HALFWAVE_DUR.div(8) + MAINS_HALFWAVE_DUR.div(16);

    let retrig_dur = if trig_offs < retrig_thres {
        retrig_thres - trig_offs
    } else if trig_offs > MAINS_HALFWAVE_DUR - retrig_thres {
        MAINS_HALFWAVE_DUR - trig_offs
    } else {
        RelLargeTimestamp::from_micros(0)
    };

    let retrig_dur: i16 = retrig_dur.into();
    let half_pulse_len: i8 = HALF_PULSE_LEN.into();
    let pulse_len = half_pulse_len as i16 * 2;

    ((retrig_dur / pulse_len) as u8).max(1)
}

pub struct Triac {
    phi_offs: MutexCell<RelLargeTimestamp>,
    trigger_pending: MutexCell<bool>,
}

impl Triac {
    pub const fn new() -> Self {
        Self {
            phi_offs: MutexCell::new(RelLargeTimestamp::new()),
            trigger_pending: MutexCell::new(false),
        }
    }

    pub fn set_phi_offs_ms(&self, m: &MainCtx<'_>, ms: Fixpt) {
        self.phi_offs.set(m, RelLargeTimestamp::from_ms_fixpt(ms));
    }

    pub fn set_phi_offs_shutoff(&self, m: &MainCtx<'_>) {
        self.phi_offs.set(m, MAINS_HALFWAVE_DUR);
    }

    pub fn run(
        &self,
        m: &MainCtx<'_>,
        phase_update: PhaseUpdate,
        phase: Phase,
        phaseref: LargeTimestamp,
        shutoff: Shutoff,
    ) {
        if phase == Phase::Notsync || shutoff == Shutoff::MachineShutoff {
            triac_timer_cancel();
            set_trigger(false);
            self.trigger_pending.set(m, false);
            return;
        }

        if phase_update == PhaseUpdate::Changed {
            triac_timer_cancel();
            set_trigger(false);
            self.trigger_pending.set(m, true);
        }

        if self.trigger_pending.get(m) {
            let trig_offs = self.phi_offs.get(m);
            if trig_offs <= MAX_TRIG_OFFS {
                let trig_time = phaseref + trig_offs;

                if trig_time - timer_get_large() <= RelLargeTimestamp::from_ticks(0x3F) {
                    let trig_time: Timestamp = trig_time.into();
                    triac_timer_arm(trig_time, calc_trig_count(trig_offs));
                    self.trigger_pending.set(m, false);
                }
            } else {
                set_trigger(false);
                self.trigger_pending.set(m, false);
            }
        }
    }
}

// vim: ts=4 sw=4 expandtab
