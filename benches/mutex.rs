// Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
// SPDX-License-Identifier: Apache-2.0
// See LICENSE in the repository root for full license text.

use std::hint::black_box;
use std::num::NonZero;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

trait Lock: Send + Sync + 'static {
    const NAME: &'static str;

    fn new() -> Self;

    fn increment(&self);
}

impl Lock for quiesce::Mutex<u64> {
    const NAME: &'static str = "quiesce";

    fn new() -> Self {
        quiesce::Mutex::new(0)
    }

    fn increment(&self) {
        *self.lock() += 1;
    }
}

impl Lock for std::sync::Mutex<u64> {
    const NAME: &'static str = "std";

    fn new() -> Self {
        std::sync::Mutex::new(0)
    }

    fn increment(&self) {
        *self.lock().expect("Infallible: no holder panics here") += 1;
    }
}

impl Lock for spin::Mutex<u64> {
    const NAME: &'static str = "spin";

    fn new() -> Self {
        spin::Mutex::new(0)
    }

    fn increment(&self) {
        *self.lock() += 1;
    }
}

fn cores() -> usize {
    thread::available_parallelism().map_or(4, NonZero::get)
}

fn uncontended<L: Lock>(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("uncontended");
    let lock = L::new();

    group.throughput(Throughput::Elements(1));
    group.bench_function(L::NAME, |bencher| {
        bencher.iter(|| black_box(&lock).increment());
    });

    group.finish();
}

fn contended_for(threads: usize, lock: &Arc<impl Lock>, iterations: u64) -> Duration {
    let start = Arc::new(Barrier::new(threads + 1));

    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let lock = Arc::clone(lock);
            let start = Arc::clone(&start);

            thread::spawn(move || {
                start.wait();

                for _ in 0..iterations {
                    lock.increment();
                }
            })
        })
        .collect();

    start.wait();
    let started = Instant::now();

    for handle in handles {
        handle.join().expect("a thread of the benchmark panicked");
    }

    started.elapsed()
}

fn contended<L: Lock>(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("contended");

    for threads in [2, 4, cores(), 4 * cores()] {
        let lock = Arc::new(L::new());

        group.throughput(Throughput::Elements(threads as u64));
        group.bench_with_input(
            BenchmarkId::new(L::NAME, format!("{threads} threads")),
            &threads,
            |bencher, &threads| {
                bencher.iter_custom(|iterations| contended_for(threads, &lock, iterations));
            },
        );
    }

    group.finish();
}

fn every_lock(criterion: &mut Criterion) {
    uncontended::<quiesce::Mutex<u64>>(criterion);
    uncontended::<std::sync::Mutex<u64>>(criterion);
    uncontended::<spin::Mutex<u64>>(criterion);

    contended::<quiesce::Mutex<u64>>(criterion);
    contended::<std::sync::Mutex<u64>>(criterion);
    contended::<spin::Mutex<u64>>(criterion);
}

criterion_group!(benches, every_lock);
criterion_main!(benches);
