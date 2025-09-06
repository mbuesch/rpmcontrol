// -*- coding: utf-8 -*-

use crate::{diagram_area::DiagramArea, serial::SerDat};
use anyhow as ah;
use gtk4::{self as gtk, glib, prelude::*};
use plotters::prelude::*;
use plotters_cairo::CairoBackend;
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};

struct DiagramData {
    reference: Instant,
    speedo: VecDeque<(f64, f64)>,
    speedo_status: VecDeque<(f64, f64)>,
    setpoint: VecDeque<(f64, f64)>,
    pid_y: VecDeque<(f64, f64)>,
    mon_debounce: VecDeque<(f64, f64)>,
}

const T_INTERVAL: Duration = Duration::from_millis(10000);

const N_MIN: f64 = 0.0;
const N_MAX: f64 = 25_000.0;

const SPEEDO_STATUS_FACT: f64 = 100.0;
const MON_DEBOUNCE_FACT: f64 = N_MAX / 150.0;
const STROKE_WIDTH: u32 = 3;

macro_rules! check_ts {
    ($var:ident, $deque:expr, $fun:ident) => {
        if let Some(t) = $deque.map(|(t, _)| *t) {
            $var.$fun(t)
        } else {
            $var
        }
    };
}

impl DiagramData {
    pub fn new() -> Self {
        Self {
            reference: Instant::now(),
            speedo: VecDeque::new(),
            speedo_status: VecDeque::new(),
            setpoint: VecDeque::new(),
            pid_y: VecDeque::new(),
            mon_debounce: VecDeque::new(),
        }
    }

    pub fn oldest_timestamp(&self) -> f64 {
        let mut oldest = f64::MAX;
        oldest = check_ts!(oldest, self.speedo.front(), min);
        oldest = check_ts!(oldest, self.speedo_status.front(), min);
        oldest = check_ts!(oldest, self.setpoint.front(), min);
        oldest = check_ts!(oldest, self.pid_y.front(), min);
        oldest = check_ts!(oldest, self.mon_debounce.front(), min);
        if oldest < f64::MAX { oldest } else { 0.0 }
    }

    pub fn newest_timestamp(&self) -> f64 {
        let mut newest = 0.0_f64;
        newest = check_ts!(newest, self.speedo.back(), max);
        newest = check_ts!(newest, self.speedo_status.back(), max);
        newest = check_ts!(newest, self.setpoint.back(), max);
        newest = check_ts!(newest, self.pid_y.back(), max);
        newest = check_ts!(newest, self.mon_debounce.back(), max);
        newest
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
            SerDat::Speedo(t, val) => {
                self.speedo.push_back((self.timestamp(t), val));
            }
            SerDat::SpeedoStatus(t, val) => {
                self.speedo_status
                    .push_back((self.timestamp(t), val as f64 * SPEEDO_STATUS_FACT));
            }
            SerDat::Setpoint(t, val) => {
                self.setpoint.push_back((self.timestamp(t), val));
            }
            SerDat::PidY(t, val) => {
                self.pid_y.push_back((self.timestamp(t), val));
            }
            SerDat::MonDebounce(t, val) => {
                self.mon_debounce
                    .push_back((self.timestamp(t), val as f64 * MON_DEBOUNCE_FACT));
            }
            SerDat::Sync(_) => (),
        }
        Self::prune_items(&mut self.speedo, age_thres);
        Self::prune_items(&mut self.speedo_status, age_thres);
        Self::prune_items(&mut self.setpoint, age_thres);
        Self::prune_items(&mut self.pid_y, age_thres);
        Self::prune_items(&mut self.mon_debounce, age_thres);
    }
}

fn draw(backend: CairoBackend, diagram_data: Rc<RefCell<DiagramData>>) {
    let diagram_data = diagram_data.borrow();

    let area = backend.into_drawing_area();
    area.fill(&WHITE).unwrap();

    let t_min = diagram_data.oldest_timestamp();
    let t_max = diagram_data.newest_timestamp();

    let mut chart = ChartBuilder::on(&area)
        .caption("rpmcontrol", ("sans-serif", 12).into_font())
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(60)
        .build_cartesian_2d(t_min..t_max, N_MIN..N_MAX)
        .unwrap();

    chart
        .configure_mesh()
        .x_desc("time")
        .y_desc("RPM")
        .draw()
        .unwrap();

    chart
        .draw_series(LineSeries::new(
            diagram_data.speedo.iter().copied(),
            full_palette::RED.stroke_width(STROKE_WIDTH),
        ))
        .unwrap()
        .label("speedo")
        .legend(|(x, y)| {
            PathElement::new(
                vec![(x, y), (x + 20, y)],
                full_palette::RED.stroke_width(STROKE_WIDTH),
            )
        });

    chart
        .draw_series(LineSeries::new(
            diagram_data.speedo_status.iter().copied(),
            full_palette::BLACK.stroke_width(STROKE_WIDTH),
        ))
        .unwrap()
        .label("speedo-stat")
        .legend(|(x, y)| {
            PathElement::new(
                vec![(x, y), (x + 20, y)],
                full_palette::BLACK.stroke_width(STROKE_WIDTH),
            )
        });

    chart
        .draw_series(LineSeries::new(
            diagram_data.setpoint.iter().copied(),
            full_palette::BLUE.stroke_width(STROKE_WIDTH),
        ))
        .unwrap()
        .label("setpoint")
        .legend(|(x, y)| {
            PathElement::new(
                vec![(x, y), (x + 20, y)],
                full_palette::BLUE.stroke_width(STROKE_WIDTH),
            )
        });

    chart
        .draw_series(LineSeries::new(
            diagram_data.pid_y.iter().copied(),
            full_palette::ORANGE.stroke_width(STROKE_WIDTH),
        ))
        .unwrap()
        .label("pid-Y")
        .legend(|(x, y)| {
            PathElement::new(
                vec![(x, y), (x + 20, y)],
                full_palette::ORANGE.stroke_width(STROKE_WIDTH),
            )
        });

    chart
        .draw_series(LineSeries::new(
            diagram_data.mon_debounce.iter().copied(),
            full_palette::GREY.stroke_width(STROKE_WIDTH),
        ))
        .unwrap()
        .label("mon-debounce")
        .legend(|(x, y)| {
            PathElement::new(
                vec![(x, y), (x + 20, y)],
                full_palette::GREY.stroke_width(STROKE_WIDTH),
            )
        });

    chart
        .configure_series_labels()
        .background_style(WHITE)
        .border_style(BLACK)
        .draw()
        .unwrap();
}

fn periodic_work(
    ser_rx: Rc<mpsc::Receiver<SerDat>>,
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
    _diagram_area: Rc<RefCell<DiagramArea>>,
    _diagram_data: Rc<RefCell<DiagramData>>,
}

impl MainWindow {
    pub fn new(
        app: &gtk::Application,
        ser_rx: Rc<mpsc::Receiver<SerDat>>,
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
                    Rc::clone(&ser_rx),
                    Rc::clone(&diagram_area),
                    Rc::clone(&diagram_data),
                );
                glib::ControlFlow::Continue
            }
        });

        Ok(Rc::new(RefCell::new(Self {
            appwindow,
            _diagram_area: diagram_area,
            _diagram_data: diagram_data,
        })))
    }

    pub fn application_window(&self) -> &gtk::ApplicationWindow {
        &self.appwindow
    }
}

// vim: ts=4 sw=4 expandtab
