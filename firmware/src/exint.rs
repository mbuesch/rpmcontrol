use crate::hw::mcu;
use avr_context::{InitCtx, InitCtxCell, IrqCtx};

#[allow(non_snake_case)]
pub struct ExInt {
    pub EXINT: mcu::EXINT,
}

// SAFETY: Is initialized when constructing the MainCtx.
pub static EXINT: InitCtxCell<ExInt> = unsafe { InitCtxCell::uninit() };

const PCINT_ENA_0: bool = false;
const PCINT_ENA_1: bool = true; // PA1: Mains vsense.
const PCINT_ENA_2: bool = false;
const PCINT_ENA_3: bool = false;
const PCINT_ENA_4: bool = false;
const PCINT_ENA_5: bool = false;
const PCINT_ENA_6: bool = false;
const PCINT_ENA_7: bool = false;
const PCINT_ENA_8: bool = false;
const PCINT_ENA_9: bool = false;
const PCINT_ENA_10: bool = false;
const PCINT_ENA_11: bool = false;
const PCINT_ENA_12: bool = false;
const PCINT_ENA_13: bool = false;
const PCINT_ENA_14: bool = false;
const PCINT_ENA_15: bool = false;

impl ExInt {
    #[allow(clippy::identity_op)]
    pub fn setup(&self, _: &InitCtx) {
        self.EXINT.pcmsk0().write(|w| {
            w.set(
                ((PCINT_ENA_0 as u8) << 0)
                    | ((PCINT_ENA_1 as u8) << 1)
                    | ((PCINT_ENA_2 as u8) << 2)
                    | ((PCINT_ENA_3 as u8) << 3)
                    | ((PCINT_ENA_4 as u8) << 4)
                    | ((PCINT_ENA_5 as u8) << 5)
                    | ((PCINT_ENA_6 as u8) << 6)
                    | ((PCINT_ENA_7 as u8) << 7),
            )
        });
        self.EXINT.pcmsk1().write(|w| {
            w.set(
                ((PCINT_ENA_8 as u8) << 0)
                    | ((PCINT_ENA_9 as u8) << 1)
                    | ((PCINT_ENA_10 as u8) << 2)
                    | ((PCINT_ENA_11 as u8) << 3)
                    | ((PCINT_ENA_12 as u8) << 4)
                    | ((PCINT_ENA_13 as u8) << 5)
                    | ((PCINT_ENA_14 as u8) << 6)
                    | ((PCINT_ENA_15 as u8) << 7),
            )
        });
        self.EXINT.gifr().write(|w| w.pcif().set_bit());
        self.EXINT.gimsk().write(|w| w.pcie().set(0x3));
    }
}

pub fn irq_handler_pcint(c: &IrqCtx) {
    crate::mains::irq_handler_pcint(c);
    crate::usi_uart::irq_handler_pcint(c);
}

// vim: ts=4 sw=4 expandtab
