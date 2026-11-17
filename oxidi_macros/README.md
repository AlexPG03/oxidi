# Oxidi

> **Zero-cost dependency injection for Rust.** Inspired by [Koin](https://insert-koin.io/) (Kotlin), Oxidi provides a declarative, type-safe DI container with zero runtime reflection — everything is resolved at compile time through procedural macros.

[![Crates.io](https://img.shields.io/crates/v/oxidi_core.svg)](https://crates.io/crates/oxidi_core)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/oxidi_core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## Table of Contents

- [Vision](#vision)
- [Features](#features)
- [Quick Start](#quick-start)
- [Use Case](#use-case)
- [Detailed Guide](#detailed-guide)
- [API Reference](#api-reference)
- [Architecture](#architecture)
- [Plugin System](#plugin-system)
- [IDE Compatibility](#ide-compatibility)
- [License](#license)

---

## Vision

Building Rust applications often requires manually wiring dependencies through function parameters, leading to boilerplate-heavy code and tight coupling. Frameworks like Spring (Java) and Koin (Kotlin) solved this problem with **Dependency Injection containers** that manage object lifecycles and resolve dependencies automatically.

**Oxidi brings this paradigm to Rust** — without the cost. Unlike runtime-based DI frameworks that rely on reflection or dynamic dispatch, Oxidi generates all resolution code at **compile time** using procedural macros. The result is:

- **Zero runtime overhead**: no `Any` downcasting at runtime, no hash maps, no reflection.
- **Full type safety**: every `get::<T>()` call is verified by the compiler.
- **Declarative syntax**: define your entire service graph in a single, readable block.
- **Global access**: retrieve services from anywhere without passing containers through call chains.

Oxidi is designed for developers who want the productivity of a DI framework with the performance guarantees of Rust.

---

## Features

| Feature | Description |
|---|---|
| **Singleton services** | `single { ... }` — one shared instance across the application |
| **Factory services** | `factory { ... }` — new instance created on every `get()` call |
| **Type-safe resolution** | `get::<T>()` — compile-time verified, zero-cost dispatch |
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

## API Reference

### `oxidi_core::module! { ... }`

Declares a DI module. Must be placed at module level.

| Keyword | Description |
|---|---|
| `single { ... }` | Registers a singleton service |
| `factory { ... }` | Registers a factory service |
| `get::<T>()` | Resolves a dependency of type `T` |

### `oxidi_core::get::<T>() -> T`

Retrieves a service of type `T` from the global container.

**Requirements:**
- `T: Clone + OxidiResolve`
- `__oxidi_init__()` must have been called

**Panics** if the container has not been initialized.

### `__oxidi_init__()`

Initializes the global container. Generated automatically by `module!`. Call exactly once at application startup.

### `OxidiResolve` Trait

Implemented automatically for each service type by the `module!` macro. Do not implement manually.

### `OxidiModulePlugin` Trait

Convention trait for extending the DSL. External crates can implement this trait to add custom service types.

---

## Architecture

Oxidi consists of three crates:

```
oxidi/
├── oxidi_core/        # Runtime: get(), OxidiResolve, global container
├── oxidi_macros/      # Proc-macro: module!(), DSL parser, analyzer, codegen
└── oxidi_expample_app/ # Example application
```

### Compilation Pipeline

```
module! { ... }
    │
    ▼
┌──────────────┐
│  DSL Parser   │  Parses `single` / `factory` blocks, extracts get::<T>() calls
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
2. **Factory methods** — one per `factory` service, creates new instances on demand
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
