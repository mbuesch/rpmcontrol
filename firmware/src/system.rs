// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

use crate::{
    analog::{Ac, Adc, AdcChannel},
    calibration::{
        rpm_pid::{RPMPID_ILIM_NEG, RPMPID_ILIM_POS, RPMPID_PARAMS, RPMPID_PARAMS_SYNCING},
        setpoint::{SP_MIN_CUTOFF, SP_STEPS, SP_SYNC_THRES},
        speedo::{NO_SPEED_TIMEOUT, SPEED_FILTER_DIV, SYNC_SPEEDO_SUBSTITUTE},
        system::{MAX_RPM, MOT_SOFT_LIMIT, STARTUP_DELAY},
    },
    debug::Debug,
    filter::Filter,
    freq::Freq,
    hw::mcu,
    mains::{MAINS_QUARTERWAVE_DUR, Mains, PhaseUpdate},
    mon::Mon,
    mon_pocheck::{PoCheck, PoState},
    pid::{Pid, PidIlim},
    shutoff::{Shutoff, set_secondary_shutoff},
    snap::Snap,
    speedo::{MotorSpeed, Speedo},
    temp::{Temp, TempAdc},
    timer::{LargeTimestamp, timer_get_large},
    triac::Triac,
};
use avr_context::{MainCtx, MainCtxCell};
use avr_q::{Q7p8, q7p8, q15p8};

#[cfg(feature = "debug")]
use crate::{
    hw::interrupt,
    ports::{PORTB, PortOps as _},
};

macro_rules! rpm {
    ($rpm: expr) => {
        // rpm / 60 / 4
        const {
            use avr_q::q15p8;
            use $crate::freq::Freq;

            const RPM: i16 = $rpm;

    let rps = q15p8!(const RPM / 60);
            let freq = rps.const_div(Freq::FACT.to_q15p8());

            Freq(freq.to_q7p8())
        }
    };
}
pub(crate) use rpm;

/// Convert 0..0x3FF to 0..400 Hz to 0..x Freq
fn setpoint_to_f(adc: u16) -> Freq {
    const MAX_FREQ: i16 = rpm!(MAX_RPM).0.to_int() as i16;

    let adc = adc as i16;
    let freq = q15p8!(adc) / q15p8!(0x3FF / MAX_FREQ);

    Freq(freq.to_q7p8())
}

/// Clamp negative frequency to 0.
/// Convert 0..x Freq into pi..0 radians.
/// Convert pi..0 radians into 10..0 ms.
fn f_to_trig_offs(f: Freq) -> Q7p8 {
    const MAX_FREQ: i16 = rpm!(MAX_RPM).0.to_int() as i16;

    let fmin = Q7p8::from_int(0);
    let fmax = rpm!(MAX_RPM).0;
    let f = f.0;
    let f = f.max(fmin);
    let f = f.min(fmax);

    (fmax - f) * q7p8!(const 10 / MAX_FREQ)
}

