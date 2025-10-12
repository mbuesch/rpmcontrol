use crate::ports::PORTA;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Shutoff {
    MachineShutoff = 0,
    MachineRunning,
}

impl core::ops::BitOr for Shutoff {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        if self == Self::MachineShutoff || other == Self::MachineShutoff {
            Self::MachineShutoff
        } else {
            Self::MachineRunning
        }
    }
}

impl core::ops::BitOrAssign for Shutoff {
    fn bitor_assign(&mut self, other: Self) {
        *self = *self | other;
    }
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
