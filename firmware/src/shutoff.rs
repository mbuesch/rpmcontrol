use crate::ports::PORTA;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Shutoff {
    MachineShutoff = 0,
    MachineRunning,
}

/// Secondary shutoff path.
pub fn set_secondary_shutoff(state: Shutoff) {
    let n_shutoff = match state {
        Shutoff::MachineShutoff => false,
        Shutoff::MachineRunning => true,
    };
    PORTA.set(4, n_shutoff);
}

// vim: ts=4 sw=4 expandtab
