use avr_context::{MainCtx, MainCtxCell};
use avr_q::{Q7p8, Q15p8, q7p8, q15p8};

pub struct Filter {
    buf: MainCtxCell<Q15p8>,
    out: MainCtxCell<Q7p8>,
}

impl Filter {
    pub const fn new() -> Self {
        Self {
            buf: MainCtxCell::new(q15p8!(const 0)),
            out: MainCtxCell::new(q7p8!(const 0)),
        }
    }

    pub fn reset(&self, m: &MainCtx<'_>) {
        self.buf.set(m, q15p8!(const 0));
        self.out.set(m, q7p8!(const 0));
    }

    #[inline(never)]
    pub fn run(&self, m: &MainCtx<'_>, input: Q7p8, div: Q7p8) -> Q7p8 {
        let mut buf = self.buf.get(m);
        buf -= self.out.get(m).into();
        buf += input.into();
        self.buf.set(m, buf);

        let out = (buf / div.into()).into();
        self.out.set(m, out);

        out
    }

    pub fn get(&self, m: &MainCtx<'_>) -> Q7p8 {
        self.out.get(m)
    }
}

// vim: ts=4 sw=4 expandtab
