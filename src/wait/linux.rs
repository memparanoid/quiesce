// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("quiesce supports Linux on x86_64 and aarch64");

use core::arch::asm;
use core::sync::atomic::AtomicU32;

#[cfg(target_arch = "x86_64")]
const SYS_FUTEX: usize = 202;
#[cfg(target_arch = "aarch64")]
const SYS_FUTEX: usize = 98;

// `FUTEX_WAIT` (0) and `FUTEX_WAKE` (1) with `FUTEX_PRIVATE_FLAG` (128): the
// kernel keys the wait by this process's mappings alone.
const FUTEX_WAIT_PRIVATE: usize = 128;
const FUTEX_WAKE_PRIVATE: usize = 129;

pub(crate) fn wait(atomic: &AtomicU32, expected: u32) {
    // SAFETY: the address is a live `AtomicU32`, which the kernel only reads
    // and compares, and the timeout is null, which waits without one.
    unsafe { futex(atomic, FUTEX_WAIT_PRIVATE, expected as usize) };
}

pub(crate) fn wake_one(atomic: &AtomicU32) {
    // SAFETY: the kernel uses the address as a key and touches no memory.
    unsafe { futex(atomic, FUTEX_WAKE_PRIVATE, 1) };
}

#[cfg(target_arch = "x86_64")]
unsafe fn futex(atomic: &AtomicU32, op: usize, value: usize) {
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_FUTEX => _,
            in("rdi") atomic.as_ptr(),
            in("rsi") op,
            in("rdx") value,
            in("r10") 0_usize,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn futex(atomic: &AtomicU32, op: usize, value: usize) {
    unsafe {
        asm!(
            "svc 0",
            in("x8") SYS_FUTEX,
            inlateout("x0") atomic.as_ptr() => _,
            in("x1") op,
            in("x2") value,
            in("x3") 0_usize,
            options(nostack),
        );
    }
}
