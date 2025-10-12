use crate::{
    fixpt::{BigFixpt, Fixpt, fixpt},
    mutex::{MainCtx, MutexCell},
};

pub struct Filter {
    buf: MutexCell<BigFixpt>,
    out: MutexCell<Fixpt>,
}

impl Filter {
    pub const fn new() -> Self {
        Self {
            buf: MutexCell::new(fixpt!(0).upgrade()),
            out: MutexCell::new(fixpt!(0)),
        }
    }

    pub fn reset(&self, m: &MainCtx<'_>) {
        self.buf.set(m, const { fixpt!(0).upgrade() });
        self.out.set(m, fixpt!(0));
    }

    #[inline(never)]
    pub fn run(&self, m: &MainCtx<'_>, input: Fixpt, div: Fixpt) -> Fixpt {
        let mut buf = self.buf.get(m);
        buf -= self.out.get(m).into();
        buf += input.into();
        self.buf.set(m, buf);

        let out = (buf / div.into()).into();
        self.out.set(m, out);

        out
    }

    pub fn get(&self, m: &MainCtx<'_>) -> Fixpt {
        self.out.get(m)
    }
}

// vim: ts=4 sw=4 expandtab
