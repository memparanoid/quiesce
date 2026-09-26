// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

use std::hint::spin_loop;
use std::thread;
use std::time::Instant;

use super::{AWAKE, HELD, thread_cpu_time};

// ============================================================================
// thread_cpu_time
// ============================================================================

#[test]
fn test_thread_cpu_time_charges_a_thread_that_spins() {
    let before = thread_cpu_time();
    let started = Instant::now();

    while started.elapsed() < HELD {
        spin_loop();
    }

    let spent = thread_cpu_time() - before;

    assert!(
        spent >= HELD / 2,
        "a thread that spun was charged {spent:?}"
    );
}

#[test]
fn test_thread_cpu_time_does_not_charge_a_thread_that_sleeps() {
    let before = thread_cpu_time();

    thread::sleep(HELD);

    let spent = thread_cpu_time() - before;

    assert!(spent < AWAKE, "a thread that slept was charged {spent:?}");
}
