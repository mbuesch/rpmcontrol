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
        i = i.min(params.ilim);
        i = i.max(-params.ilim);
        if reset {
            i = Fixpt::from_int(0);
        }
        self.i.set(m, i);

        p + i
    }
}

// vim: ts=4 sw=4 expandtab
