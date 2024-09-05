use crate::fixpt::Fixpt;

#[derive(Clone)]
pub struct PiParams {
    pub kp: Fixpt,
    pub ki: Fixpt,
    pub ilim: Fixpt,
}

pub struct Pi {
    params: PiParams,
    sp: Fixpt,
    i: Fixpt,
}

impl Pi {
    pub const fn new(params: PiParams) -> Self {
        Self {
            params,
            sp: Fixpt::new(0),
            i: Fixpt::new(0),
        }
    }

    pub fn setpoint(&mut self, sp: Fixpt) {
        self.sp = sp;
    }

    pub fn run(&mut self, r: Fixpt) -> Fixpt {
        // deviation
        let e = self.sp - r;

        // P term
        let p = self.params.kp * e;

        // I term
        let mut i = self.i + (self.params.ki * e);
        if i > self.params.ilim {
            i = self.params.ilim;
        }
        if i < -self.params.ilim {
            i = -self.params.ilim;
        }
        self.i = i;

        p + i
    }
}

// vim: ts=4 sw=4 expandtab
