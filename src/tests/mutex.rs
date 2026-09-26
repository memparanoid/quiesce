// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

use std::cell::Cell;
use std::hint::spin_loop;
use std::mem::forget;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use crate::Mutex;

use crate::tests::support::{AWAKE, DEADLINE, HELD, SETTLE, thread_cpu_time, within};

const HOLDERS: usize = 8;
const ROUNDS: usize = 10_000;

fn is_sync<T: Sync>() {}

// ============================================================================
// Sync for Mutex
// ============================================================================

#[test]
fn test_a_mutex_is_sync_when_its_value_is_only_send() {
    is_sync::<Mutex<Cell<u64>>>();
}

// ============================================================================
// Mutex::new
// ============================================================================

#[test]
fn test_new_holds_the_value_it_is_given() {
    let mutex = Mutex::new(7_u64);

    assert_eq!(*mutex.lock(), 7);
}

// ============================================================================
// Mutex::lock
// ============================================================================

#[test]
fn test_lock_gives_the_value_to_one_holder_at_a_time() {
    let mutex = Arc::new(Mutex::new(0_usize));

    let holders: Vec<_> = (0..HOLDERS)
        .map(|_| {
            let mutex = Arc::clone(&mutex);

            thread::spawn(move || {
                for _ in 0..ROUNDS {
                    let mut held = mutex.lock();
                    let seen = *held;

                    spin_loop();

                    *held = seen + 1;
                }
            })
        })
        .collect();

    for holder in holders {
        holder.join().expect("a holder panicked");
    }

    assert_eq!(*mutex.lock(), HOLDERS * ROUNDS);
}

#[test]
fn test_lock_waits_while_another_holds_it() {
    let mutex = Arc::new(Mutex::new(()));
    let (locked, locking) = mpsc::channel();

    let held = mutex.lock();

    let waiter = Arc::clone(&mutex);
    thread::spawn(move || {
        let _held = waiter.lock();

        let _ = locked.send(());
    });

    thread::sleep(SETTLE);

    assert!(
        locking.try_recv().is_err(),
        "a second holder took the lock while the first held it"
    );

    drop(held);

    locking
        .recv_timeout(DEADLINE)
        .expect("the waiter did not take the lock once it was let go");
}

#[test]
fn test_lock_sleeps_while_it_waits() {
    let mutex = Arc::new(Mutex::new(()));
    let (locked, locking) = mpsc::channel();

    let held = mutex.lock();

    let waiter = Arc::clone(&mutex);
    thread::spawn(move || {
        let before = thread_cpu_time();
        let _held = waiter.lock();

        let _ = locked.send(thread_cpu_time() - before);
    });

    thread::sleep(HELD);
    drop(held);

    let spent = locking
        .recv_timeout(DEADLINE)
        .expect("the waiter did not take the lock once it was let go");

    assert!(
        spent < AWAKE,
        "the waiter was charged {spent:?} while it waited"
    );
}

#[test]
fn test_lock_is_taken_again_after_a_holder_panicked() {
    let mutex = Arc::new(Mutex::new(()));

    let panicking = Arc::clone(&mutex);
    let outcome = thread::spawn(move || {
        let _held = panicking.lock();

        panic!("the holder panics while it holds the lock");
    })
    .join();

    assert!(outcome.is_err(), "the holder did not panic");

    within("a lock after a holder panicked", move || {
        drop(mutex.lock());
    });
}

// ============================================================================
// Mutex::get_mut
// ============================================================================

#[test]
fn test_get_mut_reaches_the_value() {
    let mut mutex = Mutex::new(1_u64);

    *mutex.get_mut() = 2;

    assert_eq!(*mutex.lock(), 2);
}

// ============================================================================
// Mutex::lock_contended
// ============================================================================

#[test]
fn test_lock_contended_takes_a_lock_nobody_holds() {
    let mutex = Arc::new(Mutex::new(()));
    let (locked, locking) = mpsc::channel();

    let contending = Arc::clone(&mutex);
    within("a contended lock nobody holds", move || {
        contending.lock_contended();
    });

    let waiter = Arc::clone(&mutex);
    thread::spawn(move || {
        let _held = waiter.lock();

        let _ = locked.send(());
    });

    thread::sleep(SETTLE);

    assert!(
        locking.try_recv().is_err(),
        "a second holder took the lock the contended lock took"
    );

    mutex.unlock();

    locking
        .recv_timeout(DEADLINE)
        .expect("the waiter did not take the lock once it was let go");
}

#[test]
fn test_lock_contended_sleeps_until_the_holder_lets_go() {
    let mutex = Arc::new(Mutex::new(()));
    let (locked, locking) = mpsc::channel();

    let held = mutex.lock();

    let waiter = Arc::clone(&mutex);
    thread::spawn(move || {
        let before = thread_cpu_time();

        waiter.lock_contended();

        let _ = locked.send(thread_cpu_time() - before);

        waiter.unlock();
    });

    thread::sleep(HELD);

    assert!(
        locking.try_recv().is_err(),
        "the contended lock returned while the holder held it"
    );

    drop(held);

    let spent = locking
        .recv_timeout(DEADLINE)
        .expect("the contended lock did not return once it was let go");

    assert!(
        spent < AWAKE,
        "the waiter was charged {spent:?} while it waited"
    );
}

// ============================================================================
// Mutex::unlock
// ============================================================================

#[test]
fn test_unlock_lets_the_lock_be_taken_again() {
    within("a lock after an unlock", || {
        let mutex = Mutex::new(());

        forget(mutex.lock());
        mutex.unlock();

        drop(mutex.lock());
    });
}

#[test]
fn test_unlock_wakes_a_thread_that_waits() {
    let mutex = Arc::new(Mutex::new(()));
    let (locked, locking) = mpsc::channel();

    forget(mutex.lock());

    let waiter = Arc::clone(&mutex);
    thread::spawn(move || {
        let _held = waiter.lock();

        let _ = locked.send(());
    });

    // Asleep before the unlock, or the waiter is still spinning and takes the
    // lock without the wake this asks about.
    thread::sleep(SETTLE);

    assert!(
        locking.try_recv().is_err(),
        "the waiter took the lock before the unlock"
    );

    mutex.unlock();

    locking
        .recv_timeout(DEADLINE)
        .expect("the waiter was not woken by the unlock");
}

// ============================================================================
// Deref for MutexGuard
// ============================================================================

#[test]
fn test_deref_reads_the_value() {
    let mutex = Mutex::new(7_u64);
    let held = mutex.lock();

    assert_eq!(*Deref::deref(&held), 7);
}

// ============================================================================
// DerefMut for MutexGuard
// ============================================================================

#[test]
fn test_deref_mut_writes_the_value() {
    let mutex = Mutex::new(7_u64);

    let mut held = mutex.lock();
    *DerefMut::deref_mut(&mut held) = 8;
    drop(held);

    assert_eq!(*mutex.lock(), 8);
}

// ============================================================================
// Drop for MutexGuard
// ============================================================================

#[test]
fn test_dropping_the_guard_lets_the_same_thread_lock_again() {
    within("a second lock by the same thread", || {
        let mutex = Mutex::new(());

        drop(mutex.lock());
        drop(mutex.lock());
    });
}
