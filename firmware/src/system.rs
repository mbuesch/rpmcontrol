use crate::{
    analog::{Ac, Adc, AdcChannel, ac_capture_get},
    debug::Debug,
    filter::Filter,
    fixpt::{Fixpt, fixpt},
    hw::mcu,
    mains::{Mains, PhaseUpdate},
    mutex::{MainCtx, MutexCell},
    pid::{Pid, PidParams},
    ports::PORTB,
    speedo::Speedo,
    timer::{RelLargeTimestamp, RelTimestamp, timer_get},
    triac::Triac,
};
use curveipo::Curve;

/// The position of the PID calculation, relative to mains zero crossing.
const RPMPID_CALC_POS: RelLargeTimestamp = RelLargeTimestamp::from_millis(5);

const RPMPI_PARAMS: PidParams = PidParams {
    kp: fixpt!(5 / 2),
    ki: fixpt!(1 / 8),
    kd: fixpt!(1 / 16),
};

const RPMPI_PARAMS_SYNCING: PidParams = PidParams {
    kp: fixpt!(5 / 2),
    ki: fixpt!(0),
    kd: fixpt!(0),
};

const RPMPI_ILIM: Curve<Fixpt, (Fixpt, Fixpt), 4> = Curve::new([
    // (speedo, I-limit)
    (rpm(0), fixpt!(0)),
    (rpm(1000), fixpt!(0)),
    (rpm(1001), fixpt!(12)),
    (rpm(24000), fixpt!(24)),
]);

const SYNC_SPEEDO_SUBSTITUTE: Curve<Fixpt, (Fixpt, Fixpt), 2> = Curve::new([
    // (setpoint, speedo-substitute)
    (rpm(0), rpm(0)),
    (rpm(1000), rpm(800)),
]);

const RPM_SYNC_THRES: Fixpt = rpm(1000);

const MAX_16HZ: i16 = rpm(24000).to_int(); // 24000/min, 400 Hz, 25 16-Hz

/// Absolute maximum motor RPM that will trigger a hard triac inhibit.
const MOT_LIMIT: Fixpt = rpm(24500);

const SETPOINT_FILTER_DIV: Fixpt = fixpt!(5 / 1);
const SPEED_FILTER_DIV: Fixpt = fixpt!(4 / 1);

/// Convert RPM to fixpt-16Hz
const fn rpm(rpm: i16) -> Fixpt {
    // rpm / 60 / 16
    Fixpt::from_fraction(rpm, 240).div(fixpt!(4))
}

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

#[allow(dead_code)]
pub fn debug_toggle() {
    PORTB.set(6, !PORTB.get(6));
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
    setpoint_filter: Filter,
    speedo: Speedo,
    speed_filter: Filter,
    mains: Mains,
    rpm_pid: Pid,
    rpm_pid_calcd: MutexCell<bool>,
    triac: Triac,
}

impl System {
    pub const fn new() -> Self {
        Self {
            state: MutexCell::new(SysState::PorCheck),
            ac: Ac::new(),
            adc: Adc::new(),
            setpoint_filter: Filter::new(),
            speedo: Speedo::new(),
            speed_filter: Filter::new(),
            mains: Mains::new(),
            rpm_pid: Pid::new(),
            rpm_pid_calcd: MutexCell::new(false),
            triac: Triac::new(),
        }
    }

    pub fn init(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        self.adc.init(m, sp);
        self.ac.init(sp);
        self.triac.set_phi_offs_shutoff(m);
    }

    pub fn run(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        let mut triac_shutoff = false;

        let ac = ac_capture_get();

        self.speedo.update(m, sp, ac);
        let mut speedo_hz;

        match self.state.get(m) {
            SysState::PorCheck => {
                //TODO
                self.state.set(m, SysState::Syncing);
                speedo_hz = fixpt!(0);
                self.speed_filter.reset(m);
                triac_shutoff = true;
            }
            SysState::Syncing => {
                speedo_hz = fixpt!(0);
                if self.speedo.get_speed(m).is_some() {
                    self.state.set(m, SysState::Running);
                }
            }
            SysState::Running => {
                let speed = self.speedo.get_speed(m);
                if let Some(speed) = speed {
                    speedo_hz = self.speed_filter.run(m, speed.as_16hz(), SPEED_FILTER_DIV);
                } else {
                    speedo_hz = self.speed_filter.get(m, SPEED_FILTER_DIV);
                }
            }
        }
        if speedo_hz > MOT_LIMIT {
            triac_shutoff = true;
        }

        // Run the ADC measurements.
        self.adc.run(m, sp);

        // Update the mains synchronization.
        let phase_update = self.mains.run(m);

        // Check if we need to run the controller.
        let mut run_pid = false;
        if phase_update == PhaseUpdate::Changed {
            self.rpm_pid_calcd.set(m, false);
        } else if !self.rpm_pid_calcd.get(m)
            && let Some(time_since_zerocrossing) = self.mains.get_time_since_zerocrossing(m)
            && time_since_zerocrossing >= RPMPID_CALC_POS
        {
            self.rpm_pid_calcd.set(m, true);
            run_pid = true;
        }

        // Run the controller.
        if run_pid {
            let setpoint = self.adc.get_result(m, AdcChannel::Setpoint);
            if let Some(setpoint) = setpoint {
                let setpoint = setpoint_to_f(setpoint);
                let setpoint = self.setpoint_filter.run(m, setpoint, SETPOINT_FILTER_DIV);

                if setpoint <= RPM_SYNC_THRES {
                    self.state.set(m, SysState::Syncing);
                }

                Debug::Speedo.log_fixpt(speedo_hz);

                let rpmpi_params;
                let reset_i;
                if self.state.get(m) == SysState::Running {
                    rpmpi_params = &RPMPI_PARAMS;
                    reset_i = false;
                } else {
                    speedo_hz = SYNC_SPEEDO_SUBSTITUTE.lin_inter(setpoint);
                    rpmpi_params = &RPMPI_PARAMS_SYNCING;
                    reset_i = true;
                }
                self.rpm_pid.set_ilim(m, RPMPI_ILIM.lin_inter(speedo_hz));

                let y = self
                    .rpm_pid
                    .run(m, rpmpi_params, setpoint, speedo_hz, reset_i);
                Debug::Setpoint.log_fixpt(setpoint);
                Debug::PidY.log_fixpt(y);

                let phi_offs_ms = f_to_trig_offs(y);
                self.triac.set_phi_offs_ms(m, phi_offs_ms);
            } else {
                triac_shutoff = true;
            }
        }

        // Update the triac state.
        let phase = self.mains.get_phase(m);
        let phaseref = self.mains.get_phaseref(m);
        self.triac
            .run(m, phase_update, phase, phaseref, triac_shutoff);
    }
}

// vim: ts=4 sw=4 expandtab
