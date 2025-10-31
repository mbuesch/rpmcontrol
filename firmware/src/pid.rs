use avr_context::{MainCtx, MainCtxCell};
use avr_q::{Q7p8, q7p8};

#[derive(Clone)]
pub struct PidParams {
    pub kp: Q7p8,
    pub ki: Q7p8,
    pub kd: Q7p8,
}

#[derive(Clone)]
pub struct PidIlim {
    pub neg: Q7p8,
    pub pos: Q7p8,
}

pub struct Pid {
    i: MainCtxCell<Q7p8>,
    prev_e: MainCtxCell<Q7p8>,
}

impl Pid {
    pub const fn new() -> Self {
        Self {
            i: MainCtxCell::new(q7p8!(const 0)),
            prev_e: MainCtxCell::new(q7p8!(const 0)),
        }
    }

    pub fn run(
        &self,
        m: &MainCtx<'_>,
        params: &PidParams,
        ilim: &PidIlim,
        sp: Q7p8,
        r: Q7p8,
        reset: bool,
    ) -> Q7p8 {
        // deviation
        let e = sp - r;

        // P term
        let p = params.kp * e;

        // I term
        let mut i = self.i.get(m) + (params.ki * e);
        if reset {
            i = q7p8!(const 0);
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
