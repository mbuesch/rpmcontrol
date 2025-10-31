use avr_context::{MainCtx, MainCtxCell};

pub struct History<T, const SIZE: usize> {
    hist: [MainCtxCell<T>; SIZE],
}

impl<T, const SIZE: usize> History<T, SIZE> {
    pub const fn new(hist: [MainCtxCell<T>; SIZE]) -> Self {
        Self { hist }
    }
}

impl<T: Copy, const SIZE: usize> History<T, SIZE> {
    pub fn push_back(&self, m: &MainCtx<'_>, new: T) {
        for i in 1..SIZE {
            self.hist[i - 1].set(m, self.hist[i].get(m))
        }
        self.hist[SIZE - 1].set(m, new);
    }

    pub fn get(&self, m: &MainCtx<'_>, index: usize) -> T {
        self.hist[index].get(m)
    }

    pub fn oldest(&self, m: &MainCtx<'_>) -> T {
        self.get(m, 0)
    }
}

// vim: ts=4 sw=4 expandtab
