// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[cfg(test)]
mod tests;

pub(crate) const HELD: Duration = Duration::from_millis(500);
/// More CPU than a sleeping waiter is charged, and far less than one spinning
/// through `HELD` would be.
pub(crate) const AWAKE: Duration = Duration::from_millis(100);
pub(crate) const SETTLE: Duration = Duration::from_millis(100);
pub(crate) const DEADLINE: Duration = Duration::from_secs(10);

/// Fails the test when `operation` has not returned by `DEADLINE`, where a
/// lock that never returns would hang it instead.
pub(crate) fn within<F>(what: &str, operation: F)
where
    F: FnOnce() + Send + 'static,
{
    let (finished, finishing) = mpsc::channel();

    thread::spawn(move || {
        operation();

        let _ = finished.send(());
    });

    if finishing.recv_timeout(DEADLINE).is_err() {
        panic!("{what} did not finish within {DEADLINE:?}");
    }
}

#[cfg(unix)]
pub(crate) fn thread_cpu_time() -> Duration {
    #[repr(C)]
    struct Timespec {
        seconds: i64,
        nanoseconds: i64,
    }

    #[cfg(target_os = "linux")]
    const CLOCK_THREAD_CPUTIME_ID: i32 = 3;
    #[cfg(target_os = "macos")]
    const CLOCK_THREAD_CPUTIME_ID: i32 = 16;

    unsafe extern "C" {
        fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
    }

    let mut time = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };

    let answer = unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut time) };

    assert_eq!(answer, 0, "clock_gettime refused the thread's clock");

    Duration::new(time.seconds as u64, time.nanoseconds as u32)
}

#[cfg(windows)]
pub(crate) fn thread_cpu_time() -> Duration {
    use core::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    impl FileTime {
        fn ticks(&self) -> u64 {
            (u64::from(self.high) << 32) | u64::from(self.low)
        }
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> *mut c_void;

        fn GetThreadTimes(
            thread: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }

    let mut creation = FileTime::default();
    let mut exit = FileTime::default();
    let mut kernel = FileTime::default();
    let mut user = FileTime::default();

    let answer = unsafe {
        GetThreadTimes(
            GetCurrentThread(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };

    assert_ne!(answer, 0, "GetThreadTimes refused the current thread");

    Duration::from_nanos((kernel.ticks() + user.ticks()) * 100)
}
