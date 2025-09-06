// -*- coding: utf-8 -*-

use anyhow::{self as ah, Context as _, format_err as err};
use std::{
    collections::VecDeque,
    sync::mpsc,
    time::{Duration, Instant},
};

const BAUD: u32 = 19_200;

type SerBuf = [u8; 3];

#[derive(Debug, Clone)]
pub enum SerDat {
    Speedo(Instant, f64),
    SpeedoStatus(Instant, u16),
    Setpoint(Instant, f64),
    PidY(Instant, f64),
    MonDebounce(Instant, u16),
    TempMot(Instant, f64),
    TempUc(Instant, f64),
    Sync,
}

const FIXPT_SHIFT: usize = 8;

fn hz_to_rpm(val: f64) -> f64 {
    val * 60.0
}

fn hz16_to_hz(val: f64) -> f64 {
    val * 16.0
}

fn double_celsius_to_celsius(val: f64) -> f64 {
    val * 2.0
}

fn fixpt_to_f64(val: u16) -> f64 {
    (val as f64) / ((1 << FIXPT_SHIFT) as f64)
}

fn fixpt_to_rpm(val: u16) -> f64 {
    hz_to_rpm(hz16_to_hz(fixpt_to_f64(val)))
}

fn fixpt_to_celsius(val: u16) -> f64 {
    double_celsius_to_celsius(fixpt_to_f64(val))
}

impl SerDat {
    pub fn parse(buf: &SerBuf) -> ah::Result<SerDat> {
        let now = Instant::now();
        let val = u16::from_le_bytes([buf[1], buf[2]]);
        match buf[0] {
            0 => Ok(SerDat::Speedo(now, fixpt_to_rpm(val))),
            1 => Ok(SerDat::SpeedoStatus(now, val)),
            2 => Ok(SerDat::Setpoint(now, fixpt_to_rpm(val))),
            3 => Ok(SerDat::PidY(now, fixpt_to_rpm(val))),
            4 => Ok(SerDat::MonDebounce(now, val)),
            5 => Ok(SerDat::TempMot(now, fixpt_to_celsius(val))),
            6 => Ok(SerDat::TempUc(now, fixpt_to_celsius(val))),
            0xFF => Ok(SerDat::Sync),
            cmd => Err(err!("SerBuf::parse: Unknown command 0x{cmd:02X}")),
        }
    }
}

fn process_one(
    serial: &mut Box<dyn serialport::SerialPort>,
    notify_tx: &mpsc::Sender<SerDat>,
) -> ah::Result<()> {
    let mut buf: SerBuf = Default::default();
    serial.read_exact(&mut buf).context("Serial port read")?;
    let dat = SerDat::parse(&buf).context("Parse SerBuf")?;
    notify_tx.send(dat).context("Send SerDat")?;
    Ok(())
}

fn synchronize(serial: &mut Box<dyn serialport::SerialPort>) -> ah::Result<()> {
    let mut sync = VecDeque::new();
    let mut count = 0;
    loop {
        if count > 128 {
            return Err(err!("Serial port sync failed"));
        }
        let mut buf = [0u8];
        serial.read_exact(&mut buf).context("Serial port read")?;
        sync.push_back(buf[0]);
        if sync.len() >= 3 {
            if sync[0] == 0xFF && sync[1] == 0xFF && sync[2] == 0xFF {
                break;
            }
            sync.pop_front();
            count += 1;
        }
    }
    Ok(())
}

pub fn run_serial(port: &Option<String>, notify_tx: &mpsc::Sender<SerDat>) -> ah::Result<()> {
    let port = port.as_deref().unwrap_or("/dev/ttyUSB1");
    let mut serial = serialport::new(port, BAUD)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .flow_control(serialport::FlowControl::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(500))
        .open()
        .context("Open serial port")?;

    // Main serial communication loop.
    let mut debounce = 0_usize;
    synchronize(&mut serial)?;
    loop {
        match process_one(&mut serial, notify_tx) {
            Ok(_) => {
                debounce = debounce.saturating_sub(1);
            }
            Err(e) => {
                debounce = debounce.saturating_add(3);
                if debounce >= 15 {
                    return Err(e);
                }
                synchronize(&mut serial)?;
            }
        }
    }
}

// vim: ts=4 sw=4 expandtab
