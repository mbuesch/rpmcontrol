use crate::{
    analog::{Ac, Adc, AdcChannel, ac_capture_get},
    debug::Debug,
    filter::Filter,
    fixpt::{Fixpt, fixpt},
    hw::mcu,
    mains::{MAINS_QUARTERWAVE_DUR, Mains, PhaseUpdate},
    mon::{Mon, MonResult},
    mutex::{MainCtx, MutexCell},
    pid::{Pid, PidParams},
    ports::PORTB,
    shutoff::{Shutoff, set_secondary_shutoff},
    speedo::Speedo,
    temp::{Temp, TempAdc},
    timer::{RelTimestamp, timer_get},
    triac::Triac,
};
use curveipo::Curve;

const RPMPI_PARAMS: PidParams = PidParams {
    kp: fixpt!(5 / 1),
    ki: fixpt!(1 / 4),
    kd: fixpt!(0),
    //kd: fixpt!(1 / 16),
};

const RPMPI_PARAMS_SYNCING: PidParams = PidParams {
    kp: fixpt!(5 / 1),
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

/// Maximum motor RPM that will trigger a hard triac inhibit.
const MOT_SOFT_LIMIT: Fixpt = rpm(24500);

/// Maximum motor RPM that will trigger a monitoring fault.
pub const MOT_HARD_LIMIT: Fixpt = rpm(25500);

const SETPOINT_FILTER_DIV: Fixpt = fixpt!(5 / 1);
const SPEED_FILTER_DIV: Fixpt = fixpt!(4 / 1);

/// Convert RPM to fixpt-16Hz
pub const fn rpm(rpm: i16) -> Fixpt {
    // rpm / 60 / 16
    Fixpt::from_fraction(rpm, 240).div(fixpt!(4))
}

/// Convert 0..0x3FF to 0..400 Hz to 0..25 16Hz
fn setpoint_to_f(adc: u16) -> Fixpt {
    Fixpt::from_fraction(adc as i16, 8) / fixpt!(128 / 25)
}

/// Clamp negative frequency to 0.
/// Convert 0..25 16Hz into pi..0 radians.
/// Convert pi..0 radians into 10..0 ms.
fn f_to_trig_offs(f: Fixpt) -> Fixpt {
    let fmin = Fixpt::from_int(0);
    let fmax = Fixpt::from_int(MAX_16HZ);
    let f = f.max(fmin);
    let f = f.min(fmax);
    let fact = fixpt!(10 / 25);
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
    mon: Mon,
    temp: Temp,
    mains: Mains,
    rpm_pid: Pid,
    mains_90deg_done: MutexCell<bool>,
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
            mon: Mon::new(),
            temp: Temp::new(),
            mains: Mains::new(),
            rpm_pid: Pid::new(),
            mains_90deg_done: MutexCell::new(false),
            triac: Triac::new(),
        }
    }

    pub fn init(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        set_secondary_shutoff(Shutoff::MachineShutoff);
        self.adc.init(m, sp);
        self.ac.init(sp);
        self.triac.set_phi_offs_shutoff(m);
    }

    pub fn run(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        let mut triac_shutoff = Shutoff::MachineRunning;

        // Read the speedo Analog Comparator.
        let ac = ac_capture_get();

        // Evaluate the speedo signal.
        self.speedo.update(m, sp, ac);

        // Get the actual motor speed and sync state.
        let mut speedo_hz;
        match self.state.get(m) {
            SysState::PorCheck => {
                //TODO turn on secondary shutoff path and turn off triac.
                // Then check that speedo=0 for one second.

                self.state.set(m, SysState::Syncing);
                speedo_hz = fixpt!(0);
                self.speed_filter.reset(m);
                triac_shutoff = Shutoff::MachineShutoff;
            }
            state @ SysState::Syncing | state @ SysState::Running => {
                if let Some(speed) = self.speedo.get_speed(m) {
                    speedo_hz = self.speed_filter.run(m, speed.as_16hz(), SPEED_FILTER_DIV);
                    if state != SysState::Running {
                        self.state.set(m, SysState::Running);
                        self.temp.init(m);
                    }
                } else if state == SysState::Running {
                    speedo_hz = self.speed_filter.get(m, SPEED_FILTER_DIV);
                } else {
                    speedo_hz = fixpt!(0);
                }
            }
        }
        if speedo_hz > MOT_SOFT_LIMIT {
            triac_shutoff = Shutoff::MachineShutoff;
        }

        // Run the ADC measurements.
        self.adc.run(m, sp);

        // Convert the setpoint to 16Hz
        let setpoint = if let Some(setpoint) = self.adc.get_result(m, AdcChannel::Setpoint) {
            setpoint_to_f(setpoint)
        } else {
            rpm(0)
        };

        // Update the mains synchronization.
        let phase_update = self.mains.run(m);

        // Check if we are at mains zero crossing + 90 degrees.
        let mut mains_90deg_trigger = false;
        if phase_update == PhaseUpdate::Changed {
            // Zero crossing.
            self.mains_90deg_done.set(m, false);
        } else if !self.mains_90deg_done.get(m)
            && let Some(time_since_zerocrossing) = self.mains.get_time_since_zerocrossing(m)
            && time_since_zerocrossing >= MAINS_QUARTERWAVE_DUR
        {
            // We are at 90 deg.
            self.mains_90deg_done.set(m, true);
            mains_90deg_trigger = true;
        }

        if mains_90deg_trigger {
            // Evaluate the temperatures.
            self.temp.run(
                m,
                TempAdc {
                    uc: self.adc.get_result(m, AdcChannel::UcTemp),
                    mot: self.adc.get_result(m, AdcChannel::MotTemp),
                },
            );

            let setpoint_filt = self.setpoint_filter.run(m, setpoint, SETPOINT_FILTER_DIV);
            if setpoint_filt <= RPM_SYNC_THRES {
                self.state.set(m, SysState::Syncing);
            }

            Debug::Speedo.log_fixpt(speedo_hz);

            // Run the RPM controller.
            let rpmpi_params;
            let reset_i;
            match self.state.get(m) {
                SysState::PorCheck => {
                    rpmpi_params = &RPMPI_PARAMS_SYNCING;
                    reset_i = true;
                    triac_shutoff = Shutoff::MachineShutoff;
                }
                SysState::Syncing => {
                    speedo_hz = SYNC_SPEEDO_SUBSTITUTE.lin_inter(setpoint_filt);
                    rpmpi_params = &RPMPI_PARAMS_SYNCING;
                    reset_i = true;
                }
                SysState::Running => {
                    rpmpi_params = &RPMPI_PARAMS;
                    reset_i = false;
                }
            }
            self.rpm_pid.set_ilim(m, RPMPI_ILIM.lin_inter(speedo_hz));
            let y = self
                .rpm_pid
                .run(m, rpmpi_params, setpoint_filt, speedo_hz, reset_i);

            Debug::Setpoint.log_fixpt(setpoint_filt);
            Debug::PidY.log_fixpt(y);

            let phi_offs_ms = f_to_trig_offs(y);
            self.triac.set_phi_offs_ms(m, phi_offs_ms);
        }

        // Temperature shutoff.
        let temp_shutoff = self.temp.get_shutoff(m);

        // Safety monitoring check.
        let mon_res = self.mon.check(m, setpoint, speedo_hz);

        // Safety shutoff.
        if mon_res == MonResult::Shutoff || temp_shutoff == Shutoff::MachineShutoff {
            triac_shutoff = Shutoff::MachineShutoff;
            set_secondary_shutoff(Shutoff::MachineShutoff);
        } else {
            set_secondary_shutoff(Shutoff::MachineRunning);
        }

        // Update the triac trigger state.
        self.triac.run(
            m,
            phase_update,
            self.mains.get_phase(m),
            self.mains.get_phaseref(m),
            triac_shutoff,
        );
    }
}

// vim: ts=4 sw=4 expandtab
