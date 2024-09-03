use crate::{mutex::CriticalSection, system::SysPeriph};

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
enum MainsState {
    /// Synchronizing to line phase.
    Synchronize,
    /// Wait to trigger.
    Wait,
    /// Trigger.
    Trigger,
}

pub struct Mains {
    state: MainsState,
    cycle_count: u8,
}

impl Mains {
    pub const fn new() -> Self {
        Self {
            state: MainsState::Synchronize,
            cycle_count: 0,
        }
    }

    pub fn run(&mut self, cs: CriticalSection<'_>, sp: &SysPeriph) {
        match self.state {
            MainsState::Synchronize => {
                //TODO read vsense from digital input
            }
            MainsState::Wait => {
                //TODO
            }
            MainsState::Trigger => {
                //TODO
            }
        }
    }
}

// vim: ts=4 sw=4 expandtab
