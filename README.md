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

## Credits

The three-state lock is the one Mara Bos builds in *Rust Atomics and Locks*,
chapter 9, and the one in the standard library. Which call to make on each
platform was learned from [atomic-wait](https://github.com/m-ou-se/atomic-wait),
by the same author.

## License

Apache-2.0.
