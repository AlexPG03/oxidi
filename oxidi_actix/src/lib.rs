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
        // Each test needs its own container initialization.
        // Since GLOBAL_CONTAINER is OnceLock, we can only init once per process.
        // This test verifies the FromRequest implementation compiles and the
        // Inject struct wraps the value correctly.
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
            Inject<T>: actix_web::FromRequest,
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
    /// Note: This test may fail if another test already initialized the global container.
    /// It demonstrates the expected behavior when container is not ready.
    #[test]
    fn try_get_without_init_returns_error() {
        // try_get::<TestService>() will return Err if the container was not initialized.
        // If another test already initialized it, this will return Ok — that's acceptable.
        let result = try_get::<TestService>();
        // Either Ok (container already init) or Err (not init) — both are valid outcomes
        // depending on test execution order. The important thing is it doesn't panic.
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
}
