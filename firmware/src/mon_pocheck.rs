use crate::{
    mains::MAINS_HALFWAVE_DUR_MS,
    mutex::{MainCtx, MainCtxCell},
    shutoff::Shutoff,
    speedo::MotorSpeed,
    system::{debug_toggle, rpm},
    timer::{LargeTimestamp, RelLargeTimestamp, timer_get_large},
};
use avr_q::{Q7p8, q7p8};

/// Duration of the `PoStatePart::Pre` part.
const DUR_PRE: RelLargeTimestamp = RelLargeTimestamp::from_millis(50);

/// Duration of the `PoStatePart::Check` part.
const DUR_CHECK: RelLargeTimestamp = RelLargeTimestamp::from_millis(400);

/// RPM below or equal to this limit are considered to be zero RPM.
const RPM_ZERO_LIMIT: Q7p8 = rpm!(5);

/// Triac offset for the enabled-check.
const TRIAC_TRIG_OFFS_ENABLED_MS: Q7p8 = MAINS_HALFWAVE_DUR_MS.const_div(q7p8!(const 10));

/// Show state transitions on the debug pin?
const DEBUG_PIN_ENA: bool = false;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PoState {
    /// Initial check: Primary and secondary shutoff.
    CheckIdle = 0,

    /// Secondary shutoff only. Primary running.
    CheckSecondaryShutoff,

    /// Primary shutoff only. Secondary running.
    CheckPrimaryShutoff,

    /// Error detected. This state is sticky and final.
    Error,

    /// All checks done. Everything is Ok. This state is sticky and final.
    DoneOk,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum PoStatePart {
    /// First part of the `PoState`.
    /// No actual checks during this part.
    Pre = 0,

    /// Second part of the `PoState`.
    /// The check is performed during this part.
    Check,
}

impl PoState {
    pub fn next(&self) -> Self {
        match self {
            PoState::CheckIdle => PoState::CheckSecondaryShutoff,
            PoState::CheckSecondaryShutoff => PoState::CheckPrimaryShutoff,
            PoState::CheckPrimaryShutoff => PoState::DoneOk,
            PoState::Error => PoState::Error, // never leave error state.
            PoState::DoneOk => PoState::DoneOk, // never leave done state.
        }
    }
}

pub struct PoCheck {
    state: MainCtxCell<PoState>,
    next_transition: MainCtxCell<LargeTimestamp>,
    part: MainCtxCell<PoStatePart>,
}

impl PoCheck {
    pub const fn new() -> Self {
        Self {
            state: MainCtxCell::new(PoState::CheckIdle),
            next_transition: MainCtxCell::new(LargeTimestamp::new()),
            part: MainCtxCell::new(PoStatePart::Pre),
        }
    }

    pub fn init(&self, m: &MainCtx<'_>) {
        self.next_transition.set(m, timer_get_large() + DUR_PRE);
    }

    pub fn run(&self, m: &MainCtx<'_>, speedo_hz: Option<MotorSpeed>) -> PoState {
        let mut state = self.state.get(m);

        match state {
            PoState::CheckIdle | PoState::CheckSecondaryShutoff | PoState::CheckPrimaryShutoff => {
                // Transition to the next state part?
                let now = timer_get_large();
                let transition = now >= self.next_transition.get(m);

                match self.part.get(m) {
                    PoStatePart::Pre => {
                        if transition {
                            self.part.set(m, PoStatePart::Check);
                            self.next_transition.set(m, now + DUR_CHECK);
                            if DEBUG_PIN_ENA {
                                debug_toggle();
                            }
                        }
                    }
                    PoStatePart::Check => {
                        if transition {
                            self.part.set(m, PoStatePart::Pre);
                            state = state.next();
                            self.next_transition.set(m, now + DUR_PRE);
                            if DEBUG_PIN_ENA {
                                debug_toggle();
                            }
                        }

                        // Run the actual machine state check.
                        if self.is_error_condition(state, speedo_hz) {
                            // Error detected.
                            // Shutoff everything.
                            state = PoState::Error;
                        }
                    }
                }
            }
            PoState::Error | PoState::DoneOk => (),
        }

        self.state.set(m, state);
        state
    }

    fn is_error_condition(&self, state: PoState, speedo_hz: Option<MotorSpeed>) -> bool {
        match state {
            PoState::CheckIdle
            | PoState::CheckSecondaryShutoff
            | PoState::CheckPrimaryShutoff
            | PoState::Error
            | PoState::DoneOk => {
                let rpm_is_zero = speedo_hz
                    .map(|s| s.as_16hz() <= RPM_ZERO_LIMIT)
                    .unwrap_or(true);
                // RPM must always be zero throughout the whole test.
                !rpm_is_zero
            }
        }
    }

    pub fn get_triac_phi_offs_ms(&self, m: &MainCtx<'_>) -> Option<Q7p8> {
        match self.state.get(m) {
            PoState::CheckIdle => None,
            PoState::CheckSecondaryShutoff => Some(TRIAC_TRIG_OFFS_ENABLED_MS),
            PoState::CheckPrimaryShutoff => Some(TRIAC_TRIG_OFFS_ENABLED_MS),
            PoState::Error => None,
            PoState::DoneOk => None,
        }
    }

    pub fn get_triac_shutoff(&self, m: &MainCtx<'_>) -> Shutoff {
        match self.state.get(m) {
            PoState::CheckIdle => Shutoff::MachineShutoff,
            PoState::CheckSecondaryShutoff => Shutoff::MachineRunning,
            PoState::CheckPrimaryShutoff => Shutoff::MachineShutoff,
            PoState::Error => Shutoff::MachineShutoff,
            PoState::DoneOk => Shutoff::MachineRunning,
        }
    }

    pub fn get_secondary_shutoff(&self, m: &MainCtx<'_>) -> Shutoff {
        match self.state.get(m) {
            PoState::CheckIdle => Shutoff::MachineShutoff,
            PoState::CheckSecondaryShutoff => Shutoff::MachineShutoff,
            PoState::CheckPrimaryShutoff => Shutoff::MachineRunning,
            PoState::Error => Shutoff::MachineShutoff,
            PoState::DoneOk => Shutoff::MachineRunning,
        }
    }
}

// vim: ts=4 sw=4 expandtab
