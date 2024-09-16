use crate::{
    fixpt::Fixpt,
    mutex::{MainCtx, MutexCell},
};

#[derive(Clone)]
pub struct PiParams {
    pub kp: Fixpt,
    pub ki: Fixpt,
    pub ilim: Fixpt,
}

pub struct Pi {
    i: MutexCell<Fixpt>,
}

impl Pi {
    pub const fn new() -> Self {
        Self {
            i: MutexCell::new(Fixpt::from_int(0)),
        }
    }

    pub fn run(&self, m: &MainCtx<'_>, params: &PiParams, sp: Fixpt, r: Fixpt) -> Fixpt {
        // deviation
        let e = sp - r;

        // P term
        let p = params.kp * e;

        // I term
        let i = self.i.get(m) + (params.ki * e);
        let i = i.min(params.ilim);
        let i = i.max(-params.ilim);
        self.i.set(m, i);

        p + i
    }
}

// vim: ts=4 sw=4 expandtab
