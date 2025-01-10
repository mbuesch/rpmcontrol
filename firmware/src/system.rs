use crate::{
    analog::{Ac, AcCapture, Adc, AdcChannel},
    fixpt::{fixpt, Fixpt},
    hw::mcu,
    mains::Mains,
    mutex::{MainCtx, MutexCell},
    pi::{Pi, PiParams},
    speedo::{MotorSpeed, Speedo},
    timer::{timer_get, timer_get_large, LargeTimestamp, RelLargeTimestamp, RelTimestamp},
    triac::Triac,
};

const RPMPI_DT: RelLargeTimestamp = RelLargeTimestamp::from_millis(10);
const RPMPI_PARAMS: PiParams = PiParams {
    kp: fixpt!(10 / 1), //TODO
    ki: fixpt!(1 / 10), //TODO
    ilim: fixpt!(10 / 1),
};

/// Convert 0..0x3FF to 0..128 Hz to 0..8 16Hz
fn setpoint_to_f(adc: u16) -> Fixpt {
    Fixpt::from_fraction(adc as i16, 8 * 16)
}

/// Convert -8..8 16Hz into pi..0 radians.
/// Convert pi..0 radians into 20..0 ms.
fn f_to_trig_offs(f: Fixpt) -> Fixpt {
    let fmin = Fixpt::from_int(-8);
    let fmax = Fixpt::from_int(8);
    let f = f.max(fmin);
    let f = f.min(fmax);
    let fact = fixpt!(20 / 16);
    (fmax - f) * fact
}

#[allow(non_snake_case)]
pub struct SysPeriph {
    pub AC: mcu::AC,
    pub ADC: mcu::ADC,
    pub PORTA: mcu::PORTA,
    pub PORTB: mcu::PORTB,
    pub TC1: mcu::TC1,
}

#[allow(dead_code)]
pub fn debug(m: &MainCtx<'_>, sp: &SysPeriph, ticks: i8) {
    sp.PORTB.portb.modify(|_, w| w.pb6().set_bit());
    let end = timer_get(&m.to_any()) + RelTimestamp::from_ticks(ticks);
    while timer_get(&m.to_any()) < end {}
    sp.PORTB.portb.modify(|_, w| w.pb6().clear_bit());
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
    adc: Adc,
    speedo: Speedo,
    mains: Mains,
    rpm_pi: Pi,
    next_rpm_pi: MutexCell<LargeTimestamp>,
    triac: Triac,
}

impl System {
    pub const fn new() -> Self {
        Self {
            ac: Ac::new(),
            adc: Adc::new(),
            speedo: Speedo::new(),
            mains: Mains::new(),
            rpm_pi: Pi::new(),
            next_rpm_pi: MutexCell::new(LargeTimestamp::new()),
            triac: Triac::new(),
        }
    }

    pub fn init(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        self.adc.init(m, sp);
        self.adc.enable(
            m,
            AdcChannel::Setpoint.mask() | AdcChannel::ShuntDiff.mask() | AdcChannel::ShuntHi.mask(),
        );
        self.ac.init(sp);
        self.triac.shutoff(m);
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

    pub fn run(&self, m: &MainCtx<'_>, sp: &SysPeriph, ac: AcCapture) {
        self.speedo.update(m, sp, &ac);
        let speedo_hz = self.speedo.get_speed(m).unwrap_or(MotorSpeed::zero());

        let phase_update = self.mains.run(m, sp);
        let phase = self.mains.get_phase(m);
        let phaseref = self.mains.get_phaseref(m);

        self.adc.run(m, sp);
        let setpoint = self.adc.get_result(m, AdcChannel::Setpoint);
        let shuntdiff = self.adc.get_result(m, AdcChannel::ShuntDiff);
        let shunthi = self.adc.get_result(m, AdcChannel::ShuntHi);

        let now = timer_get_large(m);
        if now >= self.next_rpm_pi.get(m) {
            self.next_rpm_pi.set(m, now + RPMPI_DT);

            if let Some(setpoint) = setpoint {
                let setpoint = setpoint_to_f(setpoint);
                let y = self
                    .rpm_pi
                    .run(m, &RPMPI_PARAMS, setpoint, speedo_hz.as_16hz());
                //let y = setpoint;
                let phi_offs_ms = f_to_trig_offs(y);
                //debug(m, sp, phi_offs_ms.to_int() as i8);
                self.triac.set_phi_offs_ms(m, phi_offs_ms);
            } else {
                self.triac.shutoff(m);
            }
        }

        self.triac.run(m, sp, phase_update, phase, phaseref);
    }
}

// vim: ts=4 sw=4 expandtab
