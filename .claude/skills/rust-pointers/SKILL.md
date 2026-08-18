---
name: rust-pointers
description: Choosing the right Rust pointer/container - &T, Box, Rc/Arc, Cell/RefCell, Mutex/RwLock, OnceCell/OnceLock - with Send/Sync thread-safety rules. Load when picking a smart pointer, sharing state across threads, or debugging Send/Sync bounds.
---

## Thread safety basics

- `Send`: the value can move to another thread. `Sync`: it can be referenced from multiple threads (`&T` is `Send`).
- A pointer is only as thread-safe as the data behind it - the bounds below are all conditional on `T`.

## Selection table

| Type | Use for | `Send` when | `Sync` when |
|------|---------|-------------|-------------|
| `&T` | Shared read-only access | `T: Sync` | `T: Sync` |
| `&mut T` | Exclusive mutation | `T: Send` | `T: Sync` |
| `Box<T>` | Single-owner heap data; recursive types; large structs | `T: Send` | `T: Sync` |
| `Rc<T>` | Multiple owners, single thread | Never | Never |
| `Arc<T>` | Multiple owners across threads (`Arc<[T]>` for shared slices) | `T: Send + Sync` | `T: Send + Sync` |
| `Cell<T>` | Interior mutability for `Copy` types, single thread | `T: Send` | Never |
| `RefCell<T>` | Interior mutability, runtime-checked borrows, single thread - **can panic** | `T: Send` | Never |
| `Mutex<T>` | Exclusive mutable access across threads (usually `Arc<Mutex<T>>`) | `T: Send` | `T: Send` |
| `RwLock<T>` | Many readers OR one writer across threads (usually in `Arc`) | `T: Send` | `T: Send + Sync` |
| `OnceCell<T>` | One-time init, single thread | `T: Send` | Never |
| `OnceLock<T>` | One-time init in a `static`, thread-safe | `T: Send` | `T: Send + Sync` |
| `*const T` / `*mut T` | FFI / raw memory only, `unsafe` | Never | Never |

Raw pointers are `!Send`/`!Sync` by default; a wrapper type that guarantees safety may `unsafe impl` them.

## Rules of thumb

- Default to `&T` / `&mut T`; reach for smart pointers only when ownership or sharing demands them.
- Recursive enums/structs need `Box` (note `Vec` is already heap-allocated - `Multi(Vec<T>)` needs no extra box).
- Escalation ladder for shared mutable state: single thread `Cell` (Copy) → `RefCell`; multi-thread `Arc<Mutex<T>>` → `Arc<RwLock<T>>` if reads dominate.
- `RefCell` enforces borrow rules at **runtime**: holding a `borrow()` while taking `borrow_mut()` panics.
- Global/lazy statics: `OnceLock` with `get_or_init` covers both set-once and lazy closure-based init. (`LazyCell`/`LazyLock` need Rust 1.80 and this crate's MSRV is 1.74.)

```rust
static CONFIG: OnceLock<HashMap<String, Value>> = OnceLock::new();

fn config() -> &'static HashMap<String, Value> {
    CONFIG.get_or_init(|| read_config().into())
}
```

Further reading: [Mara Bos - Rust Atomics and Locks](https://marabos.nl/atomics/).
