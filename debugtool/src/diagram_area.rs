// -*- coding: utf-8 -*-

use gtk4::{self as gtk, prelude::*};
use plotters_cairo::CairoBackend;
use std::{cell::RefCell, rc::Rc};

pub struct DiagramArea {
    area: gtk::DrawingArea,
}

impl DiagramArea {
    pub fn new<D: FnMut(CairoBackend) + 'static>(
        builder: &gtk::Builder,
        area_name: &str,
        mut draw: D,
    ) -> Rc<RefCell<Self>> {
        let area: gtk::DrawingArea = builder.object(area_name).unwrap();

        area.set_draw_func(move |_, cr, width, height| {
            if let Ok(backend) = CairoBackend::new(cr, (width as u32, height as u32)) {
                draw(backend);
            }
        });

        Rc::new(RefCell::new(Self { area }))
    }

    pub fn redraw(&self) {
        self.area.queue_draw();
    }
}
// vim: ts=4 sw=4 expandtab
