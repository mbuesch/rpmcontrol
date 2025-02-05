// -*- coding: utf-8 -*-

use crate::{diagram_area::DiagramArea, serial::SerDat};
use anyhow as ah;
use gtk4::{self as gtk, glib, prelude::*};
use plotters::prelude::*;
use plotters_cairo::CairoBackend;
use rand::prelude::*;
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

struct DiagramData {
    reference: Instant,
    speedo: VecDeque<(f64, f64)>,
    setpoint: VecDeque<(f64, f64)>,
    pid_y: VecDeque<(f64, f64)>,
}

const T_INTERVAL: Duration = Duration::from_millis(10000);

impl DiagramData {
    pub fn new() -> Self {
        Self {
            reference: Instant::now(),
            speedo: VecDeque::new(),
            setpoint: VecDeque::new(),
            pid_y: VecDeque::new(),
        }
    }

    fn timestamp(&self, t: Instant) -> f64 {
        t.duration_since(self.reference).as_secs_f64()
    }

    fn prune_items(buf: &mut VecDeque<(f64, f64)>, age_thres: f64) {
        while let Some((t, _)) = buf.front() {
            if *t < age_thres {
                buf.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn add(&mut self, dat: SerDat) {
        let now = Instant::now();
        let age_thres = self.timestamp(now - T_INTERVAL);
        match dat {
            SerDat::Speedo(t, val) => self.speedo.push_back((self.timestamp(t), val)),
            SerDat::Setpoint(t, val) => self.setpoint.push_back((self.timestamp(t), val)),
            SerDat::PidY(t, val) => self.pid_y.push_back((self.timestamp(t), val)),
            SerDat::Sync(_) => (),
        }
        Self::prune_items(&mut self.speedo, age_thres);
        Self::prune_items(&mut self.setpoint, age_thres);
        Self::prune_items(&mut self.pid_y, age_thres);
    }
}

fn draw(backend: CairoBackend, diagram_data: Rc<RefCell<DiagramData>>) {
    let area = backend.into_drawing_area();
    area.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&area)
        .caption("rpmcontrol", ("sans-serif", 12).into_font())
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0.0..20.0, 0.0..3000.0)
        .unwrap();
    chart
        .configure_mesh()
        .x_desc("time")
        .y_desc("RPM")
        .draw()
        .unwrap();
    /*
    chart
        .draw_series(LineSeries::new((0..10).map(|x| (x, x * x)), &RED))
        .unwrap()
        .label("speedo")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));*/
    let mut rng = rand::rng();
    let fac: f64 = rng.random_range(1.0..3.0);
    chart
        .draw_series(LineSeries::new(
            (5..500).map(|x| (x as f64 / 50.0, x as f64 * fac)),
            &BLUE,
        ))
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

fn periodic_work(
    ser_rx: Arc<mpsc::Receiver<SerDat>>,
    diagram_area: Rc<RefCell<DiagramArea>>,
    diagram_data: Rc<RefCell<DiagramData>>,
) {
    {
        let mut diagram_data = diagram_data.borrow_mut();
        for dat in ser_rx.try_iter() {
            diagram_data.add(dat);
        }
    }
    diagram_area.borrow().redraw();
}

pub struct MainWindow {
    appwindow: gtk::ApplicationWindow,
    diagram_area: Rc<RefCell<DiagramArea>>,
    diagram_data: Rc<RefCell<DiagramData>>,
}

impl MainWindow {
    pub fn new(
        app: &gtk::Application,
        ser_rx: Arc<mpsc::Receiver<SerDat>>,
    ) -> ah::Result<Rc<RefCell<Self>>> {
        let builder = gtk::Builder::from_string(include_str!("main_window.glade"));

        let appwindow: gtk::ApplicationWindow = builder.object("main_window").unwrap();
        appwindow.set_application(Some(app));
        appwindow.set_title(Some("rpmcontrol debug"));
        appwindow.set_default_size(800, 600);

        let diagram_data = Rc::new(RefCell::new(DiagramData::new()));
        let diagram_area = DiagramArea::new(&builder, "drawing_area", {
            let diagram_data = Rc::clone(&diagram_data);
            move |backend| draw(backend, Rc::clone(&diagram_data))
        });

        glib::source::timeout_add_local(Duration::from_millis(100), {
            let diagram_area = Rc::clone(&diagram_area);
            let diagram_data = Rc::clone(&diagram_data);
            move || {
                periodic_work(
                    Arc::clone(&ser_rx),
                    Rc::clone(&diagram_area),
                    Rc::clone(&diagram_data),
                );
                glib::ControlFlow::Continue
            }
        });

        Ok(Rc::new(RefCell::new(Self {
            appwindow,
            diagram_area,
            diagram_data,
        })))
    }

    pub fn application_window(&self) -> &gtk::ApplicationWindow {
        &self.appwindow
    }
}

// vim: ts=4 sw=4 expandtab
