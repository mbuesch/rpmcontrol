use crate::{
    fixpt::Fixpt,
    mains::MAINS_QUARTERWAVE_DUR_MS,
    mutex::{MainCtx, MutexCell},
    shutoff::Shutoff,
    speedo::MotorSpeed,
    system::rpm,
    timer::{LargeTimestamp, RelLargeTimestamp, timer_get_large},
};

const TRANSITION_DIST: RelLargeTimestamp = RelLargeTimestamp::from_millis(400);
const RPM_ZERO_LIMIT: Fixpt = rpm!(5);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PoState {
    CheckIdle = 0,
    CheckSecondaryShutoff,
    CheckPrimaryShutoff,
    Error,
    DoneOk,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum PoStatePart {
    Pre = 0,
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
    state: MutexCell<PoState>,
    next_transition: MutexCell<LargeTimestamp>,
    part: MutexCell<PoStatePart>,
}

impl PoCheck {
    pub const fn new() -> Self {
        Self {
            state: MutexCell::new(PoState::CheckIdle),
            next_transition: MutexCell::new(LargeTimestamp::new()),
            part: MutexCell::new(PoStatePart::Pre),
        }
    }

    pub fn init(&self, m: &MainCtx<'_>) {
        self.next_transition
            .set(m, timer_get_large() + TRANSITION_DIST);
    }

    pub fn run(&self, m: &MainCtx<'_>, speedo_hz: Option<MotorSpeed>) -> PoState {
        let mut state = self.state.get(m);

        match state {
            PoState::CheckIdle | PoState::CheckSecondaryShutoff | PoState::CheckPrimaryShutoff => {
                // Transition to the next state part?
                let now = timer_get_large();
                let transition = if now >= self.next_transition.get(m) {
                    self.next_transition.set(m, now + TRANSITION_DIST);
                    true
                } else {
                    false
                };

                match self.part.get(m) {
                    PoStatePart::Pre => {
                        if transition {
                            self.part.set(m, PoStatePart::Check);
                        }
                    }
                    PoStatePart::Check => {
                        if transition {
                            self.part.set(m, PoStatePart::Pre);
                            state = state.next();
                            crate::system::debug_toggle();
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

    pub fn get_triac_phi_offs_ms(&self, m: &MainCtx<'_>) -> Option<Fixpt> {
        match self.state.get(m) {
            PoState::CheckIdle => None,
            PoState::CheckSecondaryShutoff => Some(MAINS_QUARTERWAVE_DUR_MS),
            PoState::CheckPrimaryShutoff => Some(MAINS_QUARTERWAVE_DUR_MS),
            PoState::Error => None,
            PoState::DoneOk => None,
        }
    }

    pub fn get_triac_shutoff(&self, m: &MainCtx<'_>) -> Shutoff {
        match self.state.get(m) {
            PoState::CheckIdle => Shutoff::MachineShutoff,
            PoState::CheckSecondaryShutoff => Shutoff::MachineShutoff, //TODO Shutoff::MachineRunning,
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
