# oxidi_axum

> **Axum integration for Oxidi** — inject Oxidi services directly into Axum handlers as extractors.

[![Crates.io](https://img.shields.io/crates/v/oxidi_axum.svg)](https://crates.io/crates/oxidi_axum)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/oxidi_axum)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

`oxidi_axum` provides the `Inject<T>` extractor for [Axum](https://docs.rs/axum), allowing services registered in [Oxidi](https://crates.io/crates/oxidi_core) to be resolved automatically in handler signatures.

## Installation

```bash
cargo add oxidi_axum oxidi_core
```

## Quick Start

```rust
use oxidi_axum::Inject;
use oxidi_core::module;
use axum::routing::get;

module! {
    single {
        Database::new("postgres://localhost")
    }

    single {
        let db: Database = get();
        UserRepository::new(db)
    }
}

async fn health_check(Inject(repo): Inject<UserRepository>) -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    __oxidi_init__();

    let app = axum::Router::new().route("/health", get(health_check));
    axum::serve(
        tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap(),
        app,
    )
    .await
    .unwrap();
}
```

## How It Works

`Inject<T>` implements Axum's `FromRequestParts<S>` extractor. When a handler declares `Inject<T>` as a parameter, Axum calls the extractor, which resolves `T` from the global Oxidi container via `try_get::<T>()`.

Unlike `shaku_axum`, which stores the module in Axum's `State`, Oxidi already uses a global container — so no `State` wiring is needed.

## API

- **`Inject<T>`** — An extractor that resolves `T` from the Oxidi container. Implements `Deref` for convenient access to the inner value.

## Requirements

- Call `__oxidi_init__()` **before** starting the Axum server.
- The service type `T` must implement `Clone + OxidiResolve`.

## License

MIT
