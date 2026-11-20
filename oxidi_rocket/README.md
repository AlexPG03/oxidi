# oxidi_rocket

Rocket integration for the [Oxidi](https://crates.io/crates/oxidi_core) zero-cost dependency injection framework.

## Overview

This crate provides the `Inject<T>` extractor for [Rocket](https://rocket.rs), allowing you to resolve Oxidi services directly in handler signatures with zero boilerplate.

## Quick Start

Add `oxidi_rocket` to your `Cargo.toml`:

```toml
[dependencies]
oxidi_rocket = "0.7"
oxidi_core = "0.7"
rocket = { version = "0.5", features = ["json"] }
```

Define your DI module and use `Inject<T>` in handlers:

```rust
use oxidi_rocket::Inject;
use oxidi_core::get;
use rocket::get;

oxidi_core::module! {
    single {
        Database::new("postgres://localhost")
    }

    single {
        let db: Database = get();
        UserRepository::new(db)
    }
}

#[get("/health")]
fn health_check(Inject(repo): Inject<UserRepository>) -> &'static str {
    "OK"
}

#[launch]
fn rocket() -> _ {
    __oxidi_init__();

    rocket::build().mount("/", routes![health_check])
}
```

## How It Works

`Inject<T>` implements Rocket's `FromRequest<'r>` trait. When a handler declares `Inject<T>` as a parameter, Rocket calls the extractor, which resolves `T` from the global Oxidi container via `try_get::<T>()`.

No `State` or `FromState` wiring is needed — Oxidi uses a global container that is accessible from anywhere.

## Requirements

- Call `__oxidi_init__()` **before** launching the Rocket instance.
- The service type `T` must implement `Clone + OxidiResolve`.
- For thread-safe sharing without cloning, wrap services in `Arc<T>`.

## License

MIT
