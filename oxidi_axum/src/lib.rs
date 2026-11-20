//! Oxidi integration for [Axum](https://docs.rs/axum).
//!
//! This crate provides the `Inject<T>` extractor, which resolves Oxidi services
//! directly inside Axum handler signatures — zero boilerplate.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use oxidi_axum::Inject;
//! use oxidi_core::module;
//! use axum::routing::get;
//!
//! module! {
//!     single {
//!         Database::new("postgres://localhost")
//!     }
//!
//!     single {
//!         let db: Database = get();
//!         UserRepository::new(db)
//!     }
//! }
//!
//! async fn health_check(Inject(repo): Inject<UserRepository>) -> &'static str {
//!     // Use `repo` directly — resolved from Oxidi container
//!     "OK"
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     __oxidi_init__();
//!
//!     let app = axum::Router::new().route("/health", get(health_check));
//!     axum::serve(
//!         tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap(),
//!         app,
//!     )
//!     .await
//!     .unwrap();
//! }
//! ```
//!
//! # How It Works
//!
//! `Inject<T>` implements Axum's `FromRequestParts<S>` extractor. When a handler
//! declares `Inject<T>` as a parameter, Axum calls the extractor, which resolves
//! `T` from the global Oxidi container via `try_get::<T>()`.
//!
//! Unlike `shaku_axum`, which stores the module in Axum's `State`, Oxidi already
//! uses a global container — so no `State` wiring is needed.
//!
//! # Requirements
//!
//! - Call `__oxidi_init__()` **before** starting the Axum server.
//! - The service type `T` must implement `Clone + OxidiResolve`.
//! - For thread-safe sharing without cloning, wrap services in `Arc<T>`.

use std::ops::Deref;

use axum::http::StatusCode;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use oxidi_core::{OxidiResolve, try_get};

/// An Axum extractor that resolves a service from the Oxidi container.
///
/// `Inject<T>` implements `FromRequestParts<S>` for any type `T` that
/// implements `Clone + OxidiResolve`. When used in a handler signature,
/// it automatically resolves the service from the global Oxidi container.
///
/// # Usage
///
/// ```rust,ignore
/// async fn my_handler(Inject(db): Inject<Database>) -> String {
///     format!("Connected to {}", db.url())
/// }
/// ```
///
/// # Destructuring
///
/// The inner value is accessed via destructuring or `Deref`:
///
/// ```rust,ignore
/// // Destructure in the handler signature
/// async fn handler(Inject(repo): Inject<UserRepository>) -> String {
///     repo.count().to_string()
/// }
///
/// // Or use Deref
/// async fn handler(injected: Inject<UserRepository>) -> String {
///     injected.count().to_string()
/// }
/// ```
///
/// # Errors
///
/// If the Oxidi container is not initialized (i.e., `__oxidi_init__()` was
/// not called), the extractor returns an HTTP 500 Internal Server Error.
pub struct Inject<T>(pub T);

impl<T> Deref for Inject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> FromRequestParts<S> for Inject<T>
where
    T: Clone + OxidiResolve,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match try_get::<T>() {
            Ok(service) => Ok(Inject(service)),
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic smoke test: verify Inject<T> compiles and Deref works.
    #[test]
    fn inject_deref() {
        let injected = Inject(42);
        assert_eq!(*injected, 42);
        assert_eq!(injected.0, 42);
    }
}
