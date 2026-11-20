//! Examples of using Oxidi test support features.
//!
//! This file demonstrates how to use `TestContainer`, `with_override`, and `get_override`
//! for writing isolated unit tests with mock services.
//!
//! These tests use `TestContainer` directly without a `module!()` definition,
//! demonstrating the simplest use case: overriding services with mocks.

use oxidi_core::test_support::{TestContainer, get_override, scope_with};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

// --- Mock services ---

/// A simple mock database for testing
#[derive(Clone, Debug)]
pub struct MockDatabase {
    pub connection_string: String,
    pub call_count: Arc<AtomicU32>,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            connection_string: "mock://test-db".to_string(),
            call_count: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn query(&self) -> String {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        format!("Mock result from {}", self.connection_string)
    }

    pub fn is_mock(&self) -> bool {
        true
    }
}

/// A simple mock logger for testing
#[derive(Clone, Debug)]
pub struct MockLogger {
    pub messages: Arc<std::sync::Mutex<Vec<String>>>,
}

impl MockLogger {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn log(&self, message: &str) {
        self.messages.lock().unwrap().push(message.to_string());
    }

    pub fn get_messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }
}

// --- Test examples ---

/// Example 1: Basic override with manual mock
///
/// Shows how to replace a service with a mock instance using `get_override()`.
#[test]
fn test_basic_override() {
    // Create a test container with overrides
    let container = TestContainer::builder()
        .with_override::<MockDatabase>(MockDatabase::new())
        .with_override::<MockLogger>(MockLogger::new())
        .build();

    // Resolve services from the test container
    let db: MockDatabase = get_override(&container);
    assert!(db.is_mock());
    assert_eq!(db.connection_string, "mock://test-db");

    let logger: MockLogger = get_override(&container);
    logger.log("test message");
    assert_eq!(logger.get_messages().len(), 1);
}

/// Example 2: Multiple overrides
///
/// Shows how to override multiple services at once.
#[test]
fn test_multiple_overrides() {
    let mock_db = MockDatabase::new();
    let mock_logger = MockLogger::new();

    let container = TestContainer::builder()
        .with_override::<MockDatabase>(mock_db.clone())
        .with_override::<MockLogger>(mock_logger.clone())
        .build();

    // Both services should be the mocks
    let db: MockDatabase = get_override(&container);
    let logger: MockLogger = get_override(&container);

    assert!(db.is_mock());
    assert_eq!(logger.get_messages().len(), 0);
}

/// Example 3: Scopes with test container
///
/// Shows how to use scope_with() for scoped services in tests.
#[test]
fn test_scope_with_container() {
    let container = TestContainer::builder()
        .with_override::<MockDatabase>(MockDatabase::new())
        .build();

    // Use scope_with to create a scope context
    scope_with(|| {
        // Inside the scope, services can be resolved
        let db: MockDatabase = get_override(&container);
        assert!(db.is_mock());
    });
}

/// Example 4: Test container isolation
///
/// Demonstrates that each TestContainer is independent.
#[test]
fn test_container_isolation() {
    let db1 = MockDatabase::new();
    let db2 = MockDatabase::new();

    let container1 = TestContainer::builder()
        .with_override::<MockDatabase>(db1.clone())
        .build();

    let container2 = TestContainer::builder()
        .with_override::<MockDatabase>(db2.clone())
        .build();

    // Each container has its own override
    let resolved1: MockDatabase = get_override(&container1);
    let resolved2: MockDatabase = get_override(&container2);

    assert!(resolved1.is_mock());
    assert!(resolved2.is_mock());
}

/// Example 5: has_override check
#[test]
fn test_has_override() {
    let container = TestContainer::builder()
        .with_override::<MockDatabase>(MockDatabase::new())
        .build();

    assert!(container.has_override::<MockDatabase>());
    assert!(!container.has_override::<MockLogger>());
}

/// Example 6: Parallel tests simulation
///
/// Shows that TestContainer can be used in parallel without interference.
#[test]
fn test_parallel_simulation() {
    use std::thread;

    let handle1 = thread::spawn(|| {
        let container = TestContainer::builder()
            .with_override::<MockDatabase>(MockDatabase::new())
            .build();
        let db: MockDatabase = get_override(&container);
        assert!(db.is_mock());
    });

    let handle2 = thread::spawn(|| {
        let container = TestContainer::builder()
            .with_override::<MockDatabase>(MockDatabase::new())
            .build();
        let db: MockDatabase = get_override(&container);
        assert!(db.is_mock());
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}

/// Example 7: get_override_instance method
#[test]
fn test_get_override_instance() {
    let container = TestContainer::builder()
        .with_override::<MockDatabase>(MockDatabase::new())
        .build();

    let db: MockDatabase = container.get_override_instance();
    assert!(db.is_mock());
}
