use crate::mutex::{CriticalSection, Mutex};
use core::cell::Cell;

pub struct Ring<T, const SIZE: usize> {
    buf: [Mutex<Cell<T>>; SIZE],
    wr: Mutex<Cell<u8>>,
    rd: Mutex<Cell<u8>>,
}

impl<T, const SIZE: usize> Ring<T, SIZE> {
    const MASK: u8 = (SIZE - 1) as u8;

    pub const fn new(buf: [Mutex<Cell<T>>; SIZE]) -> Self {
        Self {
            buf,
            wr: Mutex::new(Cell::new(0)),
            rd: Mutex::new(Cell::new(0)),
        }
    }
}

impl<T: Copy, const SIZE: usize> Ring<T, SIZE> {
    fn count<'cs>(&self, cs: CriticalSection<'cs>) -> u8 {
        let wr = self.wr.borrow(cs).get();
        let rd = self.rd.borrow(cs).get();
        wr.wrapping_sub(rd)
    }

    fn is_full<'cs>(&self, cs: CriticalSection<'cs>) -> bool {
        self.count(cs) >= SIZE as _
    }

    fn is_empty<'cs>(&self, cs: CriticalSection<'cs>) -> bool {
        self.count(cs) == 0
    }

    pub fn insert<'cs>(&self, cs: CriticalSection<'cs>, value: T) -> bool {
        if self.is_full(cs) {
            false
        } else {
            let wr = self.wr.borrow(cs).get();
            self.buf[(wr & Self::MASK) as usize].borrow(cs).set(value);
            let wr = wr.wrapping_add(1);
            self.wr.borrow(cs).set(wr);
            true
        }
    }

    pub fn get<'cs>(&self, cs: CriticalSection<'cs>) -> Option<T> {
        if self.is_empty(cs) {
            None
        } else {
            let rd = self.rd.borrow(cs).get();
            let value = self.buf[(rd & Self::MASK) as usize].borrow(cs).get();
            let rd = rd.wrapping_add(1);
            self.rd.borrow(cs).set(rd);
            Some(value)
        }
    }
}

// vim: ts=4 sw=4 expandtab
