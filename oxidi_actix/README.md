# oxidi_actix

Actix-web integration for the [Oxidi](https://crates.io/crates/oxidi_core) zero-cost dependency injection framework.

## Overview

This crate provides the `Inject<T>` extractor for [Actix-web](https://actix.rs), allowing you to resolve Oxidi services directly in handler signatures with zero boilerplate.

## Quick Start

Add `oxidi_actix` to your `Cargo.toml`:

```toml
[dependencies]
oxidi_actix = "0.7"
oxidi_core = "0.7"
actix-web = "4"
```

Define your DI module and use `Inject<T>` in handlers:

```rust
use oxidi_actix::Inject;
use oxidi_core::get;
use actix_web::{get, web, App, HttpServer};

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
async fn health_check(Inject(repo): Inject<UserRepository>) -> &'static str {
    "OK"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    __oxidi_init__();

    HttpServer::new(|| {
        App::new().service(health_check)
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}
```

## How It Works

`Inject<T>` implements Actix-web's `FromRequest` trait. When a handler declares `Inject<T>` as a parameter, Actix calls the extractor, which resolves `T` from the global Oxidi container via `try_get::<T>()`.

No `web::Data` or `App::data()` wiring is needed — Oxidi uses a global container that is accessible from anywhere.

## Requirements

- Call `__oxidi_init__()` **before** starting the Actix-web server.
- The service type `T` must implement `Clone + OxidiResolve`.
- For thread-safe sharing without cloning, wrap services in `Arc<T>`.

## License

MIT
