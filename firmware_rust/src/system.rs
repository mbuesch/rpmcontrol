use crate::{
    analog::{AcCapture, Adc, AdcChannel},
    hw::Peripherals,
    mutex::{CriticalSection, MutexCell},
};

#[derive(Copy, Clone)]
enum SysState {
    /// POR system check.
    Check,
    /// Synchronizing to line phase.
    Syncing,
    /// Synchronized.
    Synced,
}

pub struct System {
    state: MutexCell<SysState>,
    adc: Adc,
}

impl System {
    pub const fn new() -> Self {
        Self {
            state: MutexCell::new(SysState::Check),
            adc: Adc::new(),
        }
    }

    pub fn init(&self, cs: CriticalSection<'_>, dp: &Peripherals) {
        self.adc.init(cs, dp);
        self.adc.enable(
            cs,
            AdcChannel::Setpoint.mask()
                | AdcChannel::Vsense.mask()
                | AdcChannel::ShuntDiff.mask()
                | AdcChannel::ShuntHi.mask(),
        );
        //TODO more inits for SysState::Check needed?
    }

    fn run_state_check(&self, cs: CriticalSection<'_>, _dp: &Peripherals, _ac: &AcCapture) {
        let Some(_setpoint) = self.adc.get_result(cs, AdcChannel::Setpoint) else {
            return;
        };
        //TODO

        let Some(_vsense) = self.adc.get_result(cs, AdcChannel::Vsense) else {
            return;
        };
        //TODO

        let Some(_shuntdiff) = self.adc.get_result(cs, AdcChannel::ShuntDiff) else {
            return;
        };
        //TODO

        let Some(_shunthi) = self.adc.get_result(cs, AdcChannel::ShuntHi) else {
            return;
        };
        //TODO

        self.state.set(cs, SysState::Syncing);
    }

    fn run_state_syncing(&self, _cs: CriticalSection<'_>, _dp: &Peripherals, _ac: &AcCapture) {
        //TODO
    }

    fn run_state_synced(&self, _cs: CriticalSection<'_>, _dp: &Peripherals, _ac: &AcCapture) {
        //TODO
    }

    pub fn run(&self, cs: CriticalSection<'_>, dp: &Peripherals, ac: AcCapture) {
        match self.state.get(cs) {
            SysState::Check => self.run_state_check(cs, dp, &ac),
            SysState::Syncing => self.run_state_syncing(cs, dp, &ac),
            SysState::Synced => self.run_state_synced(cs, dp, &ac),
        }
        self.adc.run(cs, dp)
    }
}

// vim: ts=4 sw=4 expandtab