/// Toggle the debug pin.
pub fn debug_toggle() {
    #[cfg(feature = "debug")]
    interrupt::free(|cs| PORTB.toggle(cs, 6));
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum SysState {
    /// Early startup delay.
    Startup = 0,
    /// Power-on system check.
    PoCheck,
    /// Speedometer syncing.
    Syncing,
    /// Up and running.
    Running,
}

pub struct System {
    startup_delay_timeout: MainCtxCell<LargeTimestamp>,
    state: MainCtxCell<SysState>,
    mon: Mon,
    mon_pocheck: PoCheck,
    ac: Ac,
    adc: Adc,
    setpoint_snap: Snap<Freq>,
    speedo: Speedo,
    speed_filter: Filter,
    prev_valid_speed: MainCtxCell<LargeTimestamp>,
    temp: Temp,
    mains: Mains,
    rpm_pid: Pid,
    mains_90deg_done: MainCtxCell<bool>,
    triac: Triac,
}

impl System {
    pub const fn new() -> Self {
        Self {
            startup_delay_timeout: MainCtxCell::new(LargeTimestamp::new()),
            state: MainCtxCell::new(SysState::Startup),
            mon: Mon::new(),
            mon_pocheck: PoCheck::new(),
            ac: Ac::new(),
            adc: Adc::new(),
            setpoint_snap: Snap::new(Freq(q7p8!(const 0))),
            speedo: Speedo::new(),
            speed_filter: Filter::new(),
            prev_valid_speed: MainCtxCell::new(LargeTimestamp::new()),
            temp: Temp::new(),
            mains: Mains::new(),
            rpm_pid: Pid::new(),
            mains_90deg_done: MainCtxCell::new(false),
            triac: Triac::new(),
        }
    }

    /// System initialization.
    #[allow(non_snake_case)]
    pub fn init(&self, m: &MainCtx<'_>, ADC: &mcu::ADC, AC: &mcu::AC) {
        // Set all shutoff paths.
        set_secondary_shutoff(Shutoff::MachineShutoff);
        self.triac.set_phi_offs_shutoff(m);

        self.adc.init(m, ADC);
        self.ac.init(AC);

        self.startup_delay_timeout
            .set(m, timer_get_large() + STARTUP_DELAY);
    }

    /// Enter `SysState::PoCheck` for the first time.
    fn init_pocheck(&self, m: &MainCtx<'_>) {
        let now = timer_get_large();

        self.mains.init(m, now);
        self.mon_pocheck.init(m, now);
        self.speedo.init(m, now);
    }

    /// Enter `SysState::Syncing` for the first time.
    fn init_syncing(&self, m: &MainCtx<'_>) {
        let now = timer_get_large();

        self.mon.init(m, now);
        self.temp.init(m, now);
    }

    /// Run the initial startup delay.
    fn run_startup(&self, m: &MainCtx<'_>) {
        let now = timer_get_large();

        // On startup delay timeout, continue to power-on-check.
        if now > self.startup_delay_timeout.get(m) {
            self.state.set(m, SysState::PoCheck);
            self.init_pocheck(m);
        }
    }

    /// Run the power-on-check.
    fn run_pocheck(&self, m: &MainCtx<'_>, speed: Option<MotorSpeed>) -> Shutoff {
        // Run the power-on-check state machine.
        match self.mon_pocheck.run(m, speed) {
            PoState::CheckIdle | PoState::CheckSecondaryShutoff | PoState::CheckPrimaryShutoff => {
                // Power-on-check is still running.

                // Get power-on-check triac offset override.
                if let Some(phi_offs_ms) = self.mon_pocheck.get_triac_phi_offs_ms(m) {
                    self.triac.set_phi_offs_ms(m, phi_offs_ms);
                } else {
                    self.triac.set_phi_offs_shutoff(m);
                }
            }
            PoState::Error => {
                // Power-on-check detected an error.

                // Ensure triac is turned off.
                self.triac.set_phi_offs_shutoff(m);
            }
            PoState::DoneOk => {
                // Power-on-check finished successfully.

                // Ensure triac is turned off.
                self.triac.set_phi_offs_shutoff(m);

                // Go to next system state.
                self.state.set(m, SysState::Syncing);
                // Enter syncing for the first time.
                self.init_syncing(m);
            }
        }

        // Set the secondary shutoff according to what the power-on-check wants.
        set_secondary_shutoff(self.mon_pocheck.get_secondary_shutoff(m));

        // Set the primary shutoff according to what the power-on-check wants.
        self.mon_pocheck.get_triac_shutoff(m)
    }

    /// The system is in normal state (Syncing or Running).
    fn run_normal(
        &self,
        m: &MainCtx<'_>,
        phase_update: PhaseUpdate,
        speed: Option<MotorSpeed>,
    ) -> Shutoff {
        let now = timer_get_large();
        let mut triac_shutoff = Shutoff::MachineRunning;

        // Interpret and filter the motor speed.
        let speed_filt = if let Some(speed) = speed {
            // We are sync'd now. Leave sync state.
            self.state.set(m, SysState::Running);
            self.prev_valid_speed.set(m, now);
            // Filter the speed.
            Freq(
                self.speed_filter
                    .run(m, speed.as_freq().0, SPEED_FILTER_DIV),
            )
        } else if self.state.get(m) == SysState::Running
            && now - self.prev_valid_speed.get(m) < NO_SPEED_TIMEOUT
        {
            // No valid speed measurement.
            // We had a valid speed measurement recently. Use it.
            Freq(self.speed_filter.get(m))
        } else {
            // Drop out of running state.
            self.state.set(m, SysState::Syncing);
            self.speed_filter.reset(m);
            Freq(q7p8!(const 0))
        };
        // Raw speedo signal is considered ok if we have a valid speed measurement.
        let raw_speedo_signal_is_ok = speed.is_some();

        // If the motor is too fast, turn the triac off.
        if speed_filt > MOT_SOFT_LIMIT {
            triac_shutoff = Shutoff::MachineShutoff;
        }

        // Convert the setpoint to frequency.
        let setpoint = if let Some(setpoint) = self.adc.get_result(m, AdcChannel::Setpoint) {
            self.setpoint_snap.update(
                m,
                rpm!(0),                  // min
                rpm!(MAX_RPM),            // max
                rpm!(MAX_RPM / SP_STEPS), // hyst
                setpoint_to_f(setpoint),
            )
        } else {
            rpm!(0)
        };

        // If the setpoint is below the minimum cutoff, turn the triac off.
        if setpoint < SP_MIN_CUTOFF {
            triac_shutoff = Shutoff::MachineShutoff;
        }

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

            if setpoint <= SP_SYNC_THRES {
                self.state.set(m, SysState::Syncing);
            }

            // Run the RPM controller.
            let pid_speed;
            let pid_params;
            let pid_reset_i;
            match self.state.get(m) {
                SysState::Startup | SysState::PoCheck | SysState::Syncing => {
                    pid_speed = SYNC_SPEEDO_SUBSTITUTE.lin_inter(setpoint);
                    pid_params = &RPMPID_PARAMS_SYNCING;
                    pid_reset_i = true;
                }
                SysState::Running => {
                    pid_speed = speed_filt;
                    pid_params = &RPMPID_PARAMS;
                    pid_reset_i = false;
                }
            }
            let pid_y = Freq(self.rpm_pid.run(
                m,
                pid_params,
                &PidIlim {
                    pos: RPMPID_ILIM_POS.lin_inter(speed_filt.0),
                    neg: RPMPID_ILIM_NEG.lin_inter(speed_filt.0),
                },
                setpoint.0,
                pid_speed.0,
                pid_reset_i,
            ));

            Debug::Setpoint.log_fixpt(setpoint.0);
            Debug::Speedo.log_fixpt(speed_filt.0);
            Debug::PidY.log_fixpt(pid_y.0);

            let phi_offs_ms = f_to_trig_offs(pid_y);
            self.triac.set_phi_offs_ms(m, phi_offs_ms);
        }

        // Temperature shutoff.
        let mut safety_shutoff = self.temp.get_shutoff(m);

        // Safety monitoring check.
        safety_shutoff |= self.mon.check(
            m,
            setpoint,
            speed_filt,
            raw_speedo_signal_is_ok,
            mains_90deg_trigger,
        );

        // Secondary shutoff path.
        if safety_shutoff == Shutoff::MachineShutoff {
            // Safety shutoff: Activate both shutoff paths.
            triac_shutoff = Shutoff::MachineShutoff;
            set_secondary_shutoff(Shutoff::MachineShutoff);
        } else {
            // Normal operation.
            set_secondary_shutoff(Shutoff::MachineRunning);
        }

        triac_shutoff
    }

    /// Main loop.
    #[allow(non_snake_case)]
    pub fn run(&self, m: &MainCtx<'_>, ADC: &mcu::ADC) {
        self.mon.meas_main_runtime(m);

        let state = self.state.get(m);
        if state == SysState::Startup {
            // Startup delay.
            self.run_startup(m);
        } else {
            // Update the mains synchronization.
            let phase_update = self.mains.run(m);

            // Run the ADC measurements.
            self.adc.run(m, ADC);

            // Evaluate the speedo signal.
            let speed = self.speedo.run(m);

            let triac_shutoff = match state {
                SysState::Startup => Shutoff::MachineShutoff,
                SysState::PoCheck => self.run_pocheck(m, speed),
                SysState::Syncing | SysState::Running => self.run_normal(m, phase_update, speed),
            };

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
}

// vim: ts=4 sw=4 expandtab
