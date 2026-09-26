// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

use core::ffi::c_void;
use core::sync::atomic::AtomicU32;

const OS_SYNC_WAIT_ON_ADDRESS_NONE: u32 = 0;
const OS_SYNC_WAKE_BY_ADDRESS_NONE: u32 = 0;

// In libSystem from macOS 14.4: an older system refuses to load the binary.
unsafe extern "C" {
    fn os_sync_wait_on_address(address: *mut c_void, value: u64, size: usize, flags: u32) -> i32;

    fn os_sync_wake_by_address_any(address: *mut c_void, size: usize, flags: u32) -> i32;
}

pub(crate) fn wait(atomic: &AtomicU32, expected: u32) {
    // SAFETY: the address is a live `AtomicU32` and the size is its own, which
    // the kernel only reads and compares.
    unsafe {
        os_sync_wait_on_address(
            atomic.as_ptr().cast(),
            u64::from(expected),
            size_of::<AtomicU32>(),
            OS_SYNC_WAIT_ON_ADDRESS_NONE,
        );
    }
}

pub(crate) fn wake_one(atomic: &AtomicU32) {
    // SAFETY: the kernel uses the address as a key and touches no memory.
    unsafe {
        os_sync_wake_by_address_any(
            atomic.as_ptr().cast(),
            size_of::<AtomicU32>(),
            OS_SYNC_WAKE_BY_ADDRESS_NONE,
        );
    }
}
