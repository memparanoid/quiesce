// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

//! A `no_std` mutex whose waiters sleep in the kernel instead of spinning:
//! a futex on Linux (x86_64, aarch64), `os_sync_wait_on_address` on macOS
//! 14.4 and later, `WaitOnAddress` on Windows 8 and later.

#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

mod mutex;
mod wait;

#[cfg(test)]
mod tests;

pub use mutex::{Mutex, MutexGuard};
