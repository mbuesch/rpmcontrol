use crate::{
    fixpt::{BigFixpt, Fixpt, fixpt},
    mutex::{MainCtx, MutexCell},
};

pub struct Filter {
    buf: MutexCell<BigFixpt>,
}

impl Filter {
    pub const fn new() -> Self {
        Self {
            buf: MutexCell::new(fixpt!(0).upgrade()),
        }
    }

    pub fn reset(&self, m: &MainCtx<'_>) {
        self.buf.set(m, fixpt!(0).upgrade());
    }

    #[inline(never)]
    pub fn run(&self, m: &MainCtx<'_>, input: Fixpt, div: Fixpt) -> Fixpt {
        let div: BigFixpt = div.into();
        let mut buf = self.buf.get(m);
        buf -= buf / div;
        buf += input.into();
        self.buf.set(m, buf);
        (buf / div).into()
    }

    pub fn get(&self, m: &MainCtx<'_>, div: Fixpt) -> Fixpt {
        (self.buf.get(m) / div.into()).into()
    }
}

// vim: ts=4 sw=4 expandtab
