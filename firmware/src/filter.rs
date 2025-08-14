use crate::{
    fixpt::Fixpt,
    mutex::{MainCtx, MutexCell},
};

pub struct Filter {
    buf: MutexCell<Fixpt>,
}

impl Filter {
    pub const fn new() -> Self {
        Self {
            buf: MutexCell::new(Fixpt::zero()),
        }
    }

    pub fn reset(&self, m: &MainCtx<'_>) {
        self.buf.set(m, Fixpt::zero());
    }

    pub fn run(&self, m: &MainCtx<'_>, input: Fixpt, div: Fixpt) -> Fixpt {
        let mut buf = self.buf.get(m);
        buf -= buf / div;
        buf += input;
        self.buf.set(m, buf);
        buf / div
    }
}

// vim: ts=4 sw=4 expandtab
