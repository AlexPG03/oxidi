# Oxidi

> **The simplest zero-cost dependency injection for Rust.** Inspired by [Koin](https://insert-koin.io/) (Kotlin), Oxidi provides a declarative DI container where **everything is resolved at compile time** — no `dyn Trait`, no virtual dispatch, no runtime overhead.

[![Crates.io](https://img.shields.io/crates/v/oxidi_core.svg)](https://crates.io/crates/oxidi_core)
[![Downloads](https://img.shields.io/crates/d/oxidi_core.svg)](https://crates.io/crates/oxidi_core)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/oxidi_core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## Table of Contents

- [Why Oxidi?](#why-oxidi)
- [Comparison with Alternatives](#comparison-with-alternatives)
- [Features](#features)
- [Quick Start](#quick-start)
- [Use Case](#use-case)
- [Detailed Guide](#detailed-guide)
- [Scoped Services](#scoped-services)
- [Testing Support](#testing-support)
- [Axum Integration](#axum-integration)
- [Rocket Integration](#rocket-integration)
- [Actix-web Integration](#actix-web-integration)
- [API Reference](#api-reference)
- [Architecture](#architecture)
- [Plugin System](#plugin-system)
- [Roadmap](#roadmap)
- [IDE Compatibility](#ide-compatibility)
- [License](#license)

---

## Why Oxidi?

Building Rust applications requires wiring dependencies through constructors, creating boilerplate that grows with your codebase. Other languages solved this with DI frameworks like Spring (Java) or Koin (Kotlin). In Rust, the options are limited:

| Approach | Problem |
|---|---|
| **Manual wiring** | Tedious, error-prone, hard to refactor |
| **shaku** | Uses `dyn Trait` + virtual dispatch at runtime; requires `Interface` trait on all types |
| **more-di** | Heavy API with builders, descriptors, and multiple features to learn |
| **No DI** | Pass containers through every function parameter |

**Oxidi takes a different approach:** it generates plain Rust code at compile time. No runtime container, no hash maps, no `dyn Trait` bounds, no virtual dispatch. The generated code is what you would have written by hand — a struct with fields and direct function calls — meaning the optimizer can inline and eliminate everything.

### What makes it different

- **100% compile-time resolution** — unlike shaku, which stores components as `dyn Interface` and resolves them at runtime, Oxidi generates direct field access.
- **No trait bounds on your types** — you don't need to implement `Interface`, `Component`, or `Provider`. Your structs stay clean.
- **Minimal API** — one macro (`module!`), one function (`get()`), one init call (`__oxidi_init__()`). No builders, no descriptors, no feature flags.
- **Koin-inspired DSL** — if you've used Koin in Kotlin, the syntax feels familiar.

---

## Comparison with Alternatives

| Feature | **Oxidi** | [shaku](https://crates.io/crates/shaku) | [more-di](https://crates.io/crates/more-di) |
|---|---|---|---|
| **Resolution** | Compile-time (generated code) | Runtime (`dyn Trait` lookup) | Runtime (service collection) |
| **Virtual dispatch** | ❌ None | ✅ `dyn Interface` | ✅ `dyn Trait` |
| **Trait bounds on types** | ❌ None required | ✅ `Interface + Component` | ✅ `#[injectable]` |
| **API surface** | 1 macro + 1 function | Module builders, providers, interfaces | ServiceCollection, descriptors, builders |
| **Cycle detection** | ✅ Compile-time | ✅ Build-time | ✅ Registration-time |
| **Scoped services** | ✅ `scoped` + `scope()` | ✅ Sub-modules | ✅ Scoped providers |
| **Framework integration** | ✅ Axum, Rocket, Actix-web | ✅ Axum, Rocket | ❌ |
| **Learning curve** | Low | Medium | High |
| **Runtime overhead** | Zero | `Box<dyn Trait>` per resolution | Service lookup per resolution |

> **Choose Oxidi if:** you want the simplest possible API with zero runtime cost, scoped services, and Axum/Rocket/Actix-web integration.
> **Choose shaku if:** you need sub-module composition.
> **Choose more-di if:** you need keyed services.

---

## Features

| Feature | Description |
|---|---|
| **Singleton services** | `single { ... }` — one shared instance across the application |
| **Factory services** | `factory { ... }` — new instance created on every `get()` call |
| **Scoped services** | `scoped { ... }` — one instance per scope, managed via `scope()` |
| **Zero-cost resolution** | `get::<T>()` generates direct field access, no virtual dispatch |
| **No trait bounds** | Your types don't need to implement any framework trait |
| **Automatic dependency ordering** | Topological sort ensures services are created in the correct order |
| **Cycle detection** | Circular dependencies are caught at compile time with clear error messages |
| **Missing dependency detection** | Unresolved `get::<T>()` calls produce compile-time errors |
| **Global container** | Access services from any function without passing a container |
| **Plugin system** | Extend the DSL with custom service types via `OxidiModulePlugin` |
| **IDE-friendly** | Module-level macro emission for full rust-analyzer support |

---

## Quick Start

Add `oxidi_core` to your `Cargo.toml`:

```toml
[dependencies]
oxidi_core = "0.1"
```

Define your services and the DI module:

```rust
use oxidi_core::get;

#[derive(Clone, Debug)]
pub struct Database {
    connection_string: String,
}

impl Database {
    pub fn new(conn: &str) -> Self {
        Self { connection_string: conn.to_string() }
    }
}

// 1. Define the module at the top level (outside any function)
oxidi_core::module! {
    single {
        Database::new("postgres://localhost:5432/mydb")
    }
}

fn main() {
    // 2. Initialize the container
    __oxidi_init__();

    // 3. Access services from anywhere
    let db: Database = get();
    println!("Connected to: {}", db.connection_string);
}
```

---

## Use Case

Imagine a web application with the following dependency graph:

```
RequestHandler
    ├── UserRepository
    │   └── Database (singleton)
    └── Logger (singleton)
```

Without DI, every function that needs a `RequestHandler` must receive it as a parameter, and `RequestHandler`'s constructor must receive `UserRepository` and `Logger`, which in turn need `Database`. This creates a **dependency chain** that propagates through your entire codebase.

With Oxidi, you declare the graph once and access any service from anywhere:

```rust
use oxidi_core::get;

// --- Your service types ---

#[derive(Clone, Debug)]
pub struct Database { connection_string: String }

impl Database {
    pub fn new(conn: &str) -> Self {
        Self { connection_string: conn.to_string() }
    }
}

#[derive(Clone, Debug)]
pub struct UserRepository { database: Database }

impl UserRepository {
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[derive(Clone, Debug)]
pub struct Logger;

impl Logger {
    pub fn new() -> Self { Self }
}

#[derive(Clone, Debug)]
pub struct RequestHandler { repo: UserRepository, logger: Logger }

impl RequestHandler {
    pub fn new(repo: UserRepository, logger: Logger) -> Self {
        Self { repo, logger }
    }
}

// --- DI Module Definition ---

oxidi_core::module! {
    single {
        Logger::new()
    }

    single {
        Database::new("postgres://localhost:5432/mydb")
    }

    single {
        let db: Database = get();
        UserRepository::new(db)
    }

    factory {
        let repo: UserRepository = get();
        let logger: Logger = get();
        RequestHandler::new(repo, logger)
    }
}

// --- Application Code ---

fn handle_http_request() {
    // No container parameter needed — just get what you need
    let handler: RequestHandler = get();
    // ... process the request
}

fn show_db_status() {
    let db: Database = get();
    println!("Database: {}", db.connection_string);
}

fn main() {
    __oxidi_init__();
    handle_http_request();
    show_db_status();
}
```

**Key observations:**

- `Database` and `Logger` are **singletons** — created once, shared everywhere.
- `RequestHandler` is a **factory** — a new instance is created every time `get::<RequestHandler>()` is called, with fresh references to the singleton dependencies.
- Functions like `handle_http_request()` and `show_db_status()` access services **without any container parameter**.
- The dependency order is **automatically resolved**: `Database` is created before `UserRepository`, which is created before `RequestHandler`.

---

## Detailed Guide

### Installation

```toml
[dependencies]
oxidi_core = "0.1"
```

> **Note:** `oxidi_core` re-exports the `module!` macro and `get()` function. You do not need to depend on `oxidi_macros` directly.

### Defining Services

Every service must implement `Clone` so it can be shared between consumers:

```rust
#[derive(Clone)]
struct MyService { /* fields */ }
```

For thread-safe sharing without cloning, wrap services in `Arc<T>`:

```rust
use std::sync::Arc;

#[derive(Clone)]  // Arc<T> implements Clone
struct MyService {
    inner: Arc<HeavyResource>,
}
```

### The `module!` Macro

The `module!` macro must be placed at the **module level** (outside any function). It accepts a block containing service definitions:

```rust
oxidi_core::module! {
    single { /* ... */ }
    factory { /* ... */ }
}
```

#### `single` — Singleton Services

A singleton is created **once** when the container is initialized and shared across all `get()` calls:

```rust
oxidi_core::module! {
    single {
        Database::new("postgres://localhost:5432/mydb")
    }
}
```

#### `factory` — Factory Services

A factory creates a **new instance** every time `get()` is called:

```rust
oxidi_core::module! {
    factory {
        RequestHandler::new(get::<UserRepository>())
    }
}
```

This is useful for request-scoped objects, controllers, or any service that should not be shared.

#### `scoped` — Scoped Services

A scoped service is created **once per scope** and lives only during that scope. Use `scope(|| { ... })` to activate a scope:

```rust
use oxidi_core::{get, scope};

oxidi_core::module! {
    single {
        Database::new("postgres://localhost:5432/mydb")
    }

    scoped {
        let db: Database = get();
        Transaction::new(db)
    }
}

fn main() {
    __oxidi_init__();

    // Each call to `scope` creates a fresh Transaction
    scope(|| {
        let tx: Transaction = get(); // new instance, cached in this scope
        // ... use tx ...
    }); // tx is dropped here

    scope(|| {
        let tx: Transaction = get(); // another new instance
        // ... use tx ...
    });

    // Outside a scope, scoped services are not available
    let db: Database = get(); // global singleton still works
}
```

**Common use cases for scoped services:**

- **Database transactions** — one transaction per request, automatically committed/rolled back
- **Request context** — user session data, request IDs, tracing spans
- **Per-task state** — background jobs with isolated state

**Nested scopes** are supported: an inner scope shadows an outer scope for the same type, and the outer scope's instance is restored when the inner scope ends.

#### `async_scope()` — Async-Compatible Scopes

For async code with `.await` points, use `async_scope()` instead of `scope()`. It preserves the scope context across task migrations between threads, making it compatible with any async runtime (Tokio, async-std, smol, etc.):

```rust
use oxidi_core::{get, async_scope};

#[tokio::main]
async fn main() {
    __oxidi_init__();

    async_scope(async {
        let tx: Transaction = get();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        // tx is still available after .await
        let tx2: Transaction = get();
        assert_eq!(tx.id(), tx2.id());
    }).await;
}
```

The `scope()` function also works in async contexts as long as there are no `.await` points within the closure. Use `async_scope()` when you need to cross `.await` boundaries.

#### Resolving Dependencies

Inside a service block, use `get()` to reference other services:

```rust
single {
    let db: Database = get();           // Type inferred from `db`
    let logger = get::<Logger>();       // Explicit type annotation
    UserRepository::new(db, logger)
}
```

Both forms are equivalent. The type can be inferred from the variable annotation or specified explicitly with turbofish syntax.

### Initializing the Container

After defining the module, call `__oxidi_init__()` in `main()` (or your entry point) **before** any `get()` call:

```rust
fn main() {
    __oxidi_init__();  // Must be called exactly once

    let db: Database = get();  // Now safe to use
}
```

> **Important:** `__oxidi_init__()` is generated by the `module!` macro. Calling it more than once will panic.

### Accessing Services

Use `get<T>()` from anywhere in your code:

```rust
// Type inference
let db: Database = get();

// Explicit type
let handler = get::<RequestHandler>();
```

No container needs to be passed through function parameters — the container is stored globally using `OnceLock`.

### Error Handling

Oxidi catches errors at **compile time**:

#### Missing Dependencies

```rust
oxidi_core::module! {
    single {
        let repo: UserRepository = get();  // ERROR: no UserRepository defined
        RequestHandler::new(repo)
    }
}
```

#### Circular Dependencies

```rust
oxidi_core::module! {
    single {
        let b: ServiceB = get();
        ServiceA::new(b)
    }
    single {
        let a: ServiceA = get();  // ERROR: circular dependency A -> B -> A
        ServiceB::new(a)
    }
}
```

Both errors produce clear `compile_error!` messages indicating the problem and how to fix it.

---

## Testing Support

Oxidi provides a `test-support` feature for writing isolated unit tests with mock services. Enable it in your `Cargo.toml`:

```toml
[dependencies]
oxidi_core = { version = "0.1", features = ["test-support"] }
```

### Basic Override

Replace services with mocks using `TestContainer`:

```rust
use oxidi_core::test_support::{TestContainer, get_override};

#[test]
fn test_with_mock() {
    let container = TestContainer::builder()
        .with_override::<Database>(MockDatabase::new())
        .with_override::<Logger>(MockLogger::new())
        .build();

    // Resolve mock services from the test container
    let db: Database = get_override(&container);
    let logger: Logger = get_override(&container);

    assert!(db.is_mock());
}
```

### Full Container Integration

For services that depend on other services, use `build_with_container()`:

```rust
use oxidi_core::test_support::{TestContainer, get_from};

#[test]
fn test_with_full_container() {
    let container = TestContainer::builder()
        .with_override::<Database>(MockDatabase::new())
        .build_with_container(OxidiContainer::with_overrides);

    // Services that depend on Database will get the mock
    let repo: UserRepository = get_from(&container);
}
```

### Parallel Tests

Each `TestContainer` is fully isolated — tests can run in parallel without interference:

```rust
#[test]
fn test_parallel_1() {
    let container = TestContainer::builder()
        .with_override::<Database>(MockDatabase::new())
        .build();
    let db: Database = get_override(&container);
}

#[test]
fn test_parallel_2() {
    let container = TestContainer::builder()
        .with_override::<Database>(MockDatabase::new())
        .build();
    let db: Database = get_override(&container);
}
```

### API Overview

| Function | Description |
|---|---|
| `TestContainer::builder()` | Start configuring overrides |
| `.with_override::<T>(instance)` | Replace service `T` with a mock |
| `.build()` | Create a simple test container |
| `.build_with_container(fn)` | Create a full container with overrides |
| `get_override(&container)` | Get an overridden service (simple mode) |
| `get_from(&container)` | Resolve from full container (with dependencies) |
| `scope_with(&container, \|\| { ... })` | Use scopes with test containers |

> **Note:** The `test-support` feature has **zero overhead** in production — it's completely excluded from builds that don't enable it.

---

## Axum Integration

The [`oxidi_axum`](https://crates.io/crates/oxidi_axum) crate provides the `Inject<T>` extractor, which resolves Oxidi services directly inside Axum handler signatures.

### Installation

```toml
[dependencies]
oxidi_core = "0.1"
oxidi_axum = "0.1"
axum = "0.8"
tokio = { version = "1", features = ["full"] }
```

### Example

```rust
use oxidi_axum::Inject;
use oxidi_core::get;
use axum::{Json, Router, routing::get as axum_get};
use serde::Serialize;

#[derive(Clone)]
pub struct Database { connection_string: String }

impl Database {
    pub fn new(conn: &str) -> Self {
        Self { connection_string: conn.to_string() }
    }
}

#[derive(Clone)]
pub struct UserRepository { database: Database }

impl UserRepository {
    pub fn new(database: Database) -> Self { Self { database } }
    pub fn user_count(&self) -> usize { 42 }
}

oxidi_core::module! {
    single {
        Database::new("postgres://localhost:5432/mydb")
    }
    single {
        let db: Database = get();
        UserRepository::new(db)
    }
}

#[derive(Serialize)]
struct HealthResponse { status: String, users: usize }

async fn health_check(Inject(repo): Inject<UserRepository>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        users: repo.user_count(),
    })
}

#[tokio::main]
async fn main() {
    __oxidi_init__();

    let app = Router::new().route("/health", axum_get(health_check));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### How It Works

`Inject<T>` implements Axum's `FromRequestParts<S>` extractor. When a handler declares `Inject<T>` as a parameter, Axum calls the extractor, which resolves `T` from the global Oxidi container via `try_get::<T>()`.

Unlike `shaku_axum`, which stores the module in Axum's `State`, Oxidi already uses a global container — so no `State` wiring is needed.

### API

- **`Inject<T>`** — An extractor that resolves `T` from the Oxidi container. Implements `Deref` for convenient access to the inner value.

---

## Rocket Integration

The [`oxidi_rocket`](https://crates.io/crates/oxidi_rocket) crate provides the `Inject<T>` extractor, which resolves Oxidi services directly inside Rocket handler signatures.

### Installation

```toml
[dependencies]
oxidi_core = "0.1"
oxidi_rocket = "0.1"
rocket = { version = "0.5", features = ["json"] }
```

### Example

```rust
use oxidi_rocket::Inject;
use oxidi_core::get;
use rocket::{get as rocket_get, launch, build, routes};
use rocket::serde::json::Json;
use serde::Serialize;

#[derive(Clone)]
pub struct Database { connection_string: String }

impl Database {
    pub fn new(conn: &str) -> Self {
        Self { connection_string: conn.to_string() }
    }
}

#[derive(Clone)]
pub struct UserRepository { database: Database }

impl UserRepository {
    pub fn new(database: Database) -> Self { Self { database } }
    pub fn user_count(&self) -> usize { 42 }
}

oxidi_core::module! {
    single {
        Database::new("postgres://localhost:5432/mydb")
    }
    single {
        let db: Database = get();
        UserRepository::new(db)
    }
}

#[derive(Serialize)]
struct HealthResponse { status: String, users: usize }

#[rocket_get("/health")]
fn health_check(Inject(repo): Inject<UserRepository>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        users: repo.user_count(),
    })
}

#[launch]
fn rocket() -> _ {
    __oxidi_init__();

    build().mount("/", routes![health_check])
}
```

### How It Works

`Inject<T>` implements Rocket's `FromRequest<'r>` extractor. When a handler declares `Inject<T>` as a parameter, Rocket calls the extractor, which resolves `T` from the global Oxidi container via `try_get::<T>()`.

Unlike frameworks that require state wiring, Oxidi uses a global container — so no `State` or `FromState` setup is needed.

### API

- **`Inject<T>`** — An extractor that resolves `T` from the Oxidi container. Implements `Deref` for convenient access to the inner value.

---

## Actix-web Integration

The [`oxidi_actix`](https://crates.io/crates/oxidi_actix) crate provides the `Inject<T>` extractor, which resolves Oxidi services directly inside Actix-web handler signatures.

### Installation

```toml
[dependencies]
oxidi_core = "0.1"
oxidi_actix = "0.1"
actix-web = "4"
```

### Example

```rust
use oxidi_actix::Inject;
use oxidi_core::get;
use actix_web::{get as actix_get, web, App, HttpServer, Responder};
use serde::Serialize;

#[derive(Clone)]
pub struct Database { connection_string: String }

impl Database {
    pub fn new(conn: &str) -> Self {
        Self { connection_string: conn.to_string() }
    }
}

#[derive(Clone)]
pub struct UserRepository { database: Database }

impl UserRepository {
    pub fn new(database: Database) -> Self { Self { database } }
    pub fn user_count(&self) -> usize { 42 }
}

oxidi_core::module! {
    single {
        Database::new("postgres://localhost:5432/mydb")
    }
    single {
        let db: Database = get();
        UserRepository::new(db)
    }
}

#[derive(Serialize)]
struct HealthResponse { status: String, users: usize }

#[actix_get("/health")]
async fn health_check(Inject(repo): Inject<UserRepository>) -> impl Responder {
    actix_web::HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        users: repo.user_count(),
    })
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

### How It Works

`Inject<T>` implements Actix-web's `FromRequest` extractor. When a handler declares `Inject<T>` as a parameter, Actix calls the extractor, which resolves `T` from the global Oxidi container via `try_get::<T>()`.

Unlike frameworks that require state wiring, Oxidi uses a global container — so no `web::Data` or `App::data()` setup is needed.

### API

- **`Inject<T>`** — An extractor that resolves `T` from the Oxidi container. Implements `Deref` for convenient access to the inner value.

---

## API Reference

### `oxidi_core::module! { ... }`

Declares a DI module. Must be placed at module level.

| Keyword | Description |
|---|---|
| `single { ... }` | Registers a singleton service |
| `factory { ... }` | Registers a factory service |
| `scoped { ... }` | Registers a scoped service (one per scope) |
| `get::<T>()` | Resolves a dependency of type `T` |

### `oxidi_core::get::<T>() -> T`

Retrieves a service of type `T` from the global container.

**Requirements:**
- `T: Clone + OxidiResolve`
- `__oxidi_init__()` must have been called

**Panics** if the container has not been initialized.

### `__oxidi_init__()`

Initializes the global container. Generated automatically by `module!`. Call exactly once at application startup.

### `oxidi_core::scope(|| { ... })`

Activates a new scope around a closure. Inside the closure, `get()` checks the scope first for scoped services, then falls back to global singletons and factories. When the closure returns, the scope (and all scoped instances) are dropped.

**Requirements:**
- `__oxidi_init__()` must have been called before entering a scope.
- Scopes are thread-local: a scope created in one thread is not visible in another.

### `OxidiResolve` Trait

Implemented automatically for each service type by the `module!` macro. Do not implement manually.

### `OxidiModulePlugin` Trait

Convention trait for extending the DSL. External crates can implement this trait to add custom service types.

---

## Architecture

Oxidi consists of three crates:

```
oxidi/
├── oxidi_actix/         # Actix-web integration: Inject<T> extractor
├── oxidi_axum/          # Axum integration: Inject<T> extractor
├── oxidi_core/          # Runtime: get(), OxidiResolve, global container
├── oxidi_macros/        # Proc-macro: module!(), DSL parser, analyzer, codegen
├── oxidi_rocket/        # Rocket integration: Inject<T> extractor
└── oxidi_expample_app/  # Example application (CLI + Axum)
```

### Compilation Pipeline

```
module! { ... }
    │
    ▼
┌──────────────┐
│  DSL Parser   │  Parses `single` / `factory` / `scoped` blocks, extracts get::<T>() calls
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Analyzer     │  Builds dependency graph, detects cycles, topological sort
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Codegen      │  Generates OxidiContainer struct, factory methods,
│               │  OxidiResolve impls, and __oxidi_init__()
└──────┬───────┘
       │
       ▼
  Expanded Rust code (compiled by rustc)
```

### Generated Code

The `module!` macro generates:

1. **`OxidiContainer` struct** — holds all singleton services as fields
2. **Factory methods** — one per `factory` or `scoped` service, creates new instances on demand
3. **`OxidiResolve` impls** — one per service type, maps `get::<T>()` to the correct container field or factory method
4. **`__oxidi_init__()` function** — builds the container and stores it globally

All code is emitted at **module level** for full IDE (rust-analyzer) support.

---

## Plugin System

Oxidi supports extending the DSL through the `OxidiModulePlugin` trait. This allows third-party crates to introduce new service types:

```rust
// In a hypothetical oxidi_viewmodel crate:
use oxidi_core::OxidiModulePlugin;

pub struct ViewModelPlugin;

impl OxidiModulePlugin for ViewModelPlugin {
    const NAME: &'static str = "viewModel";

    fn generate_code(&self) -> proc_macro2::TokenStream {
        // Generate viewModel-specific code
        quote! { /* ... */ }
    }
}
```

This enables domain-specific extensions such as:
- **ViewModel** services for UI frameworks
- **Navigation** modules for routing
- **Background workers** for async tasks

---

## Roadmap

Oxidi is in its early stages. Here's what's planned:

### v0.2 (done)
- **Axum integration** — [`oxidi_axum`](https://crates.io/crates/oxidi_axum) crate with `Inject<T>` extractor for seamless Axum handler injection

### v0.3.1 (done)
- **Rocket integration** — [`oxidi_rocket`](https://crates.io/crates/oxidi_rocket) crate with `Inject<T>` extractor for seamless Rocket handler injection

### v0.3 (done)
- **Scoped services** — `scoped { ... }` keyword with `scope(|| { ... })` for per-scope lifetimes, including nested scope support
- **Async compatibility** — `async_scope()` for async code with `.await` points, compatible with any runtime (Tokio, async-std, smol)

### v0.5 (done)
- **Actix-web integration** — [`oxidi_actix`](https://crates.io/crates/oxidi_actix) crate with `Inject<T>` extractor for seamless Actix-web handler injection

### v0.6 (planned)
- **`Option<T>` support** — optional dependencies that may not be registered
- **Named/keyed services** — register multiple instances of the same type with different keys

### v0.7 (done)
- **Testing support** — `with_override()` to replace services with mocks in tests
- **TestContainer** — isolated container for parallel tests without touching the global container
- **`get_override()`** — resolve services from a test container
- **`build_with_container()`** — full integration with generated `OxidiContainer`

### v0.8 (planned)

### Long-term
- **Generic service support** — register services with type parameters
- **Async initialization** — `async { ... }` blocks in service definitions
- **Derive macro** — `#[OxidiComponent]` for automatic dependency inference

> Have a feature request? Open an issue on [GitHub](https://github.com/AlexPG03/oxidi/issues).

---

## IDE Compatibility

Oxidi is designed to work seamlessly with rust-analyzer. The `module!` macro emits all `impl OxidiResolve` blocks at the **module level**, allowing the IDE to index trait implementations correctly.

For optimal IDE experience, ensure your `rust-analyzer.toml` includes:

```toml
procMacro.enable = true
cargo.buildScripts.enable = true
```

---

## License

This project is licensed under the [MIT License](LICENSE).
