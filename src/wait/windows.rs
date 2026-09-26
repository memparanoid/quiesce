// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

use core::ffi::c_void;
use core::sync::atomic::AtomicU32;

const INFINITE: u32 = u32::MAX;

// The API set Windows 8 introduced these in; `raw-dylib` needs no import
// library on either toolchain.
#[link(name = "api-ms-win-core-synch-l1-2-0", kind = "raw-dylib")]
unsafe extern "system" {
    fn WaitOnAddress(
        address: *const c_void,
        compare: *const c_void,
        size: usize,
        milliseconds: u32,
    ) -> i32;

    fn WakeByAddressSingle(address: *const c_void);
}

pub(crate) fn wait(atomic: &AtomicU32, expected: u32) {
    let expected: *const u32 = &expected;

    // SAFETY: both addresses are live for the call and the size is theirs;
    // Windows only reads and compares them.
    unsafe {
        WaitOnAddress(
            atomic.as_ptr().cast(),
            expected.cast(),
            size_of::<AtomicU32>(),
            INFINITE,
        );
    }
}

pub(crate) fn wake_one(atomic: &AtomicU32) {
    // SAFETY: Windows uses the address as a key and touches no memory.
    unsafe { WakeByAddressSingle(atomic.as_ptr().cast()) };
}
