# quiesce

[![CI](https://github.com/memparanoid/quiesce/actions/workflows/ci.yml/badge.svg)](https://github.com/memparanoid/quiesce/actions/workflows/ci.yml)

A `no_std` mutex whose waiters sleep in the kernel instead of spinning.

```rust
use quiesce::Mutex;

let buffer = Mutex::new(Vec::<u8>::new());

buffer.lock().extend_from_slice(b"held by one caller at a time");
```

A waiter that finds the lock held spins briefly and then sleeps until the
holder lets go, so a lock held for milliseconds costs the waiters no CPU. There
is no poisoning: a holder that panics releases the lock on the way out.

## Platforms

| Platform | Waits with |
|---|---|
| Linux x86_64, aarch64 (glibc and musl) | the `futex` syscall, issued directly |
| macOS 14.4 and later | `os_sync_wait_on_address` / `os_sync_wake_by_address_any` |
| Windows 8 and later | `WaitOnAddress` / `WakeByAddressSingle` |

Anything else fails to compile.

## Testing

CI runs the whole suite on every platform listed, debug and release, and fails
any platform whose coverage drops below 100% of functions, lines and regions.
Windows on aarch64 is the one platform whose coverage is not measured: the
compiler writes profiles there that LLVM cannot read
([rust-lang/rust#150123](https://github.com/rust-lang/rust/issues/150123)).

A waiter is measured asleep, not assumed: the CPU time the operating system
charges a thread that waits half a second for the lock stays under a tenth of
it. The meter is held to both answers first, charging a thread that spins and
not one that sleeps.

`tests/concurrency.rs` runs four holders per core against one mutex:

- **One holder inside at a time**, counted by an atomic the mutex never touches.
- **No update lost** to a read, a yield and a write under the lock.
- **Every write handed over whole**: a row written in two halves with a yield
  between them is never seen half written by the next holder.
- **Every sleeper woken**: holders sleep with the lock held, so the waiters
  sleep in the kernel and each one needs its wake; a lost one hangs the test
  until its deadline fails it.
- **Panics survived**: holders panic while they hold the lock, the others go on,
  and the count comes out exact.

## Benchmarks

Measured on a Beelink SER8: AMD Ryzen 7 8745HS (8 cores, 16 threads), 64 GB,
NixOS 25.05, Linux 6.12, rustc 1.94.1. Against `std::sync::Mutex` and
`spin::Mutex` 0.12.

`cargo bench --bench mutex`, a lock and an increment of the value it holds:

| | quiesce | std | spin |
|---|---:|---:|---:|
| one thread | 3.43 ns | 3.45 ns | 1.71 ns |
| 2 threads | 37.5 M/s | 35.3 M/s | 71.6 M/s |
| 4 threads | 25.6 M/s | 21.3 M/s | 43.8 M/s |
| 16 threads | 46.8 M/s | 33.1 M/s | 9.9 M/s |
| 64 threads | 45.6 M/s | 32.1 M/s | 10.5 M/s |

`cargo bench --bench waiting`, every thread taking the lock 500 times and
sleeping 50 µs while it holds it, with the CPU time each thread is charged
summed:

| lock | threads | wall | CPU charged | cores busy |
|---|---:|---:|---:|---:|
| quiesce | 2 | 0.106 s | 0.004 s | 0.04 |
| std | 2 | 0.108 s | 0.010 s | 0.09 |
| spin | 2 | 0.105 s | 0.072 s | 0.69 |
| quiesce | 16 | 0.852 s | 0.056 s | 0.07 |
| std | 16 | 0.851 s | 0.075 s | 0.09 |
| spin | 16 | 0.876 s | 8.391 s | 9.58 |
| quiesce | 64 | 3.420 s | 0.256 s | 0.07 |
| std | 64 | 3.416 s | 0.338 s | 0.10 |
| spin | 64 | 4.956 s | 65.487 s | 13.21 |

## Maintenance

The crate is small and meant to stay that way, so it is not expected to change
often. A long gap between releases does not mean it is abandoned: it is
maintained, CI runs on every change, and issues are answered.

## Credits

The three-state lock is the one Mara Bos builds in *Rust Atomics and Locks*,
chapter 9, and the one in the standard library. Which call to make on each
platform was learned from [atomic-wait](https://github.com/m-ou-se/atomic-wait),
by the same author.

## License

Apache-2.0.
