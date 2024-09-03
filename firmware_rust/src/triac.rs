use crate::{
    fixpt::Fixpt,
    mutex::{CriticalSection, MutexCell},
    system::SysPeriph,
};

pub struct Triac {
    phi_offs: MutexCell<Fixpt>,
}

impl Triac {
    pub const fn new() -> Self {
        Self {
            phi_offs: MutexCell::new(Fixpt::new(20)),
        }
    }

    pub fn set_phi_offs(&self, cs: CriticalSection<'_>, offs: Fixpt) {
        self.phi_offs.set(cs, offs);
    }

    pub fn run(&self, cs: CriticalSection<'_>, sp: &SysPeriph) {
        //TODO
        core::hint::black_box(self.phi_offs.get(cs));
    }
}

// vim: ts=4 sw=4 expandtab
