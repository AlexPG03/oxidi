//! Scoped services example for Oxidi.
//!
//! Demonstrates how to use `scoped` services with `scope()` closures.
//! Scoped services are created once per scope and dropped when the scope ends.
//!
//! Run with:
//! ```bash
//! cargo run -p oxidi_example_app --bin scoped_example
//! ```

use oxidi_core::{get, scope, async_scope};

// --- Example services ---

#[derive(Clone, Debug)]
pub struct Database {
    connection_string: String,
}

impl Database {
    pub fn new(conn: &str) -> Self {
        Self {
            connection_string: conn.to_string(),
        }
    }

    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }
}

#[derive(Clone, Debug)]
pub struct Transaction {
    #[allow(dead_code)]
    database: Database,
    id: u64,
}

// Global counter for generating unique transaction IDs
static mut TX_COUNTER: u64 = 0;

impl Transaction {
    pub fn new(database: Database) -> Self {
        unsafe {
            TX_COUNTER += 1;
            Self {
                database,
                id: TX_COUNTER,
            }
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn execute(&self, query: &str) -> String {
        format!("Transaction {}: executing '{}'", self.id, query)
    }
}

// --- DI module ---

oxidi_core::module! {
    single {
        Database::new("postgres://localhost:5432/mydb")
    }

    scoped {
        let db: Database = get();
        Transaction::new(db)
    }
}

// --- Main ---

fn main() {
    // Initialize the Oxidi container
    __oxidi_init__();

    println!("=== Oxidi Scoped Services Example ===\n");

    // Get a global singleton — works outside any scope
    let db: Database = get();
    println!("Global singleton: Database connected to {}", db.connection_string());
    println!();

    // First scope — creates a new Transaction
    println!("--- Scope 1 ---");
    scope(|| {
        let tx: Transaction = get();
        println!("Transaction ID: {}", tx.id());
        println!("{}", tx.execute("SELECT * FROM users"));

        // Second call in the same scope — returns the SAME Transaction
        let tx2: Transaction = get();
        println!("Second call in same scope — Transaction ID: {}", tx2.id());
        assert_eq!(tx.id(), tx2.id(), "Same scope should return the same Transaction");
    });
    // Transaction from scope 1 is dropped here
    println!("Scope 1 ended — Transaction dropped\n");

    // Second scope — creates a NEW Transaction
    println!("--- Scope 2 ---");
    scope(|| {
        let tx: Transaction = get();
        println!("Transaction ID: {}", tx.id());
        println!("{}", tx.execute("INSERT INTO users VALUES (...)"));
    });
    println!("Scope 2 ended — Transaction dropped\n");

    // Nested scopes
    println!("--- Nested Scopes ---");
    scope(|| {
        let outer_tx: Transaction = get();
        println!("Outer scope — Transaction ID: {}", outer_tx.id());

        scope(|| {
            let inner_tx: Transaction = get();
            println!("Inner scope — Transaction ID: {}", inner_tx.id());
            assert_ne!(
                outer_tx.id(),
                inner_tx.id(),
                "Inner scope should have a different Transaction"
            );
        });

        // After inner scope ends, outer scope still has its Transaction
        let outer_tx2: Transaction = get();
        println!("Back to outer scope — Transaction ID: {}", outer_tx2.id());
        assert_eq!(outer_tx.id(), outer_tx2.id(), "Outer scope should still have the same Transaction");
    });
    println!("Nested scopes ended\n");

    // Global singletons still work outside scopes
    let db2: Database = get();
    println!("Global singleton after scopes: Database connected to {}", db2.connection_string());
    println!();

    println!("All sync assertions passed! ✅\n");

    // --- Async scopes test ---
    println!("=== Async Scope Tests ===\n");

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        // Async scope — survives .await points
        println!("--- Async Scope ---");
        async_scope(async {
            let tx: Transaction = get();
            println!("Transaction ID before await: {}", tx.id());

            // Simulate async work
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // After .await, the scope is still active
            let tx2: Transaction = get();
            println!("Transaction ID after await: {}", tx2.id());
            assert_eq!(
                tx.id(),
                tx2.id(),
                "Async scope should preserve the Transaction across .await"
            );
        }).await;
        println!("Async scope ended\n");

        // Second async scope — new Transaction
        println!("--- Second Async Scope ---");
        async_scope(async {
            let tx: Transaction = get();
            println!("Transaction ID: {}", tx.id());
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let tx2: Transaction = get();
            println!("Same scope after await — Transaction ID: {}", tx2.id());
            assert_eq!(tx.id(), tx2.id());
        }).await;
        println!("Second async scope ended\n");

        // Nested async scopes
        println!("--- Nested Async Scopes ---");
        async_scope(async {
            let outer_tx: Transaction = get();
            println!("Outer async scope — Transaction ID: {}", outer_tx.id());

            async_scope(async {
                let inner_tx: Transaction = get();
                println!("Inner async scope — Transaction ID: {}", inner_tx.id());
                assert_ne!(
                    outer_tx.id(),
                    inner_tx.id(),
                    "Inner async scope should have a different Transaction"
                );
            }).await;

            let outer_tx2: Transaction = get();
            println!("Back to outer async scope — Transaction ID: {}", outer_tx2.id());
            assert_eq!(
                outer_tx.id(),
                outer_tx2.id(),
                "Outer async scope should still have the same Transaction"
            );
        }).await;
        println!("Nested async scopes ended\n");

        println!("All async assertions passed! ✅");
    });
}
