use crate::{
    fixpt::{Fixpt, fixpt},
    mutex::{MainCtx, MainCtxCell},
};

#[derive(Clone)]
pub struct PidParams {
    pub kp: Fixpt,
    pub ki: Fixpt,
    pub kd: Fixpt,
}

#[derive(Clone)]
pub struct PidIlim {
    pub neg: Fixpt,
    pub pos: Fixpt,
}

pub struct Pid {
    i: MainCtxCell<Fixpt>,
    prev_e: MainCtxCell<Fixpt>,
}

impl Pid {
    pub const fn new() -> Self {
        Self {
            i: MainCtxCell::new(Fixpt::from_int(0)),
            prev_e: MainCtxCell::new(Fixpt::from_int(0)),
        }
    }

    pub fn run(
        &self,
        m: &MainCtx<'_>,
        params: &PidParams,
        ilim: &PidIlim,
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
        if reset {
            i = fixpt!(0);
        }
        i = i.min(ilim.pos);
        i = i.max(ilim.neg);
        self.i.set(m, i);

        // D term
        let de = e - self.prev_e.get(m);
        self.prev_e.set(m, e);
        let d = de * params.kd; // assume constant delta-time between calls

        p + i + d
    }
}

// vim: ts=4 sw=4 expandtab
