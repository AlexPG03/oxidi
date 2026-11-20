//! Oxidi integration for [Rocket](https://rocket.rs).
//!
//! This crate provides the `Inject<T>` extractor, which resolves Oxidi services
//! directly inside Rocket handler signatures — zero boilerplate.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use oxidi_rocket::Inject;
//! use oxidi_core::module;
//! use rocket::get;
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
//! fn health_check(Inject(repo): Inject<UserRepository>) -> &'static str {
//!     // Use `repo` directly — resolved from Oxidi container
//!     "OK"
//! }
//!
//! #[launch]
//! fn rocket() -> _ {
//!     __oxidi_init__();
//!
//!     rocket::build().mount("/", routes![health_check])
//! }
//! ```
//!
//! # How It Works
//!
//! `Inject<T>` implements Rocket's `FromRequest<'r>` extractor. When a handler
//! declares `Inject<T>` as a parameter, Rocket calls the extractor, which resolves
//! `T` from the global Oxidi container via `try_get::<T>()`.
//!
//! Unlike frameworks that require state wiring, Oxidi uses a global container —
//! so no `State` or `FromState` setup is needed.
//!
//! # Requirements
//!
//! - Call `__oxidi_init__()` **before** launching the Rocket instance.
//! - The service type `T` must implement `Clone + OxidiResolve`.
//! - For thread-safe sharing without cloning, wrap services in `Arc<T>`.

use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use oxidi_core::{OxidiResolve, try_get};

/// A Rocket extractor that resolves a service from the Oxidi container.
///
/// `Inject<T>` implements `FromRequest<'r>` for any type `T` that
/// implements `Clone + OxidiResolve`. When used in a handler signature,
/// it automatically resolves the service from the global Oxidi container.
///
/// # Usage
///
/// ```rust,ignore
/// #[get("/users")]
/// fn list_users(Inject(repo): Inject<UserRepository>) -> Json<Vec<User>> {
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
/// fn health(Inject(repo): Inject<UserRepository>) -> String {
///     repo.count().to_string()
/// }
///
/// // Or use Deref
/// #[get("/health")]
/// fn health(injected: Inject<UserRepository>) -> String {
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

#[rocket::async_trait]
impl<'r, T> FromRequest<'r> for Inject<T>
where
    T: Clone + OxidiResolve + 'static,
{
    type Error = &'static str;

    async fn from_request(_request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match try_get::<T>() {
            Ok(service) => Outcome::Success(Inject(service)),
            Err(e) => Outcome::Error((Status::InternalServerError, e)),
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
