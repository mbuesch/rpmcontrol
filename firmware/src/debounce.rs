use crate::mutex::{MainCtx, MutexCell};

pub struct Debounce<const ERRSTEP: u8, const LIMIT: u8, const STICKY: bool> {
    count: MutexCell<u8>,
}

impl<const ERRSTEP: u8, const LIMIT: u8, const STICKY: bool> Debounce<ERRSTEP, LIMIT, STICKY> {
    pub const fn new() -> Self {
        Self {
            count: MutexCell::new(0),
        }
    }

    pub fn is_ok(&self, m: &MainCtx<'_>) -> bool {
        self.count.get(m) < LIMIT
    }

    pub fn error(&self, m: &MainCtx<'_>) {
        self.count.set(m, self.count.get(m).saturating_add(ERRSTEP));
    }

    pub fn error_no_debounce(&self, m: &MainCtx<'_>) {
        self.count.set(m, LIMIT);
    }

    pub fn ok(&self, m: &MainCtx<'_>) {
        if !STICKY || self.is_ok(m) {
            self.count.set(m, self.count.get(m).saturating_sub(1));
        }
    }

    pub fn count(&self, m: &MainCtx<'_>) -> u8 {
        self.count.get(m)
    }
}

// vim: ts=4 sw=4 expandtab
