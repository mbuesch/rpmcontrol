// -*- coding: utf-8 -*-

use anyhow::{self as ah, format_err as err, Context as _};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
    collections::VecDeque,
};

const BAUD: u32 = 19_200;

type SerBuf = [u8; 3];

#[derive(Debug, Clone)]
pub enum SerDat {
    Speedo(Instant, f64),
    Setpoint(Instant, f64),
    PidY(Instant, f64),
    Sync(Instant),
}

const FIXPT_SHIFT: usize = 8;

fn fixpt_to_f64(val: u16) -> f64 {
    (val as f64) / ((1 << FIXPT_SHIFT) as f64)
}

impl SerDat {
    pub fn parse(buf: &SerBuf) -> ah::Result<SerDat> {
        let now = Instant::now();
        let val = u16::from_le_bytes([buf[1], buf[2]]);
        match buf[0] {
            0 => Ok(SerDat::Speedo(now, fixpt_to_f64(val))),
            1 => Ok(SerDat::Setpoint(now, fixpt_to_f64(val))),
            2 => Ok(SerDat::PidY(now, fixpt_to_f64(val))),
            0xFF => Ok(SerDat::Sync(now)),
            cmd => Err(err!("SerBuf::parse: Unknown command 0x{cmd:02X}")),
        }
    }
}

pub fn run_serial(port: &Option<String>, notify_tx: &mpsc::Sender<SerDat>) -> ah::Result<()> {
    let port = port.as_deref().unwrap_or("/dev/ttyUSB0");
    let mut serial = serialport::new(port, BAUD)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .flow_control(serialport::FlowControl::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(500))
        .open()
        .context("Open serial port")?;

    // Synchronize.
    {
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
    }

    // Main serial communication loop.
    loop {
        let mut buf: SerBuf = Default::default();
        serial.read_exact(&mut buf).context("Serial port read")?;
        let dat = SerDat::parse(&buf).context("Parse SerBuf")?;
        notify_tx.send(dat).context("Send SerDat")?;
    }
}

// vim: ts=4 sw=4 expandtab
