use oxidi_core::get;

// Example services

#[derive(Clone, Debug)]
pub struct Database {
    connection_string: String,
}

impl Database {
    pub fn new(conn: &str) -> Self {
        println!("Database::new({})", conn);
        Self {
            connection_string: conn.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserRepository {
    #[allow(dead_code)]
    database: Database,
}

impl UserRepository {
    pub fn new(database: Database) -> Self {
        println!("UserRepository::new(database)");
        Self { database }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Logger;

impl Logger {
    pub fn new() -> Self {
        println!("Logger::new()");
        Self
    }
}

#[derive(Clone, Debug)]
pub struct RequestHandler {
    #[allow(dead_code)]
    repo: UserRepository,
    #[allow(dead_code)]
    logger: Logger,
}

impl RequestHandler {
    pub fn new(repo: UserRepository, logger: Logger) -> Self {
        println!("RequestHandler::new(repo, logger)");
        Self { repo, logger }
    }
}

// Define the DI module at the module level (outside any function).
// This generates the container struct, OxidiResolve impls, and __oxidi_init__().
oxidi_core::module! {
    single {
        Logger::new()
    }
    
    single {
        Database::new("postgres://localhost:5432/mydb")
    }
    
    single {
        let db: Database = get();
        let _logger: Logger = get();
        UserRepository::new(db)
    }
    
    factory {
        let repo: UserRepository = get();
        let logger: Logger = get();
        RequestHandler::new(repo, logger)
    }
}

// Function that uses global get() without passing container
fn process_request() {
    let handler: RequestHandler = get();
    println!("Processing request with handler: {:?}", handler);
}

fn show_database_info() {
    let db: Database = get();
    println!("Database connection: {}", db.connection_string);
}

fn main() {
    println!("=== Initializing Oxidi container ===");
    
    // Initialize the global container
    __oxidi_init__();
    
    println!("\n=== Accessing services from main ===");
    
    // Access services from anywhere with get()
    let db: Database = get();
    println!("Got database: {:?}", db);
    let db: Database = get();
    println!("Got database: {:?}", db);
    
    let repo: UserRepository = get();
    println!("Got user repo: {:?}", repo);
    
    // Factory creates new instance each time
    let handler1: RequestHandler = get();
    let handler2: RequestHandler = get();
    println!("Got handler 1: {:?}", handler1);
    println!("Got handler 2: {:?}", handler2);
    
    println!("\n=== Calling functions that use get() ===");
    
    // Functions can access services without passing container
    process_request();
    show_database_info();
    
    println!("\n=== Done ===");
}
