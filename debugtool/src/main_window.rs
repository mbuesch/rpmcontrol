// -*- coding: utf-8 -*-

use crate::{diagram_area::DiagramArea, serial::SerBuf};
use anyhow as ah;
use gtk4::{self as gtk, glib, prelude::*};
use plotters::prelude::*;
use plotters_cairo::CairoBackend;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{mpsc, Arc},
    time::Duration,
};

fn draw(backend: CairoBackend) {
    let area = backend.into_drawing_area();
    area.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&area)
        .caption("rpmcontrol", ("sans-serif", 12).into_font())
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0..20, 0..3000)
        .unwrap();
    chart
        .configure_mesh()
        .x_desc("time")
        .y_desc("RPM")
        .draw()
        .unwrap();
    chart
        .draw_series(LineSeries::new((0..10).map(|x| (x, x * x)), &RED))
        .unwrap()
        .label("speedo")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));
    chart
        .draw_series(LineSeries::new((5..20).map(|x| (x, 2 * x)), &BLUE))
        .unwrap()
        .label("setpoint")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));
    chart
        .configure_series_labels()
        .background_style(&WHITE)
        .border_style(&BLACK)
        .draw()
        .unwrap();
}

fn periodic_work(ser_notify_rx: Arc<mpsc::Receiver<SerBuf>>) {
    println!("XXX");
}

pub struct MainWindow {
    appwindow: gtk::ApplicationWindow,
    diagram_area: Rc<RefCell<DiagramArea>>,
}

impl MainWindow {
    pub fn new(
        app: &gtk::Application,
        ser_notify_rx: Arc<mpsc::Receiver<SerBuf>>,
    ) -> ah::Result<Rc<RefCell<Self>>> {
        let builder = gtk::Builder::from_string(include_str!("main_window.glade"));

        let appwindow: gtk::ApplicationWindow = builder.object("main_window").unwrap();
        appwindow.set_application(Some(app));
        appwindow.set_title(Some("rpmcontrol debug"));
        appwindow.set_default_size(800, 600);

        let diagram_area = DiagramArea::new(&builder, "drawing_area", draw);

        glib::source::timeout_add_local(Duration::from_millis(1000), {
            move || {
                periodic_work(Arc::clone(&ser_notify_rx));
                glib::ControlFlow::Continue
            }
        });

        Ok(Rc::new(RefCell::new(Self {
            appwindow,
            diagram_area,
        })))
    }

    pub fn application_window(&self) -> &gtk::ApplicationWindow {
        &self.appwindow
    }
}

// vim: ts=4 sw=4 expandtab
