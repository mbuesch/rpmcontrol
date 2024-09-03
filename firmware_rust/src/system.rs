use crate::{
    analog::{AcCapture, Adc, AdcChannel},
    fixpt::Fixpt,
    hw::mcu,
    mains::Mains,
    mutex::{CriticalSection, MutexCell, MutexRefCell},
    pi::Pi,
    speedo::Speedo,
};

const RPMPI_KP: Fixpt = Fixpt::new(10);
const RPMPI_KI: Fixpt = Fixpt::new(1);

#[allow(non_snake_case)]
pub struct SysPeriph {
    pub AC: mcu::AC,
    pub ADC: mcu::ADC,
    pub PORTA: mcu::PORTA,
    pub PORTB: mcu::PORTB,
    pub TC1: mcu::TC1,
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
enum SysState {
    /// POR system check.
    Check,
    /// Up and running.
    Run,
}

pub struct System {
    state: MutexCell<SysState>,
    adc: MutexRefCell<Adc>,
    speedo: MutexRefCell<Speedo>,
    mains: MutexRefCell<Mains>,
    rpm_pi: MutexRefCell<Pi>,
}

//TODO read setpoint
//TODO read speedo

impl System {
    pub const fn new() -> Self {
        Self {
            state: MutexCell::new(SysState::Check),
            adc: MutexRefCell::new(Adc::new()),
            speedo: MutexRefCell::new(Speedo::new()),
            mains: MutexRefCell::new(Mains::new()),
            rpm_pi: MutexRefCell::new(Pi::new(RPMPI_KP, RPMPI_KI)),
        }
    }

    pub fn init(&self, cs: CriticalSection<'_>, sp: &SysPeriph) {
        let mut adc = self.adc.borrow_mut(cs);
        adc.init(sp);
        adc.enable(
            AdcChannel::Setpoint.mask() | AdcChannel::ShuntDiff.mask() | AdcChannel::ShuntHi.mask(),
        );
        //TODO more inits for SysState::Check needed?
    }

    fn run_initial_check(&self, cs: CriticalSection<'_>, _sp: &SysPeriph) {
        let adc = self.adc.borrow(cs);

        let Some(_setpoint) = adc.get_result(AdcChannel::Setpoint) else {
            return;
        };
        //TODO

        let Some(_shuntdiff) = adc.get_result(AdcChannel::ShuntDiff) else {
            return;
        };
        //TODO

        let Some(_shunthi) = adc.get_result(AdcChannel::ShuntHi) else {
            return;
        };
        //TODO

        self.speedo.borrow_mut(cs).reset();
        self.state.set(cs, SysState::Run);
    }

    fn debug(&self, _cs: CriticalSection<'_>, sp: &SysPeriph) {
        sp.PORTB.portb.modify(|r, w| w.pb6().bit(!r.pb6().bit()));
    }

    pub fn run(&self, cs: CriticalSection<'_>, sp: &SysPeriph, ac: AcCapture) {
        self.debug(cs, sp);
        self.speedo.borrow_mut(cs).update(cs, &ac);

        match self.state.get(cs) {
            SysState::Check => {
                self.run_initial_check(cs, sp);
            }
            SysState::Run => {
                self.mains.borrow_mut(cs).run(cs, sp);
            }
        }

        core::hint::black_box(self.rpm_pi.borrow_mut(cs).run(
            core::hint::black_box(10.into()),
            core::hint::black_box(10.into()),
        ));

        let setpoint = {
            let mut adc = self.adc.borrow_mut(cs);
            adc.run(sp);
            adc.get_result(AdcChannel::Setpoint)
        };

        if let Some(setpoint) = setpoint {}
    }
}

// vim: ts=4 sw=4 expandtab
