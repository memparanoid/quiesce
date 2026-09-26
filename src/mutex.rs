// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::wait::{wait, wake_one};

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;
/// Held, and a waiter may be asleep: the unlock owes it a wake.
const CONTENDED: u32 = 2;

/// Reads of a held lock before going to sleep, the bound std's futex mutex uses.
const SPINS: u32 = 100;

/// A mutual-exclusion lock whose waiters sleep in the kernel. There is no
/// poisoning: a holder that panics releases it on the way out.
pub struct Mutex<T: ?Sized> {
    state: AtomicU32,
    value: UnsafeCell<T>,
}

// SAFETY: the state hands the value to one holder at a time, so sharing the
// mutex moves `T` from thread to thread and never shares it: `Send` is enough.
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// A mutex holding `value`, unlocked.
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(UNLOCKED),
            value: UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Blocks until the value is this caller's alone, and gives it back when
    /// the guard is dropped. Locking again from the thread that holds it
    /// never returns.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        if self
            .state
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            self.lock_contended();
        }

        MutexGuard {
            mutex: self,
            marker: PhantomData,
        }
    }

    /// The value, reached through `&mut self`, which already excludes every
    /// other holder.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    #[cold]
    pub(crate) fn lock_contended(&self) {
        let mut spins = 0;

        while self.state.load(Ordering::Relaxed) == LOCKED && spins < SPINS {
            spin_loop();
            spins += 1;
        }

        if self
            .state
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return;
        }

        // `CONTENDED` even when the swap takes the lock: another waiter may be
        // asleep, and `LOCKED` would let the unlock skip the wake it is owed.
        while self.state.swap(CONTENDED, Ordering::Acquire) != UNLOCKED {
            wait(&self.state, CONTENDED);
        }
    }

    pub(crate) fn unlock(&self) {
        if self.state.swap(UNLOCKED, Ordering::Release) == CONTENDED {
            wake_one(&self.state);
        }
    }
}

/// The value a [`Mutex`] holds, for as long as the guard lives.
pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
    // What keeps the guard `Sync` only when `T` is: `&Mutex<T>` alone is
    // `Sync` for any `T: Send`, and would share `&T` between threads.
    marker: PhantomData<&'a mut T>,
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: a guard exists only while its holder has the lock, and
        // borrows the mutex for as long.
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: a guard exists only while its holder has the lock, and
        // `&mut self` is the one borrow of the one guard.
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}
