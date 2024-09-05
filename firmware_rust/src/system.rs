use crate::{
    analog::{Ac, AcCapture, Adc, AdcChannel},
    fixpt::{fixpt, Fixpt},
    hw::mcu,
    mains::Mains,
    mutex::{CriticalSection, MutexCell, MutexRefCell},
    pi::{Pi, PiParams},
    speedo::Speedo,
    timer::{timer_get, Timestamp, TIMER_TICK_US},
    triac::Triac,
};

const RPMPI_DT: u8 = (1000_u16 / TIMER_TICK_US as u16) as u8;
const RPMPI_KP: Fixpt = fixpt!(10 / 1); //TODO
const RPMPI_KI: Fixpt = fixpt!(1 / 10); //TODO
const RPMPI_ILIM: Fixpt = fixpt!(10 / 1);

fn setpoint_to_f(adc: u16) -> Fixpt {
    let int = (adc >> 3) as i16;
    let frac = (adc & 7) << (Fixpt::SHIFT - 3);
    Fixpt::from_parts(int, frac)
}

fn f_to_trig_offs(f: Fixpt) -> Fixpt {
    // Convert 0..128 Hz into pi..0 radians.
    // Convert pi..0 radians into 20..0 ms.
    let fmax = Fixpt::new(128);
    if f <= fmax {
        let fact = fixpt!(5 / 32); // 1 / 6.4
        (fmax - f) * fact
    } else {
        Fixpt::new(0)
    }
}

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
    ac: Ac,
    adc: MutexRefCell<Adc>,
    speedo: MutexRefCell<Speedo>,
    mains: MutexRefCell<Mains>,
    rpm_pi: MutexRefCell<Pi>,
    next_rpm_pi: MutexCell<Timestamp>,
    triac: Triac,
}

impl System {
    pub const fn new() -> Self {
        Self {
            ac: Ac::new(),
            adc: MutexRefCell::new(Adc::new()),
            speedo: MutexRefCell::new(Speedo::new()),
            mains: MutexRefCell::new(Mains::new()),
            rpm_pi: MutexRefCell::new(Pi::new(PiParams {
                kp: RPMPI_KP,
                ki: RPMPI_KI,
                ilim: RPMPI_ILIM,
            })),
            next_rpm_pi: MutexCell::new(Timestamp::new()),
            triac: Triac::new(),
        }
    }

    pub fn init(&self, cs: CriticalSection<'_>, sp: &SysPeriph) {
        let mut adc = self.adc.borrow_mut(cs);
        adc.init(sp);
        adc.enable(
            AdcChannel::Setpoint.mask() | AdcChannel::ShuntDiff.mask() | AdcChannel::ShuntHi.mask(),
        );
        self.ac.init(sp);
    }

    /*
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
    */

    fn debug(&self, _cs: CriticalSection<'_>, sp: &SysPeriph) {
        sp.PORTB.portb.modify(|r, w| w.pb6().bit(!r.pb6().bit()));
    }

    pub fn run(&self, cs: CriticalSection<'_>, sp: &SysPeriph, ac: AcCapture) {
        self.debug(cs, sp);

        let speedo_hz = {
            let mut speedo = self.speedo.borrow_mut(cs);
            speedo.update(cs, &ac);
            speedo.get_freq_hz()
        };

        let (phase_update, phase) = {
            let mut mains = self.mains.borrow_mut(cs);
            let phase_update = mains.run(cs, sp);
            let phase = mains.get_phase();
            (phase_update, phase)
        };

        let (setpoint, _shuntdiff, _shunthi) = {
            let mut adc = self.adc.borrow_mut(cs);
            adc.run(sp);
            (
                adc.get_result(AdcChannel::Setpoint),
                adc.get_result(AdcChannel::ShuntDiff),
                adc.get_result(AdcChannel::ShuntHi),
            )
        };

        let now = timer_get(cs);

        if now >= self.next_rpm_pi.get(cs) {
            self.next_rpm_pi.set(cs, now + RPMPI_DT);

            if let Some(setpoint) = setpoint {
                if let Some(speedo_hz) = speedo_hz {
                    let setpoint = setpoint_to_f(setpoint);
                    let y = {
                        let mut rpm_pi = self.rpm_pi.borrow_mut(cs);
                        rpm_pi.setpoint(setpoint);
                        rpm_pi.run(speedo_hz)
                    };
                    let phi_offs_ms = f_to_trig_offs(y);
                    self.triac.set_phi_offs_ms(cs, phi_offs_ms);
                }
            }
        }

        self.triac.run(cs, sp, phase_update, &phase);
    }
}

// vim: ts=4 sw=4 expandtab
