// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("waiting is measured on Linux only");
}

#[cfg(target_os = "linux")]
fn main() {
    linux::main();
}

#[cfg(target_os = "linux")]
mod linux {
    use std::num::NonZero;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{Duration, Instant};

    const ROUNDS: usize = 500;
    const HELD: Duration = Duration::from_micros(50);

    trait Lock: Send + Sync + 'static {
        const NAME: &'static str;

        fn new() -> Self;

        fn hold(&self);
    }

    impl Lock for quiesce::Mutex<u64> {
        const NAME: &'static str = "quiesce";

        fn new() -> Self {
            quiesce::Mutex::new(0)
        }

        fn hold(&self) {
            let mut held = self.lock();

            thread::sleep(HELD);
            *held += 1;
        }
    }

    impl Lock for std::sync::Mutex<u64> {
        const NAME: &'static str = "std";

        fn new() -> Self {
            std::sync::Mutex::new(0)
        }

        fn hold(&self) {
            let mut held = self.lock().expect("Infallible: no holder panics here");

            thread::sleep(HELD);
            *held += 1;
        }
    }

    impl Lock for spin::Mutex<u64> {
        const NAME: &'static str = "spin";

        fn new() -> Self {
            spin::Mutex::new(0)
        }

        fn hold(&self) {
            let mut held = self.lock();

            thread::sleep(HELD);
            *held += 1;
        }
    }

    fn thread_cpu_time() -> Duration {
        #[repr(C)]
        struct Timespec {
            seconds: i64,
            nanoseconds: i64,
        }

        const CLOCK_THREAD_CPUTIME_ID: i32 = 3;

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

    fn measure<L: Lock>(threads: usize) {
        let lock = Arc::new(L::new());
        let start = Arc::new(Barrier::new(threads + 1));

        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let lock = Arc::clone(&lock);
                let start = Arc::clone(&start);

                thread::spawn(move || {
                    start.wait();

                    let before = thread_cpu_time();

                    for _ in 0..ROUNDS {
                        lock.hold();
                    }

                    thread_cpu_time() - before
                })
            })
            .collect();

        start.wait();
        let started = Instant::now();

        let charged: Duration = handles
            .into_iter()
            .map(|handle| handle.join().expect("a thread of the benchmark panicked"))
            .sum();

        let wall = started.elapsed();

        println!(
            "| {:<7} | {:>7} | {:>7.3} s | {:>11.3} s | {:>10.2} |",
            L::NAME,
            threads,
            wall.as_secs_f64(),
            charged.as_secs_f64(),
            charged.as_secs_f64() / wall.as_secs_f64(),
        );
    }

    pub(super) fn main() {
        let cores = thread::available_parallelism().map_or(4, NonZero::get);

        println!("| lock    | threads |      wall |   CPU charged | cores busy |");
        println!("|---------|---------|-----------|---------------|------------|");

        for threads in [2, cores, 4 * cores] {
            measure::<quiesce::Mutex<u64>>(threads);
            measure::<std::sync::Mutex<u64>>(threads);
            measure::<spin::Mutex<u64>>(threads);
        }
    }
}
