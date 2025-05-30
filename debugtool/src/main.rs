// -*- coding: utf-8 -*-

#![forbid(unsafe_code)]

mod diagram_area;
mod main_window;
mod serial;

use crate::serial::{SerDat, run_serial};
use anyhow as ah;
use clap::Parser;
use gtk4::{self as gtk, gio, prelude::*};
use std::{rc::Rc, sync::mpsc, thread, time::Duration};

#[derive(Parser, Debug)]
struct Opts {
    port: Option<String>,
}

fn app_fn(app: &gtk::Application, ser_notify_rx: Rc<mpsc::Receiver<SerDat>>) {
    let window = main_window::MainWindow::new(app, ser_notify_rx).unwrap();
    window.borrow().application_window().present();
}

fn main() -> ah::Result<()> {
    let opts = Opts::parse();

    let (ser_notify_tx, ser_notify_rx) = mpsc::channel();

    thread::scope(|s| {
        s.spawn(|| {
            loop {
                if let Err(e) = run_serial(&opts.port, &ser_notify_tx) {
                    eprintln!("Serial error: {e:?}");
                }
                thread::sleep(Duration::from_millis(1000));
            }
        });

        let ser_notify_rx = Rc::new(ser_notify_rx);

        let app = gtk::Application::builder()
            .flags(gio::ApplicationFlags::FLAGS_NONE)
            .application_id("ch.bues.rpmcontrol.debugtool")
            .build();
        app.connect_activate(move |app| app_fn(app, Rc::clone(&ser_notify_rx)));
        let args: Vec<&str> = vec![];
        std::process::exit(app.run_with_args(&args).into())
    });
    Ok(())
}

// vim: ts=4 sw=4 expandtab
