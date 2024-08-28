use crate::{analog::Adc, hw::Peripherals};

enum SysState {
    /// Initial POR state.
    Init,
    /// POR system check.
    Check,
    /// Synchronizing to line phase.
    Syncing,
    /// Synchronized.
    Synced,
}

pub struct System {
    state: SysState,
    adc: Adc,
}

impl System {
    pub const fn new() -> Self {
        Self {
            state: SysState::Init,
            adc: Adc::new(),
        }
    }

    pub fn init(&self, dp: &Peripherals) {
        self.adc.init(dp)
    }

    pub fn run(&self, dp: &Peripherals) {
        self.adc.run(dp)
    }
}

// vim: ts=4 sw=4 expandtab
