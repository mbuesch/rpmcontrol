// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use core::arch::{asm, naked_asm};

/// Memory pattern for unused stack space.
const PATTERN: u8 = 0x5A;

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init4")]
/// Overwrite the whole stack with the [PATTERN].
///
/// # Safety
///
/// This naked function is run before main() from the .init4 section.
unsafe extern "C" fn mon_stack_mark_pattern() {
    naked_asm!(
        "   ldi r26, lo8(__bss_end)",
        "   ldi r27, hi8(__bss_end)",
        "   ldi r17, hi8(__stack)",
        "   ldi r18, {PATTERN}",
        "1: cpi r26, lo8(__stack)",
        "   cpc r27, r17",
        "   st X+, r18",
        "   brne 1b",
        PATTERN = const PATTERN,
    );
}

/// Returns the number of stack bytes that have never been written to.
///
/// The returned value is only an estimate based on checking
/// an initialization [PATTERN].
///
/// If an actual stack overflow occured, the behavior is undefined.
#[inline(always)]
pub fn estimate_unused_stack_space() -> u16 {
    let mut nrbytes;
    // SAFETY: The assembly code only does atomic memory reads.
    unsafe {
        asm!(
            "   ldi r26, lo8(__bss_end)",
            "   ldi r27, hi8(__bss_end)",
            "   ldi r18, hi8(__stack)",
            "1: cpi r26, lo8(__stack)",
            "   cpc r27, r18",
            "   breq 2f",
            "   ld r19, X+",
            "   cpi r19, {PATTERN}",
            "   breq 1b",
            "2: movw {nrbytes}, r26",
            "   subi {nrbytes:l}, lo8(__bss_end + 1)",
            "   sbci {nrbytes:h}, hi8(__bss_end + 1)",
            nrbytes = out(reg_pair) nrbytes,
            out("r18") _,
            out("r19") _,
            out("r26") _,
            out("r27") _,
            PATTERN = const PATTERN,
        );
    }
    nrbytes
}

// vim: ts=4 sw=4 expandtab
