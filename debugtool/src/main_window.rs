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

const T_INTERVAL: Duration = Duration::from_millis(10000);

const N_MIN: f64 = -5_000.0;
const N_MAX: f64 = 25_000.0;

const SPEEDO_STATUS_FACT: f64 = 100.0;
const MON_DEBOUNCE_FACT: f64 = N_MAX / 150.0;
const TEMP_FACT: f64 = N_MAX / 100.0;
const MAXRT_FACT: f64 = N_MAX / 0.010;
const MINSTACK_FACT: f64 = N_MAX / 512.0;

const STROKE_WIDTH: u32 = 3;

struct DiagramVisibility {
    speedo: bool,
    speedo_status: bool,
    setpoint: bool,
    pid_y: bool,
    mon_debounce: bool,
    temp_mot: bool,
    temp_uc: bool,
    maxrt: bool,
    minstack: bool,
}

impl DiagramVisibility {
    fn new() -> Self {
        Self {
            speedo: true,
            speedo_status: false,
            setpoint: true,
            pid_y: false,
            mon_debounce: false,
            temp_mot: true,
            temp_uc: false,
            maxrt: false,
            minstack: false,
        }
    }
}

struct DiagramData {
    reference: Instant,
    speedo: VecDeque<(f64, f64)>,
    speedo_status: VecDeque<(f64, f64)>,
    setpoint: VecDeque<(f64, f64)>,
    pid_y: VecDeque<(f64, f64)>,
    mon_debounce: VecDeque<(f64, f64)>,
    temp_mot: VecDeque<(f64, f64)>,
    temp_uc: VecDeque<(f64, f64)>,
    maxrt: VecDeque<(f64, f64)>,
    minstack: VecDeque<(f64, f64)>,
    visibility: DiagramVisibility,
    run: bool,
}

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
            temp_mot: VecDeque::new(),
            temp_uc: VecDeque::new(),
            maxrt: VecDeque::new(),
            minstack: VecDeque::new(),
            visibility: DiagramVisibility::new(),
            run: true,
        }
    }

    pub fn oldest_timestamp(&self) -> f64 {
        let mut oldest = f64::MAX;
        oldest = check_ts!(oldest, self.speedo.front(), min);
        oldest = check_ts!(oldest, self.speedo_status.front(), min);
        oldest = check_ts!(oldest, self.setpoint.front(), min);
        oldest = check_ts!(oldest, self.pid_y.front(), min);
        oldest = check_ts!(oldest, self.mon_debounce.front(), min);
        oldest = check_ts!(oldest, self.temp_mot.front(), min);
        oldest = check_ts!(oldest, self.temp_uc.front(), min);
        oldest = check_ts!(oldest, self.maxrt.front(), min);
        oldest = check_ts!(oldest, self.minstack.front(), min);
        if oldest < f64::MAX { oldest } else { 0.0 }
    }

    pub fn newest_timestamp(&self) -> f64 {
        let mut newest = 0.0_f64;
        newest = check_ts!(newest, self.speedo.back(), max);
        newest = check_ts!(newest, self.speedo_status.back(), max);
        newest = check_ts!(newest, self.setpoint.back(), max);
        newest = check_ts!(newest, self.pid_y.back(), max);
        newest = check_ts!(newest, self.mon_debounce.back(), max);
        newest = check_ts!(newest, self.temp_mot.back(), max);
        newest = check_ts!(newest, self.temp_uc.back(), max);
        newest = check_ts!(newest, self.maxrt.back(), max);
        newest = check_ts!(newest, self.minstack.back(), max);
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
            SerDat::TempMot(t, val) => {
                self.temp_mot
                    .push_back((self.timestamp(t), val * TEMP_FACT));
            }
            SerDat::TempUc(t, val) => {
                self.temp_uc.push_back((self.timestamp(t), val * TEMP_FACT));
            }
            SerDat::MaxRt(t, val) => {
                self.maxrt.push_back((self.timestamp(t), val * MAXRT_FACT));
            }
            SerDat::MinStack(t, val) => {
                self.minstack
                    .push_back((self.timestamp(t), val as f64 * MINSTACK_FACT));
            }
            SerDat::Sync => (),
        }
        Self::prune_items(&mut self.speedo, age_thres);
        Self::prune_items(&mut self.speedo_status, age_thres);
        Self::prune_items(&mut self.setpoint, age_thres);
        Self::prune_items(&mut self.pid_y, age_thres);
        Self::prune_items(&mut self.mon_debounce, age_thres);
        Self::prune_items(&mut self.temp_mot, age_thres);
        Self::prune_items(&mut self.temp_uc, age_thres);
        Self::prune_items(&mut self.maxrt, age_thres);
        Self::prune_items(&mut self.minstack, age_thres);
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

    if diagram_data.visibility.speedo {
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
    }

    if diagram_data.visibility.speedo_status {
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
    }

    if diagram_data.visibility.setpoint {
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
    }

    if diagram_data.visibility.pid_y {
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
    }

    if diagram_data.visibility.mon_debounce {
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
    }

    if diagram_data.visibility.temp_mot {
        chart
            .draw_series(LineSeries::new(
                diagram_data.temp_mot.iter().copied(),
                full_palette::BLUE_900.stroke_width(STROKE_WIDTH),
            ))
            .unwrap()
            .label("temp-mot")
            .legend(|(x, y)| {
                PathElement::new(
                    vec![(x, y), (x + 20, y)],
                    full_palette::BLUE_900.stroke_width(STROKE_WIDTH),
                )
            });
    }

    if diagram_data.visibility.temp_uc {
        chart
            .draw_series(LineSeries::new(
                diagram_data.temp_uc.iter().copied(),
                full_palette::BLUE_200.stroke_width(STROKE_WIDTH),
            ))
            .unwrap()
            .label("temp-uc")
            .legend(|(x, y)| {
                PathElement::new(
                    vec![(x, y), (x + 20, y)],
                    full_palette::BLUE_200.stroke_width(STROKE_WIDTH),
                )
            });
    }

    if diagram_data.visibility.maxrt {
        chart
            .draw_series(LineSeries::new(
                diagram_data.maxrt.iter().copied(),
                full_palette::BLACK.stroke_width(STROKE_WIDTH),
            ))
            .unwrap()
            .label("max-rt")
            .legend(|(x, y)| {
                PathElement::new(
                    vec![(x, y), (x + 20, y)],
                    full_palette::BLACK.stroke_width(STROKE_WIDTH),
                )
            });
    }

    if diagram_data.visibility.minstack {
        chart
            .draw_series(LineSeries::new(
                diagram_data.minstack.iter().copied(),
                full_palette::BLACK.stroke_width(STROKE_WIDTH),
            ))
            .unwrap()
            .label("min-stack")
            .legend(|(x, y)| {
                PathElement::new(
                    vec![(x, y), (x + 20, y)],
                    full_palette::BLACK.stroke_width(STROKE_WIDTH),
                )
            });
    }

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
    let mut diagram_data = diagram_data.borrow_mut();
    for dat in ser_rx.try_iter() {
        if diagram_data.run {
            diagram_data.add(dat);
        }
    }
    drop(diagram_data);
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
        let builder = gtk::Builder::from_string(include_str!("gui/main_window.ui"));

        let appwindow: gtk::ApplicationWindow = builder
            .object("main_window")
            .expect("Get main_window failed");
        appwindow.set_application(Some(app));
        appwindow.set_title(Some("rpmcontrol debug"));
        appwindow.set_default_size(800, 600);

        let diagram_data = Rc::new(RefCell::new(DiagramData::new()));
        let diagram_area = DiagramArea::new(&builder, "drawing_area", {
            let diagram_data = Rc::clone(&diagram_data);
            move |backend| draw(backend, Rc::clone(&diagram_data))
        });

        macro_rules! connect_signal_cb {
            ($builder:expr, $name:expr, $field:ident) => {
                let cb: gtk::CheckButton = $builder.object($name).expect("CheckButton not found");
                let diagram_data = Rc::clone(&diagram_data);
                let diagram_area = Rc::clone(&diagram_area);
                cb.set_active(diagram_data.borrow().visibility.$field);
                cb.connect_toggled(move |cb| {
                    diagram_data.borrow_mut().visibility.$field = cb.is_active();
                    diagram_area.borrow().redraw();
                });
            };
        }

        macro_rules! connect_run_cb {
            ($builder:expr, $name:expr) => {
                let cb: gtk::CheckButton = $builder.object($name).expect("CheckButton not found");
                let diagram_data = Rc::clone(&diagram_data);
                let diagram_area = Rc::clone(&diagram_area);
                cb.set_active(true);
                cb.connect_toggled(move |cb| {
                    diagram_data.borrow_mut().run = cb.is_active();
                    diagram_area.borrow().redraw();
                });
            };
        }

        connect_signal_cb!(builder, "cb_speedo", speedo);
        connect_signal_cb!(builder, "cb_speedo_stat", speedo_status);
        connect_signal_cb!(builder, "cb_setpoint", setpoint);
        connect_signal_cb!(builder, "cb_pid_y", pid_y);
        connect_signal_cb!(builder, "cb_mon_debounce", mon_debounce);
        connect_signal_cb!(builder, "cb_temp_mot", temp_mot);
        connect_signal_cb!(builder, "cb_temp_uc", temp_uc);
        connect_signal_cb!(builder, "cb_maxrt", maxrt);
        connect_signal_cb!(builder, "cb_minstack", minstack);
        connect_run_cb!(builder, "cb_run");

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
