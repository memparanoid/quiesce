// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread;

use crate::wait::{wait, wake_one};

use crate::tests::support::{AWAKE, DEADLINE, HELD, SETTLE, thread_cpu_time, within};

// ============================================================================
// wait
// ============================================================================

#[test]
fn test_wait_returns_when_the_value_is_not_the_one_expected() {
    within("a wait on a value that is not the one expected", || {
        let atomic = AtomicU32::new(1);

        wait(&atomic, 0);
    });
}

#[test]
fn test_wait_sleeps_while_the_value_is_the_one_expected() {
    let atomic = Arc::new(AtomicU32::new(0));
    let (returned, returning) = mpsc::channel();

    let waiter = Arc::clone(&atomic);
    thread::spawn(move || {
        let before = thread_cpu_time();

        while waiter.load(Ordering::Acquire) == 0 {
            wait(&waiter, 0);
        }

        let _ = returned.send(thread_cpu_time() - before);
    });

    thread::sleep(HELD);

    assert!(
        returning.try_recv().is_err(),
        "the waiter returned while the value was the one expected"
    );

    atomic.store(1, Ordering::Release);
    wake_one(&atomic);

    let spent = returning
        .recv_timeout(DEADLINE)
        .expect("the waiter did not return once it was woken");

    assert!(
        spent < AWAKE,
        "the waiter was charged {spent:?} while it waited"
    );
}

// ============================================================================
// wake_one
// ============================================================================

#[test]
fn test_wake_one_wakes_a_thread_that_waits() {
    let atomic = Arc::new(AtomicU32::new(0));
    let (returned, returning) = mpsc::channel();

    let waiter = Arc::clone(&atomic);
    thread::spawn(move || {
        while waiter.load(Ordering::Acquire) == 0 {
            wait(&waiter, 0);
        }

        let _ = returned.send(());
    });

    // Asleep before the store, or the waiter reads 1 and returns without the
    // wake this asks about.
    thread::sleep(SETTLE);

    atomic.store(1, Ordering::Release);
    wake_one(&atomic);

    returning
        .recv_timeout(DEADLINE)
        .expect("the waiter did not return once it was woken");
}

#[test]
fn test_wake_one_returns_when_nothing_waits() {
    within("a wake with nothing waiting", || {
        let atomic = AtomicU32::new(0);

        wake_one(&atomic);
    });
}
