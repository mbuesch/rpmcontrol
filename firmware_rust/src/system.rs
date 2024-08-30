use crate::{
    analog::{AcCapture, Adc, AdcChannel},
    hw::mcu,
    mutex::{CriticalSection, MutexCell, MutexRefCell},
    speedo::Speedo,
};

#[allow(non_snake_case)]
pub struct SysPeriph {
    pub AC: mcu::AC,
    pub ADC: mcu::ADC,
    pub PORTA: mcu::PORTA,
    pub PORTB: mcu::PORTB,
}

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
    speedo: MutexRefCell<Speedo>,
}

impl System {
    pub const fn new() -> Self {
        Self {
            state: MutexCell::new(SysState::Check),
            adc: Adc::new(),
            speedo: MutexRefCell::new(Speedo::new()),
        }
    }

    pub fn init(&self, cs: CriticalSection<'_>, sp: &SysPeriph) {
        self.adc.init(cs, sp);
        self.adc.enable(
            cs,
            AdcChannel::Setpoint.mask()
                | AdcChannel::Vsense.mask()
                | AdcChannel::ShuntDiff.mask()
                | AdcChannel::ShuntHi.mask(),
        );
        //TODO more inits for SysState::Check needed?
    }

    fn run_state_check(&self, cs: CriticalSection<'_>, _sp: &SysPeriph) {
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

        self.speedo.borrow_mut(cs).reset();
        self.state.set(cs, SysState::Syncing);
    }

    fn run_state_syncing(&self, _cs: CriticalSection<'_>, _sp: &SysPeriph) {
        //TODO
    }

    fn run_state_synced(&self, _cs: CriticalSection<'_>, _sp: &SysPeriph) {
        //TODO
    }

    fn debug(&self, _cs: CriticalSection<'_>, sp: &SysPeriph) {
        sp.PORTB.portb.modify(|r, w| w.pb6().bit(!r.pb6().bit()));
    }

    pub fn run(&self, cs: CriticalSection<'_>, sp: &SysPeriph, ac: AcCapture) {
        self.debug(cs, sp);
        self.speedo.borrow_mut(cs).update(cs, &ac);
        match self.state.get(cs) {
            SysState::Check => self.run_state_check(cs, sp),
            SysState::Syncing => self.run_state_syncing(cs, sp),
            SysState::Synced => self.run_state_synced(cs, sp),
        }
        self.adc.run(cs, sp)
    }
}

// vim: ts=4 sw=4 expandtab
