use crate::{
    fixpt::Fixpt,
    hw::interrupt,
    mains::{MAINS_HALFWAVE_DUR, Phase, PhaseUpdate},
    mutex::{IrqCtx, MainCtx, MainCtxCell, Mutex},
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

static TRIAC_TIMER_STATE: Mutex<Cell<TriacTimerState>> =
    Mutex::new(Cell::new(TriacTimerState::TrigSet));
static TRIAC_TIMER_COUNT: Mutex<Cell<u8>> = Mutex::new(Cell::new(0));

#[derive(Clone, Copy, PartialEq, Eq)]
enum TriacTimerState {
    TrigSet = 0,
    TrigClr,
}

/// Arm the triac timer to an absolute time stamp.
fn triac_timer_do_arm(ts: Timestamp) {
    fence(SeqCst);
    timer_interrupt_a_arm(ts);
}

/// Cancel the possibly pending triac timer.
fn triac_timer_do_cancel() {
    fence(SeqCst);
    timer_interrupt_a_cancel();
}

/// Triac timer interrupt service routine.
/// This routine executes at the armed time.
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

/// Arm the triac timer beginning at the absolute time stamp
/// for the specified number of times.
fn triac_timer_arm(begin_time: Timestamp, count: u8) {
    interrupt::free(|cs| {
        TRIAC_TIMER_STATE.borrow(cs).set(TriacTimerState::TrigSet);
        TRIAC_TIMER_COUNT.borrow(cs).set(count);
        triac_timer_do_arm(begin_time);
    });
}

/// Cancel the possibly pending triac timer
/// and avoid any pending interrupt service routine.
fn triac_timer_cancel() {
    interrupt::free(|cs| {
        TRIAC_TIMER_COUNT.borrow(cs).set(0);
        triac_timer_do_cancel();
    });
}

/// Calculate the number of triggers needed for a specified trigger offset time.
fn calc_trig_count(trig_offs: RelLargeTimestamp) -> u8 {
    // The duration where re-triggers should happen.
    let retrig_thres =
        MAINS_HALFWAVE_DUR.div(4) + MAINS_HALFWAVE_DUR.div(8) + MAINS_HALFWAVE_DUR.div(16);

    let retrig_dur = if trig_offs < retrig_thres {
        // We are in the upper retrig range.
        // Left-hand part of the sine halfwave duration.
        retrig_thres - trig_offs
    } else if trig_offs > MAINS_HALFWAVE_DUR - retrig_thres {
        // We are in the lower retrig range.
        // Right-hand part of the sine halfwave duration.
        MAINS_HALFWAVE_DUR - trig_offs
    } else {
        // We are in the center retrig range.
        // Do not do retriggers.
        RelLargeTimestamp::from_micros(0)
    };

    let retrig_dur: i16 = retrig_dur.into();
    let half_pulse_len: i8 = HALF_PULSE_LEN.into();
    let pulse_len = half_pulse_len as i16 * 2;

    // Calculate the number of re-triggers needed
    // based on the re-trigger duration.
    ((retrig_dur / pulse_len) as u8).max(1)
}

/// Trigger the triac now.
/// true -> trigger now.
fn set_trigger(trigger: bool) {
    let trigger = !trigger; // negative logic at triac gate.
    PORTB.set(3, trigger);
}

pub struct Triac {
    phi_offs: MainCtxCell<RelLargeTimestamp>,
    trigger_pending: MainCtxCell<bool>,
}

impl Triac {
    pub const fn new() -> Self {
        Self {
            phi_offs: MainCtxCell::new(RelLargeTimestamp::new()),
            trigger_pending: MainCtxCell::new(false),
        }
    }

    /// Set the next triac trigger offset, in milliseconds.
    /// Relative to the mains zero crossing.
    pub fn set_phi_offs_ms(&self, m: &MainCtx<'_>, ms: Fixpt) {
        self.phi_offs.set(m, RelLargeTimestamp::from_ms_fixpt(ms));
    }

    /// Set the next triac trigger offset to never trigger.
    #[inline(never)]
    pub fn set_phi_offs_shutoff(&self, m: &MainCtx<'_>) {
        triac_timer_cancel();
        self.phi_offs.set(m, MAINS_HALFWAVE_DUR);
    }

    /// Run the triac trigger timer arm logic.
    pub fn run(
        &self,
        m: &MainCtx<'_>,
        phase_update: PhaseUpdate,
        phase: Phase,
        phaseref: LargeTimestamp,
        shutoff: Shutoff,
    ) {
        // Don't trigger if we're not sync'd to mains
        // or if we have a shutoff request.
        if phase == Phase::Notsync || shutoff == Shutoff::MachineShutoff {
            triac_timer_cancel();
            set_trigger(false);
            self.trigger_pending.set(m, false);
            return;
        }

        // Zero crossing detected?
        // If so, then we need to arm the next trigger timer soon.
        if phase_update == PhaseUpdate::Changed {
            triac_timer_cancel();
            set_trigger(false);
            self.trigger_pending.set(m, true);
        }

        // Check if we need to arm the next trigger timer.
        if self.trigger_pending.get(m) {
            let trig_offs = self.phi_offs.get(m);
            if trig_offs <= MAX_TRIG_OFFS {
                // Calculate the absolute trigger time.
                let trig_time = phaseref + trig_offs;

                // Does the trigger time fit into an 8 bit timestamp?
                if trig_time - timer_get_large() <= RelLargeTimestamp::from_ticks(0x3F) {
                    // Convert trigger time to 8 bit stamp.
                    let trig_time: Timestamp = trig_time.into();
                    // Arm the triac trigger timer at the calculated absolute time.
                    triac_timer_arm(trig_time, calc_trig_count(trig_offs));
                    self.trigger_pending.set(m, false);
                }
            } else {
                // The trigger offset is in shutoff state.
                // Reset trigger and don't arm a timer.
                set_trigger(false);
                self.trigger_pending.set(m, false);
            }
        }
    }
}

// vim: ts=4 sw=4 expandtab
