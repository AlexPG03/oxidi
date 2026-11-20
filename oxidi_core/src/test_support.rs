//! Test support utilities for Oxidi.
//!
//! This module provides tools for testing code that uses the Oxidi DI container.
//! It allows replacing services with mocks and running tests in isolation.
//!
//! # Features
//!
//! - **Service Override**: Replace any service in the container with a mock instance
//! - **Test Container**: Isolated container that doesn't touch the global container
//! - **Parallel Tests**: Each test gets its own container via thread-local storage
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use oxidi_core::test_support::{TestContainer, get_from};
//!
//! #[test]
//! fn test_with_mock() {
//!     // Create a test container with overrides
//!     let container = TestContainer::builder()
//!         .with_override::<Database>(MockDatabase::new())
//!         .build();
//!
//!     // Resolve services from the test container
//!     let db: Database = get_from(&container);
//!     assert!(db.is_mock());
//! }
//! ```
//!
//! # Parallel Tests
//!
//! ```rust,ignore
//! use oxidi_core::test_support::{TestContainer, scope_with};
//!
//! #[test]
//! fn test_parallel_1() {
//!     let container = TestContainer::builder()
//!         .with_override::<Database>(MockDatabase::new())
//!         .build();
//!
//!     // Use init() to set up thread-local container
//!     container.init();
//!
//!     // Now regular get() works with the test container
//!     let db: Database = oxidi_core::get();
//! }
//! // Container is automatically cleaned up when dropped
//! ```
//!
//! # Compatibility with mockall
//!
//! The override system works seamlessly with mockall-generated mocks:
//!
//! ```rust,ignore
//! use mockall::mock;
//! use oxidi_core::test_support::TestContainer;
//!
//! mock! {
//!     pub struct Database {}
//!
//!     impl Database for Database {
//!         fn connect(&self) -> Result<(), Error>;
//!     }
//! }
//!
//! #[test]
//! fn test_with_mockall() {
//!     let mut mock_db = MockDatabase::new();
//!     mock_db.expect_connect().returning(|| Ok(()));
//!
//!     let container = TestContainer::builder()
//!         .with_override::<Database>(mock_db)
//!         .build();
//!
//!     // Use the mock through the container
//!     let db: Database = get_from(&container);
//! }
//! ```

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use task_local::task_local;

use crate::OxidiResolve;

/// A map of overridden service instances, keyed by TypeId.
/// Uses Arc to allow cloning for thread-local storage.
///
/// This type is used by the generated `with_overrides` method on `OxidiContainer`.
pub type OverridesMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

/// Internal helper function called by generated code to get an override.
/// Do not call this directly.
pub fn __get_override<T: Clone + 'static>(overrides: &OverridesMap) -> Option<T> {
    overrides.get(&TypeId::of::<T>())
        .and_then(|arc| arc.downcast_ref::<T>().cloned())
}

/// Builder for creating a `TestContainer` with service overrides.
///
/// Use this to configure which services should be replaced with mock
/// instances before building the test container.
///
/// # Example
///
/// ```rust,ignore
/// let container = TestContainer::builder()
///     .with_override::<Database>(MockDatabase::new())
///     .with_override::<Logger>(MockLogger::new())
///     .build();
/// ```
pub struct TestContainerBuilder {
    overrides: OverridesMap,
}

impl Default for TestContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestContainerBuilder {
    /// Create a new builder with no overrides.
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }

    /// Replace a service of type `T` with the provided instance.
    ///
    /// The instance will be used instead of the normally constructed service
    /// when resolving from the test container.
    ///
    /// # Type Safety
    ///
    /// The type `T` must match a service registered in the `module!()` macro.
    /// If `T` is not a registered service, the override will be stored but
    /// `get_from::<T>()` will fail at runtime with a clear error message.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let container = TestContainer::builder()
    ///     .with_override::<Database>(MockDatabase::new())
    ///     .build();
    /// ```
    pub fn with_override<T: Clone + Send + Sync + 'static>(mut self, instance: T) -> Self {
        self.overrides.insert(TypeId::of::<T>(), Arc::new(instance));
        self
    }

    /// Build the test container with the configured overrides.
    ///
    /// This creates an isolated container that does not touch the global
    /// `GLOBAL_CONTAINER`. The returned `TestContainer` can be used with
    /// `get_override()` to resolve services.
    pub fn build(self) -> TestContainer {
        TestContainer {
            overrides: self.overrides,
            container: None,
            initialized: false,
        }
    }

    /// Build the test container and apply overrides to a real `OxidiContainer`.
    ///
    /// The `container_builder` function is `OxidiContainer::with_overrides` generated
    /// by the `module!()` macro. This creates a real container with overrides applied,
    /// so services that depend on other services will work correctly.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let container = TestContainer::builder()
    ///     .with_override::<Database>(MockDatabase::new())
    ///     .build_with_container(OxidiContainer::with_overrides);
    ///
    /// // Now get_from works with the real container
    /// let db: Database = get_from(&container);
    /// ```
    pub fn build_with_container<C>(self, container_builder: impl Fn(&OverridesMap) -> C) -> TestContainer
    where
        C: Any + Send + Sync,
    {
        let container = self.build();
        container.build_with_container(container_builder)
    }
}

