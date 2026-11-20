//! Oxidi integration for [Actix-web](https://actix.rs).
//!
//! This crate provides the `Inject<T>` extractor, which resolves Oxidi services
//! directly inside Actix-web handler signatures — zero boilerplate.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use oxidi_actix::Inject;
//! use oxidi_core::module;
//! use actix_web::{get, web, App, HttpServer};
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
//! #[get("/health")]
//! async fn health_check(Inject(repo): Inject<UserRepository>) -> &'static str {
//!     // Use `repo` directly — resolved from Oxidi container
//!     "OK"
//! }
//!
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!     __oxidi_init__();
//!
//!     HttpServer::new(|| {
//!         App::new().service(health_check)
//!     })
//!     .bind("127.0.0.1:3000")?
//!     .run()
//!     .await
//! }
//! ```
//!
//! # How It Works
//!
//! `Inject<T>` implements Actix-web's `FromRequest` extractor. When a handler
//! declares `Inject<T>` as a parameter, Actix calls the extractor, which resolves
//! `T` from the global Oxidi container via `try_get::<T>()`.
//!
//! Unlike frameworks that require state wiring, Oxidi uses a global container —
//! so no `web::Data` or `App::data()` setup is needed.
//!
//! # Requirements
//!
//! - Call `__oxidi_init__()` **before** starting the Actix-web server.
//! - The service type `T` must implement `Clone + OxidiResolve`.
//! - For thread-safe sharing without cloning, wrap services in `Arc<T>`.

use std::ops::Deref;
use std::future::{ready, Ready};

use actix_web::{Error, HttpRequest, dev::Payload};
use actix_web::error::ErrorInternalServerError;

use oxidi_core::{OxidiResolve, try_get};

/// An Actix-web extractor that resolves a service from the Oxidi container.
///
/// `Inject<T>` implements `FromRequest` for any type `T` that
/// implements `Clone + OxidiResolve`. When used in a handler signature,
/// it automatically resolves the service from the global Oxidi container.
///
/// # Usage
///
/// ```rust,ignore
/// #[get("/users")]
/// async fn list_users(Inject(repo): Inject<UserRepository>) -> Json<Vec<User>> {
///     Json(repo.list())
/// }
/// ```
///
/// # Destructuring
///
/// The inner value is accessed via destructuring or `Deref`:
///
/// ```rust,ignore
/// // Destructure in the handler signature
/// #[get("/health")]
/// async fn health(Inject(repo): Inject<UserRepository>) -> String {
///     repo.count().to_string()
/// }
///
/// // Or use Deref
/// #[get("/health")]
/// async fn health(injected: Inject<UserRepository>) -> String {
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

impl<T> actix_web::FromRequest for Inject<T>
where
    T: Clone + OxidiResolve,
{
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        match try_get::<T>() {
            Ok(service) => ready(Ok(Inject(service))),
            Err(e) => ready(Err(ErrorInternalServerError(e))),
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
