# quiesce

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

Anything else fails to compile. Each platform listed runs the whole suite in
CI, including the test that measures the CPU a waiter is charged while it
waits.

## Credits

The three-state lock is the one in Mara Bos's *Rust Atomics and Locks*,
chapter 9, and in the standard library. Which call to make on each platform
was learned from [atomic-wait](https://github.com/m-ou-se/atomic-wait).

## License

Apache-2.0.