/// An isolated DI container for tests.
///
/// This container holds service overrides and can be used to resolve
/// services without touching the global container. It supports:
///
/// - `get_from()` for explicit resolution from the container
/// - `init()` for setting up thread-local resolution (enables regular `get()`)
/// - Automatic cleanup via `Drop`
///
/// # Thread Safety
///
/// Each thread can have its own `TestContainer` active via `init()`.
/// Tests running in parallel will not interfere with each other.
///
/// # Building with a real container
///
/// To use `TestContainer` with the generated `OxidiContainer`, call
/// `build_with_container()` after setting up overrides:
///
/// ```rust,ignore
/// let container = TestContainer::builder()
///     .with_override::<Database>(MockDatabase::new())
///     .build_with_container(OxidiContainer::with_overrides);
/// ```
pub struct TestContainer {
    overrides: OverridesMap,
    /// The actual container with overrides applied, stored as Any for type erasure.
    /// This is set by `build_with_container()` and used by `get_from()`.
    container: Option<Box<dyn Any + Send + Sync>>,
    initialized: bool,
}

impl TestContainer {
    /// Create a new builder for configuring overrides.
    pub fn builder() -> TestContainerBuilder {
        TestContainerBuilder::new()
    }

    /// Build the test container and apply overrides to a real `OxidiContainer`.
    ///
    /// The `container_builder` function is `OxidiContainer::with_overrides` generated
    /// by the `module!()` macro. This creates a real container with overrides applied,
    /// so services that depend on other services will work correctly.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let container = TestContainer::builder()
    ///     .with_override::<Database>(MockDatabase::new())
    ///     .build_with_container(OxidiContainer::with_overrides);
    ///
    /// // Now get_from works with the real container
    /// let db: Database = get_from(&container);
    /// ```
    pub fn build_with_container<C>(self, container_builder: impl Fn(&OverridesMap) -> C) -> Self
    where
        C: Any + Send + Sync,
    {
        let container = container_builder(&self.overrides);
        Self {
            overrides: self.overrides.clone(),
            container: Some(Box::new(container)),
            initialized: false,
        }
    }

    /// Check if an override exists for type `T`.
    pub fn has_override<T: 'static>(&self) -> bool {
        self.overrides.contains_key(&TypeId::of::<T>())
    }

    /// Get an overridden instance of type `T`.
    ///
    /// # Panics
    ///
    /// Panics if no override is registered for type `T`.
    pub fn get_override_instance<T: Clone + 'static>(&self) -> T {
        self.overrides.get(&TypeId::of::<T>())
            .expect("No override registered for this type. Use `with_override::<T>()` first.")
            .downcast_ref::<T>()
            .expect("Type mismatch in override storage")
            .clone()
    }

    /// Get a reference to the overrides map for use with `OxidiContainer::with_overrides()`.
    pub fn overrides(&self) -> &OverridesMap {
        &self.overrides
    }

    /// Initialize the test container as the active container for the current thread.
    ///
    /// After calling this, regular `get()` calls will use this container's overrides
    /// instead of the global container. The container is automatically cleaned up
    /// when dropped.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let container = TestContainer::builder()
    ///     .with_override::<Database>(MockDatabase::new())
    ///     .build();
    ///
    /// container.init();
    ///
    /// // Now regular get() uses the test container
    /// let db: Database = oxidi_core::get();
    /// ```
    pub fn init(&mut self) {
        TEST_CONTAINER.with(|cell| {
            cell.borrow_mut().replace(Arc::new(self.overrides.clone()));
        });
        self.initialized = true;
    }

    /// Manually clean up the test container.
    ///
    /// This is called automatically when the container is dropped, but can be
    /// called explicitly to clean up early.
    pub fn cleanup(&mut self) {
        if self.initialized {
            TEST_CONTAINER.with(|cell| {
                *cell.borrow_mut() = None;
            });
            self.initialized = false;
        }
    }
}

