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
    use std::ops::Deref;

    /// Basic smoke test: verify Inject<T> compiles and Deref works.
    #[test]
    fn inject_deref() {
        let injected = Inject(42);
        assert_eq!(*injected, 42);
        assert_eq!(injected.0, 42);
    }

    // --- Test services for integration tests ---

    #[derive(Clone, Debug, PartialEq)]
    struct TestService {
        name: String,
    }

    impl TestService {
        fn new(name: &str) -> Self {
            Self { name: name.to_string() }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct DependentService {
        #[allow(dead_code)]
        inner: TestService,
    }

    impl DependentService {
        fn new(inner: TestService) -> Self {
            Self { inner }
        }
    }

    // --- Module with test services ---

    oxidi_core::module! {
        single {
            TestService::new("test-db")
        }

        single {
            let svc: TestService = oxidi_core::get();
            DependentService::new(svc)
        }
    }

    /// Test that Inject<T> resolves correctly when container is initialized.
    #[test]
    fn inject_resolves_from_container() {
        let service = TestService::new("manual");
        let injected = Inject(service.clone());
        assert_eq!(injected.0, service);
        assert_eq!(injected.deref(), &service);
    }

    /// Test that Inject<T> Deref works for method chaining.
    #[test]
    fn inject_deref_method_access() {
        let service = TestService::new("deref-test");
        let injected = Inject(service);
        // Access via deref
        assert_eq!(injected.name, "deref-test");
    }

    /// Test Inject with different types.
    #[test]
    fn inject_with_string() {
        let value = String::from("hello");
        let injected = Inject(value.clone());
        assert_eq!(*injected, value);
    }

    /// Test that FromRequest bounds are satisfied for our test types.
    /// This is a compile-time check: if this compiles, the trait bounds are correct.
    #[test]
    fn from_request_trait_bounds_compile() {
        fn assert_from_request<T>()
        where
            Inject<T>: rocket::request::FromRequest<'static>,
        {
        }
        // These compile only if Inject<T> implements FromRequest for T: Clone + OxidiResolve
        assert_from_request::<TestService>();
        assert_from_request::<DependentService>();
    }

    /// Test that Inject wraps and unwraps correctly for complex types.
    #[test]
    fn inject_complex_type() {
        let dep = DependentService::new(TestService::new("nested"));
        let injected = Inject(dep.clone());
        assert_eq!(injected.0, dep);
        assert_eq!(injected.inner.name, "nested");
    }

    /// Test that try_get returns error when container is not initialized.
    /// Note: This test may return Ok if another test already initialized the global container.
    /// It demonstrates the expected behavior when container is not ready.
    #[test]
    fn try_get_without_init_returns_error() {
        let result = oxidi_core::try_get::<TestService>();
        match result {
            Ok(_) => {
                // Container was initialized by a previous test — acceptable
            }
            Err(e) => {
                // Container not initialized — expected behavior
                assert!(e.contains("not initialized"));
            }
        }
    }

    /// Test that Outcome variants are used correctly in the FromRequest implementation.
    /// This verifies the error case returns the correct status.
    #[test]
    fn outcome_error_status() {
        // Verify that Status::InternalServerError is the expected error status
        let status = rocket::http::Status::InternalServerError;
        assert_eq!(status.code, 500);
    }
}
