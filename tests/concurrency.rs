// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

use std::num::NonZero;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Once, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use quiesce::Mutex;

const ROUNDS: usize = 20_000;
const SLEEPY_ROUNDS: usize = 200;
const WIDTH: usize = 64;
const PANIC_EVERY: usize = 97;
const ON_PURPOSE: &str = "a holder panics on purpose while it holds the lock";

/// Held past the spins, so the waiters sleep in the kernel and each one needs
/// its wake.
const HELD_ASLEEP: Duration = Duration::from_micros(50);
const DEADLINE: Duration = Duration::from_secs(60);

/// Four per core, so holders are preempted while they hold the lock and the
/// waiters pile up behind them.
fn holders() -> usize {
    thread::available_parallelism().map_or(4, NonZero::get) * 4
}

/// Sends from `drop`, so a holder that panics reaches its join instead of
/// reading as a hang.
struct Finished(mpsc::Sender<()>);

impl Drop for Finished {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

fn contend<F>(what: &str, rounds: usize, round: F)
where
    F: Fn(usize) + Send + Sync + 'static,
{
    let holders = holders();
    let round = Arc::new(round);
    let start = Arc::new(Barrier::new(holders));
    let (finished, finishing) = mpsc::channel();

    let handles: Vec<_> = (0..holders)
        .map(|_| {
            let round = Arc::clone(&round);
            let start = Arc::clone(&start);
            let finished = Finished(finished.clone());

            thread::spawn(move || {
                let _finished = finished;

                start.wait();

                for index in 0..rounds {
                    round(index);
                }
            })
        })
        .collect();

    let deadline = Instant::now() + DEADLINE;

    for _ in 0..holders {
        let left = deadline.saturating_duration_since(Instant::now());

        if finishing.recv_timeout(left).is_err() {
            panic!("{what}: a holder did not finish within {DEADLINE:?}");
        }
    }

    for handle in handles {
        handle
            .join()
            .unwrap_or_else(|_| panic!("{what}: a holder panicked"));
    }
}

fn into_value<T>(mut mutex: Arc<Mutex<T>>) -> T
where
    T: Copy,
{
    *Arc::get_mut(&mut mutex)
        .expect("Infallible: every holder has been joined")
        .get_mut()
}

fn silence_the_panics_on_purpose() {
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        let default = panic::take_hook();

        panic::set_hook(Box::new(move |info| {
            if info.payload().downcast_ref::<&str>() != Some(&ON_PURPOSE) {
                default(info);
            }
        }));
    });
}

// ============================================================================
// Mutex::lock
// ============================================================================

#[test]
fn test_lock_lets_one_holder_in_at_a_time() {
    let mutex = Arc::new(Mutex::new(()));
    let inside = Arc::new(AtomicUsize::new(0));

    let held = Arc::clone(&mutex);
    let counted = Arc::clone(&inside);
    contend("one holder at a time", ROUNDS, move |_| {
        let _held = held.lock();

        assert_eq!(
            counted.fetch_add(1, Ordering::SeqCst),
            0,
            "a holder found another inside"
        );

        thread::yield_now();

        counted.fetch_sub(1, Ordering::SeqCst);
    });

    assert_eq!(inside.load(Ordering::SeqCst), 0);
}

#[test]
fn test_lock_loses_no_update_under_contention() {
    let mutex = Arc::new(Mutex::new(0_usize));

    let held = Arc::clone(&mutex);
    contend("no update lost", ROUNDS, move |_| {
        let mut count = held.lock();
        let seen = *count;

        thread::yield_now();

        *count = seen + 1;
    });

    assert_eq!(into_value(mutex), holders() * ROUNDS);
}

#[test]
fn test_lock_hands_every_write_to_the_next_holder() {
    let mutex = Arc::new(Mutex::new([0_usize; WIDTH]));

    let held = Arc::clone(&mutex);
    contend("every write handed over", ROUNDS, move |_| {
        let mut row = held.lock();
        let first = row[0];

        assert!(
            row.iter().all(|&cell| cell == first),
            "a holder found a row half written by the one before"
        );

        let (low, high) = row.split_at_mut(WIDTH / 2);

        // Half a row written when the holder gives up the core: a second
        // holder let in reads it.
        low.fill(first + 1);
        thread::yield_now();
        high.fill(first + 1);
    });

    let row = into_value(mutex);

    assert!(row.iter().all(|&cell| cell == holders() * ROUNDS));
}

#[test]
fn test_lock_wakes_every_waiter_it_puts_to_sleep() {
    let mutex = Arc::new(Mutex::new(0_usize));

    let held = Arc::clone(&mutex);
    contend("every waiter woken", SLEEPY_ROUNDS, move |_| {
        let mut count = held.lock();

        thread::sleep(HELD_ASLEEP);

        *count += 1;
    });

    assert_eq!(into_value(mutex), holders() * SLEEPY_ROUNDS);
}

// ============================================================================
// Drop for MutexGuard
// ============================================================================

#[test]
fn test_dropping_the_guard_of_a_holder_that_panicked_lets_the_others_in() {
    silence_the_panics_on_purpose();

    let mutex = Arc::new(Mutex::new(0_usize));

    let held = Arc::clone(&mutex);
    contend("holders that panic", ROUNDS, move |index| {
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut count = held.lock();

            *count += 1;

            if index % PANIC_EVERY == 0 {
                panic::panic_any(ON_PURPOSE);
            }
        }));

        assert_eq!(
            outcome.is_err(),
            index % PANIC_EVERY == 0,
            "a holder panicked when it was not asked to, or did not when it was"
        );
    });

    assert_eq!(into_value(mutex), holders() * ROUNDS);
}