impl Drop for TestContainer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

// Thread-local storage for the current test container.
// Allows `get()` to work with test containers without modifying
// the global container. Each thread has its own test container context.
task_local! {
    static TEST_CONTAINER: RefCell<Option<Arc<OverridesMap>>>;
}

/// Get an overridden service of type `T` from a test container.
///
/// This is the simplest way to resolve services from a `TestContainer`.
/// It checks the container's overrides map and returns the mock instance.
/// Use this when you only need to override specific services.
///
/// # Example
///
/// ```rust,ignore
/// let container = TestContainer::builder()
///     .with_override::<Database>(MockDatabase::new())
///     .build();
///
/// let db: Database = get_override(&container);
/// ```
///
/// # Panics
///
/// Panics if no override is registered for type `T`.
pub fn get_override<T: Clone + 'static>(container: &TestContainer) -> T {
    container.overrides.get(&TypeId::of::<T>())
        .unwrap_or_else(|| panic!(
            "No override found for type `{}`. Use `with_override::<{}>()` to provide a mock instance.",
            std::any::type_name::<T>(),
            std::any::type_name::<T>()
        ))
        .downcast_ref::<T>()
        .expect("Type mismatch in override storage")
        .clone()
}

/// Resolve a service of type `T` from a test container built with `build_with_container()`.
///
/// This uses the real `OxidiContainer` with overrides applied, so services that depend
/// on other services will work correctly. The container must have been built with
/// `build_with_container()`.
///
/// # Example
///
/// ```rust,ignore
/// let container = TestContainer::builder()
///     .with_override::<Database>(MockDatabase::new())
///     .build_with_container(OxidiContainer::with_overrides);
///
/// let db: Database = get_from(&container);
/// ```
///
/// # Panics
///
/// Panics if the container was not built with `build_with_container()` or if
/// type `T` is not registered in the module.
pub fn get_from<T: Clone + OxidiResolve + 'static>(container: &TestContainer) -> T {
    // If we have a real container, use OxidiResolve to get the service
    if let Some(ref real_container) = container.container {
        return <T as OxidiResolve>::resolve(real_container);
    }

    // No real container — this is an error
    panic!(
        "get_from() requires a container built with `build_with_container()`. \
         Use `get_override()` for containers built with `build()`, \
         or call `build_with_container(OxidiContainer::with_overrides)` instead of `build()`.",
    );
}

/// Enter a new scope and run the closure, using the test container's context.
///
/// This is a convenience wrapper around `crate::scope()` that works with `TestContainer`.
/// Inside the closure, `get()` automatically checks the scope first for scoped services,
/// then falls back to the test container's overrides.
///
/// # Example
///
/// ```rust,ignore
/// let container = TestContainer::builder()
///     .with_override::<Database>(MockDatabase::new())
///     .build();
///
/// container.init();
///
/// scope_with(|| {
///     let tx: Transaction = oxidi_core::get(); // scoped service
///     let db: Database = oxidi_core::get();    // overridden singleton
/// });
/// ```
pub fn scope_with<T>(f: impl FnOnce() -> T) -> T {
    crate::scope(f)
}

/// Enter a new scope and run a future, using the test container's context.
///
/// Like `scope_with()`, but designed for async code. The future can contain `.await`
/// points and the scope will be preserved across task migrations between threads.
///
/// # Example
///
/// ```rust,ignore
/// let container = TestContainer::builder()
///     .with_override::<Database>(MockDatabase::new())
///     .build();
///
/// container.init();
///
/// async_scope_with(async {
///     let tx: Transaction = oxidi_core::get();
///     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
/// }).await;
/// ```
pub async fn async_scope_with<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    crate::async_scope(future).await
}
