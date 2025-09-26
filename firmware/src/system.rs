use crate::{
    analog::{Ac, Adc, AdcChannel},
    debug::Debug,
    filter::Filter,
    fixpt::{Fixpt, big_fixpt, fixpt},
    hw::mcu,
    mains::{MAINS_QUARTERWAVE_DUR, Mains, PhaseUpdate},
    mon::{Mon, MonResult},
    mutex::{MainCtx, MutexCell},
    pid::{Pid, PidIlim, PidParams},
    pocheck::{PoCheck, PoState},
    ports::PORTB,
    shutoff::{Shutoff, set_secondary_shutoff},
    speedo::Speedo,
    temp::{Temp, TempAdc},
    timer::{LargeTimestamp, RelLargeTimestamp, timer_get_large},
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

const RPMPI_ILIM_NEG: Curve<Fixpt, (Fixpt, Fixpt), 4> = Curve::new([
    // (speedo, I-limit)
    (rpm!(0), fixpt!(0)),
    (rpm!(1000), fixpt!(0)),
    (rpm!(1001), fixpt!(-2)),
    (rpm!(MAX_RPM), fixpt!(-6)),
]);

const RPMPI_ILIM_POS: Curve<Fixpt, (Fixpt, Fixpt), 4> = Curve::new([
    // (speedo, I-limit)
    (rpm!(0), fixpt!(0)),
    (rpm!(1000), fixpt!(0)),
    (rpm!(1001), fixpt!(12)),
    (rpm!(MAX_RPM), fixpt!(24)),
]);

const SYNC_SPEEDO_SUBSTITUTE: Curve<Fixpt, (Fixpt, Fixpt), 2> = Curve::new([
    // (setpoint, speedo-substitute)
    (rpm!(0), rpm!(0)),
    (rpm!(1000), rpm!(800)),
]);

/// Nominal maximum motor RPM.
const MAX_RPM: i16 = 24000;

/// Nominal maximum motor speed in 16-Hz units.
const MAX_16HZ: i16 = rpm!(MAX_RPM).to_int(); // 24000/min, 400 Hz, 25 16-Hz

/// Maximum motor RPM that will trigger a hard triac inhibit.
const MOT_SOFT_LIMIT: Fixpt = rpm!(MAX_RPM + 500);

/// Maximum motor RPM that will trigger a monitoring fault.
pub const MOT_HARD_LIMIT: Fixpt = rpm!(MAX_RPM + 1500);

/// Motor speed below this threshold will trigger speedometer re-syncing.
const RPM_SYNC_THRES: Fixpt = rpm!(1000);

/// Speedometer filter divider.
const SPEED_FILTER_DIV: Fixpt = fixpt!(2 / 1);

/// Setpoint filter divider.
const SETPOINT_FILTER_DIV: Fixpt = fixpt!(5 / 1);

/// Convert RPM to fixpt-16Hz
macro_rules! rpm {
    ($rpm: expr) => {
        // rpm / 60 / 16
        const {
            use $crate::fixpt::{BigFixpt, big_fixpt};
            let rps = BigFixpt::const_from_fraction($rpm, 60);
            let hz16 = rps.const_div(big_fixpt!(16));
            hz16.downgrade()
        }
    };
}
pub(crate) use rpm;

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
    ((fmax - f) * fixpt!(2)) / fixpt!(5) // *10/25
}

#[allow(non_snake_case)]
pub struct SysPeriph {
    pub AC: mcu::AC,
    pub ADC: mcu::ADC,
}

