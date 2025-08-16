use crate::{
    fixpt::Fixpt,
    mutex::{MainCtx, MutexCell},
};

#[derive(Clone)]
pub struct PiParams {
    pub kp: Fixpt,
    pub ki: Fixpt,
}

pub struct Pi {
    i: MutexCell<Fixpt>,
    ilim: MutexCell<Fixpt>,
}

impl Pi {
    pub const fn new() -> Self {
        Self {
            i: MutexCell::new(Fixpt::from_int(0)),
            ilim: MutexCell::new(Fixpt::from_int(0)),
        }
    }

    pub fn set_ilim(&self, m: &MainCtx<'_>, ilim: Fixpt) {
        self.ilim.set(m, ilim);
    }

    pub fn run(
        &self,
        m: &MainCtx<'_>,
        params: &PiParams,
        sp: Fixpt,
        r: Fixpt,
        reset: bool,
    ) -> Fixpt {
        // deviation
        let e = sp - r;

        // P term
        let p = params.kp * e;

        // I term
        let mut i = self.i.get(m) + (params.ki * e);
        let ilim = self.ilim.get(m);
        i = i.min(ilim);
        i = i.max(-ilim);
        if reset {
            i = Fixpt::from_int(0);
        }
        self.i.set(m, i);

        p + i
    }
}

// vim: ts=4 sw=4 expandtab
