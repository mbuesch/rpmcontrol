use crate::{
    fixpt::Fixpt,
    mains::Phase,
    mutex::{CriticalSection, MutexCell},
    system::SysPeriph,
    timer::timer_get_large,
};

pub struct Triac {
    phi_offs_ms: MutexCell<Fixpt>,
}

impl Triac {
    pub const fn new() -> Self {
        Self {
            phi_offs_ms: MutexCell::new(Fixpt::new(20)),
        }
    }

    pub fn set_phi_offs_ms(&self, cs: CriticalSection<'_>, ms: Fixpt) {
        self.phi_offs_ms.set(cs, ms);
    }

    pub fn run(&self, cs: CriticalSection<'_>, sp: &SysPeriph, phase: &Phase) {
        let now = timer_get_large(cs);
        let phi_offs_ms = self.phi_offs_ms.get(cs);

        //TODO
        core::hint::black_box(now);
        core::hint::black_box(phi_offs_ms);
        core::hint::black_box(phase);
    }
}

// vim: ts=4 sw=4 expandtab
