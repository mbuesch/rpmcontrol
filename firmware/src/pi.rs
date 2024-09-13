use crate::fixpt::Fixpt;

#[derive(Clone)]
pub struct PiParams {
    pub kp: Fixpt,
    pub ki: Fixpt,
    pub ilim: Fixpt,
}

pub struct Pi {
    i: Fixpt,
}

impl Pi {
    pub const fn new() -> Self {
        Self {
            i: Fixpt::from_int(0),
        }
    }

    pub fn run(&mut self, params: &PiParams, sp: Fixpt, r: Fixpt) -> Fixpt {
        // deviation
        let e = sp - r;

        // P term
        let p = params.kp * e;

        // I term
        let i = self.i + (params.ki * e);
        let i = i.min(params.ilim);
        let i = i.max(-params.ilim);
        self.i = i;

        p + i
    }
}

// vim: ts=4 sw=4 expandtab
