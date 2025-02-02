// -*- coding: utf-8 -*-

use anyhow::{self as ah, Context as _};
use std::{sync::mpsc, time::Duration};

const BAUD: u32 = 19_200;

pub type SerBuf = [u8; 3];

pub fn run_serial(port: &Option<String>, notify_tx: &mpsc::Sender<SerBuf>) -> ah::Result<()> {
    let port = port.as_deref().unwrap_or("/dev/ttyUSB0");
    let mut serial = serialport::new(port, BAUD)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .flow_control(serialport::FlowControl::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(500))
        .open()
        .context("Open serial port")?;

    loop {
        let mut buf: SerBuf = Default::default();
        serial.read_exact(&mut buf).context("Serial port read")?;
        println!("{buf:?}");
    }
}

// vim: ts=4 sw=4 expandtab
