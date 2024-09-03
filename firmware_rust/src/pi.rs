use crate::fixpt::Fixpt;

pub struct Pi {
    sp: Fixpt,
    kp: Fixpt,
    ki: Fixpt,
    i: Fixpt,
}

impl Pi {
    pub const fn new(kp: Fixpt, ki: Fixpt) -> Self {
        Self {
            sp: Fixpt::new(0),
            kp,
            ki,
            i: Fixpt::new(0),
        }
    }

    pub fn setpoint(&mut self, sp: Fixpt) {
        self.sp = sp;
    }

    pub fn run(&mut self, dt: Fixpt, r: Fixpt) -> Fixpt {
        // deviation
        let e = self.sp - r;

        // P term
        let p = self.kp * e;

        // I term
        let i = self.i + (self.ki * e * dt);
        self.i = i;

        (p + i).into()
    }
}

// vim: ts=4 sw=4 expandtab
