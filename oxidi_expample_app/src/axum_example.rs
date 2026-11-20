//! Axum integration example for Oxidi.
//!
//! Demonstrates how to use `Inject<T>` to resolve Oxidi services
//! directly inside Axum handler signatures.
//!
//! Run with:
//! ```bash
//! cargo run -p oxidi_expample_app --bin axum_example
//! ```
//! Then visit http://127.0.0.1:3000/health or http://127.0.0.1:3000/db

use oxidi_axum::Inject;
use oxidi_core::get;
use axum::{Json, Router, routing::get as axum_get};
use serde::Serialize;

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
pub struct UserRepository {
    #[allow(dead_code)]
    database: Database,
}

impl UserRepository {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub fn user_count(&self) -> usize {
        // Simulated count
        42
    }
}

// --- DI module ---

oxidi_core::module! {
    single {
        Database::new("postgres://localhost:5432/mydb")
    }

    single {
        let db: Database = get();
        UserRepository::new(db)
    }
}

// --- Axum handlers ---

/// Health check endpoint — injects `UserRepository` via `Inject<T>`.
async fn health_check(Inject(repo): Inject<UserRepository>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        users: repo.user_count(),
    })
}

/// Database info endpoint — injects `Database` via `Inject<T>`.
async fn db_info(Inject(db): Inject<Database>) -> Json<DbInfoResponse> {
    Json(DbInfoResponse {
        connection: db.connection_string().to_string(),
    })
}

// --- Response types ---

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    users: usize,
}

#[derive(Serialize)]
struct DbInfoResponse {
    connection: String,
}

// --- Main ---

#[tokio::main]
async fn main() {
    // Initialize the Oxidi container before starting the server
    __oxidi_init__();

    println!("=== Oxidi Axum Example ===");
    println!("Container initialized. Starting server...");

    let app = Router::new()
        .route("/health", axum_get(health_check))
        .route("/db", axum_get(db_info));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to 127.0.0.1:3000");

    println!("Server listening on http://127.0.0.1:3000");
    println!("  GET /health — returns user count from UserRepository");
    println!("  GET /db     — returns database connection string");
    println!("Press Ctrl+C to stop.\n");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