#[allow(dead_code)]
pub fn debug_toggle() {
    PORTB.set(6, !PORTB.get(6));
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum SysState {
    /// Power-on system check.
    PoCheck = 0,
    /// Speedometer syncing.
    Syncing,
    /// Up and running.
    Running,
}

pub struct System {
    state: MutexCell<SysState>,
    pocheck: PoCheck,
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
    prev_time: MutexCell<LargeTimestamp>,
    max_rt: MutexCell<RelLargeTimestamp>,
}

impl System {
    pub const fn new() -> Self {
        Self {
            state: MutexCell::new(SysState::PoCheck),
            pocheck: PoCheck::new(),
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
            prev_time: MutexCell::new(LargeTimestamp::new()),
            max_rt: MutexCell::new(RelLargeTimestamp::new()),
        }
    }

    pub fn init(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        set_secondary_shutoff(Shutoff::MachineShutoff);
        self.pocheck.init(m);
        self.adc.init(m, sp);
        self.ac.init(sp);
        self.triac.set_phi_offs_shutoff(m);
    }

    fn meas_runtime(&self, m: &MainCtx<'_>) {
        let now = timer_get_large();
        let runtime = now - self.prev_time.get(m);
        self.prev_time.set(m, now);
        let max_rt = self.max_rt.get(m).max(runtime);
        self.max_rt.set(m, max_rt);
        Debug::MaxRt.log_rel_large_timestamp(max_rt);
    }

    pub fn run(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        let mut triac_shutoff = Shutoff::MachineRunning;

        self.meas_runtime(m);

        // Evaluate the speedo signal.
        self.speedo.update(m);

        // Run the power-on check state machine.
        if self.state.get(m) == SysState::PoCheck {
            match self.pocheck.run(m, self.speedo.get_speed(m)) {
                PoState::CheckIdle
                | PoState::CheckSecondaryShutoff
                | PoState::CheckPrimaryShutoff
                | PoState::Error => (),
                PoState::DoneOk => {
                    self.triac.set_phi_offs_shutoff(m);
                    self.state.set(m, SysState::Syncing);
                }
            }
        }

        // Get the actual motor speed and sync state.
        let mut speedo_hz;
        match self.state.get(m) {
            SysState::PoCheck => {
                speedo_hz = fixpt!(0);
                self.speed_filter.reset(m);
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
            rpm!(0)
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

            Debug::Speedo.log_fixpt(speedo_hz);

            // Run the RPM controller.
            let rpmpi_params;
            let reset_i;
            match self.state.get(m) {
                SysState::PoCheck => {
                    rpmpi_params = &RPMPI_PARAMS_SYNCING;
                    reset_i = true;
                }
                SysState::Syncing => {
                    speedo_hz = SYNC_SPEEDO_SUBSTITUTE.lin_inter(setpoint_filt);
                    rpmpi_params = &RPMPI_PARAMS_SYNCING;
                    reset_i = true;
                }
                SysState::Running => {
                    if setpoint_filt <= RPM_SYNC_THRES {
                        self.state.set(m, SysState::Syncing);
                    }
                    rpmpi_params = &RPMPI_PARAMS;
                    reset_i = false;
                }
            }
            let y = self.rpm_pid.run(
                m,
                rpmpi_params,
                &PidIlim {
                    pos: RPMPI_ILIM_POS.lin_inter(speedo_hz),
                    neg: RPMPI_ILIM_NEG.lin_inter(speedo_hz),
                },
                setpoint_filt,
                speedo_hz,
                reset_i,
            );

            Debug::Setpoint.log_fixpt(setpoint_filt);
            Debug::PidY.log_fixpt(y);

            let phi_offs_ms = f_to_trig_offs(y);
            self.triac.set_phi_offs_ms(m, phi_offs_ms);
        }

        // Temperature shutoff.
        let temp_shutoff = self.temp.get_shutoff(m);

        // Safety monitoring check.
        let mon_res = self.mon.check(m, setpoint, speedo_hz);

        // Get power-on check shutoff paths.
        let pocheck_secondary_shutoff = self.pocheck.get_secondary_shutoff(m);
        if self.pocheck.get_triac_shutoff(m) == Shutoff::MachineShutoff {
            triac_shutoff = Shutoff::MachineShutoff;
        }
        if self.state.get(m) == SysState::PoCheck {
            if let Some(phi_offs_ms) = self.pocheck.get_triac_phi_offs_ms(m) {
                self.triac.set_phi_offs_ms(m, phi_offs_ms);
            } else {
                self.triac.set_phi_offs_shutoff(m);
            }
        }

        // Secondary shutoff path.
        if pocheck_secondary_shutoff == Shutoff::MachineShutoff {
            set_secondary_shutoff(Shutoff::MachineShutoff);
        } else if mon_res == MonResult::Shutoff || temp_shutoff == Shutoff::MachineShutoff {
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
