use crate::{
    analog::{Ac, AcCapture, Adc, AdcChannel},
    debug::Debug,
    fixpt::{Fixpt, fixpt},
    hw::mcu,
    mains::Mains,
    mutex::{MainCtx, MutexCell},
    pi::{Pi, PiParams},
    ports::PORTB,
    speedo::{MotorSpeed, Speedo},
    timer::{LargeTimestamp, RelLargeTimestamp, RelTimestamp, timer_get, timer_get_large},
    triac::Triac,
};

const RPMPI_DT: RelLargeTimestamp = RelLargeTimestamp::from_millis(10);
const RPMPI_PARAMS: PiParams = PiParams {
    kp: fixpt!(5 / 2),
    ki: fixpt!(1 / 8),
    ilim: fixpt!(12 / 1),
};
const RPMPI_PARAMS_SYNCING: PiParams = PiParams {
    kp: fixpt!(5 / 2),
    ki: fixpt!(0),
    ilim: fixpt!(0),
};
const RPM_SYNC_THRES_16HZ: Fixpt = fixpt!(25 / 24); // 1000/min

const MAX_16HZ: i16 = 25; // 400 Hz, 24000/min

/// Convert 0..0x3FF to 0..400 Hz to 0..25 16Hz
fn setpoint_to_f(adc: u16) -> Fixpt {
    Fixpt::from_fraction(adc as i16, 8) / fixpt!(128 / 25)
}

/// Convert -25..25 16Hz into pi..0 radians.
/// Convert pi..0 radians into 20..0 ms.
fn f_to_trig_offs(f: Fixpt) -> Fixpt {
    let fmin = Fixpt::from_int(-MAX_16HZ);
    let fmax = Fixpt::from_int(MAX_16HZ);
    let f = f.max(fmin);
    let f = f.min(fmax);
    let fact = fixpt!(20 / 50);
    (fmax - f) * fact
}

#[allow(non_snake_case)]
pub struct SysPeriph {
    pub AC: mcu::AC,
    pub ADC: mcu::ADC,
}

#[allow(dead_code)]
pub fn debug(ticks: i8) {
    PORTB.set(6, true);
    let end = timer_get() + RelTimestamp::from_ticks(ticks);
    while timer_get() < end {}
    PORTB.set(6, false);
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
enum SysState {
    /// POR system check.
    PorCheck,
    /// Speedometer syncing.
    Syncing,
    /// Up and running.
    Running,
}

pub struct System {
    state: MutexCell<SysState>,
    ac: Ac,
    adc: Adc,
    speedo: Speedo,
    prev_speed: MutexCell<MotorSpeed>,
    mains: Mains,
    rpm_pi: Pi,
    next_rpm_pi: MutexCell<LargeTimestamp>,
    triac: Triac,
}

impl System {
    pub const fn new() -> Self {
        Self {
            state: MutexCell::new(SysState::PorCheck),
            ac: Ac::new(),
            adc: Adc::new(),
            speedo: Speedo::new(),
            prev_speed: MutexCell::new(MotorSpeed::zero()),
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
        let mut speedo_hz;

        match self.state.get(m) {
            SysState::PorCheck => {
                //TODO
                self.state.set(m, SysState::Syncing);
                speedo_hz = fixpt!(0);
            }
            SysState::Syncing => {
                self.prev_speed.set(m, MotorSpeed::zero());
                speedo_hz = fixpt!(0);
                if self.speedo.get_speed(m).is_some() {
                    self.state.set(m, SysState::Running);
                }
            }
            SysState::Running => {
                let speed = self.speedo.get_speed(m);
                if let Some(speed) = speed {
                    speedo_hz = speed.as_16hz();
                    self.prev_speed.set(m, speed);
                } else {
                    self.prev_speed.set(m, MotorSpeed::zero());
                    speedo_hz = self.prev_speed.get(m).as_16hz();
                }
            }
        }

        let phase_update = self.mains.run(m);
        let phase = self.mains.get_phase(m);
        let phaseref = self.mains.get_phaseref(m);

        self.adc.run(m, sp);
        let setpoint = self.adc.get_result(m, AdcChannel::Setpoint);
        let shuntdiff = self.adc.get_result(m, AdcChannel::ShuntDiff);
        let shunthi = self.adc.get_result(m, AdcChannel::ShuntHi);

        let now = timer_get_large();
        if now >= self.next_rpm_pi.get(m) {
            self.next_rpm_pi.set(m, now + RPMPI_DT);

            if let Some(setpoint) = setpoint {
                let setpoint = setpoint_to_f(setpoint);

                if setpoint <= RPM_SYNC_THRES_16HZ {
                    self.state.set(m, SysState::Syncing);
                }

                Debug::Speedo.log_fixpt(speedo_hz);

                let rpmpi_params;
                let reset_i;
                if self.state.get(m) == SysState::Running {
                    rpmpi_params = &RPMPI_PARAMS;
                    reset_i = false;
                } else {
                    speedo_hz = MotorSpeed::zero().as_16hz();
                    rpmpi_params = &RPMPI_PARAMS_SYNCING;
                    reset_i = true;
                }

                let y = self
                    .rpm_pi
                    .run(m, rpmpi_params, setpoint, speedo_hz, reset_i);
                Debug::Setpoint.log_fixpt(setpoint);
                Debug::PidY.log_fixpt(y);

                let phi_offs_ms = f_to_trig_offs(y);
                //debug(m, sp, phi_offs_ms.to_int() as i8);
                self.triac.set_phi_offs_ms(m, phi_offs_ms);
            } else {
                self.triac.shutoff(m);
            }
        }

        self.triac.run(m, phase_update, phase, phaseref);
    }
}

// vim: ts=4 sw=4 expandtab
