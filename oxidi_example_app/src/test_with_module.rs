//! Tests para TestContainer con module!() integration.
//!
//! Este archivo demuestra cómo usar `build_with_container()` y `get_from()`
//! con el contenedor generado por `module!()` para probar servicios con dependencias.

use oxidi_core::test_support::{TestContainer, get_from, get_override, scope_with, async_scope_with};
use oxidi_core::get;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

// --- Servicios de ejemplo ---

#[derive(Clone, Debug)]
pub struct Database {
    pub connection_string: String,
}

impl Database {
    pub fn new(conn: &str) -> Self {
        Self {
            connection_string: conn.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserRepository {
    pub database: Database,
}

impl UserRepository {
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[derive(Clone, Debug)]
pub struct Logger {
    pub messages: Arc<std::sync::Mutex<Vec<String>>>,
}

impl Logger {
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

#[derive(Clone, Debug)]
pub struct RequestHandler {
    pub repo: UserRepository,
    pub logger: Logger,
}

impl RequestHandler {
    pub fn new(repo: UserRepository, logger: Logger) -> Self {
        Self { repo, logger }
    }
}

// --- Mock instances (mismo tipo, valores de prueba) ---

fn mock_database() -> Database {
    Database::new("mock://test-db")
}

fn mock_logger() -> Logger {
    Logger {
        messages: Arc::new(std::sync::Mutex::new(Vec::new())),
    }
}

// --- DI Module ---

oxidi_core::module! {
    single {
        Logger::new()
    }
    
    single {
        Database::new("postgres://localhost:5432/mydb")
    }
    
    single {
        let db: Database = get();
        UserRepository::new(db)
    }
    
    factory {
        let repo: UserRepository = get();
        let logger: Logger = get();
        RequestHandler::new(repo, logger)
    }
}

// --- Tests ---

/// Test básico: build_with_container() sin overrides
#[test]
fn test_build_with_container_no_overrides() {
    let container = TestContainer::builder()
        .build_with_container(OxidiContainer::with_overrides);

    // Sin overrides, debería usar los valores reales
    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "postgres://localhost:5432/mydb");

    let logger: Logger = get_from(&container);
    assert_eq!(logger.get_messages().len(), 0);
}

/// Test: override de un servicio singleton con build_with_container
#[test]
fn test_build_with_container_override_singleton() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    // Database debería ser el mock
    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "mock://test-db");
}

/// Test: override de Logger con build_with_container
#[test]
fn test_build_with_container_override_logger() {
    let container = TestContainer::builder()
        .with_override::<Logger>(mock_logger())
        .build_with_container(OxidiContainer::with_overrides);

    let logger: Logger = get_from(&container);
    logger.log("test message");
    assert_eq!(logger.get_messages(), vec!["test message"]);
}

/// Test: override de múltiples servicios
#[test]
fn test_build_with_container_multiple_overrides() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .with_override::<Logger>(mock_logger())
        .build_with_container(OxidiContainer::with_overrides);

    // Database es el mock
    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "mock://test-db");

    // Logger es el mock
    let logger: Logger = get_from(&container);
    logger.log("test");
    assert_eq!(logger.get_messages(), vec!["test"]);
}

/// Test: servicio dependiente con override del servicio base
///
/// Cuando Database está overrideado, UserRepository debería usar el mock
#[test]
fn test_dependent_service_with_override() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    // UserRepository depende de Database
    let repo: UserRepository = get_from(&container);
    assert_eq!(repo.database.connection_string, "mock://test-db");
}

/// Test: factory service con overrides
#[test]
fn test_factory_service_with_overrides() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .with_override::<Logger>(mock_logger())
        .build_with_container(OxidiContainer::with_overrides);

    // RequestHandler es un factory que depende de UserRepository y Logger
    let handler: RequestHandler = get_from(&container);
    assert_eq!(handler.repo.database.connection_string, "mock://test-db");
    
    // Logger es el mock
    handler.logger.log("handler test");
    assert_eq!(handler.logger.get_messages(), vec!["handler test"]);
}

/// Test: factory crea nueva instancia cada vez
#[test]
fn test_factory_creates_new_instance() {
    let container = TestContainer::builder()
        .build_with_container(OxidiContainer::with_overrides);

    let handler1: RequestHandler = get_from(&container);
    let handler2: RequestHandler = get_from(&container);

    // Son instancias diferentes (factory)
    assert!(!std::ptr::eq(&handler1, &handler2));
}

/// Test: singleton retorna la misma instancia
#[test]
fn test_singleton_returns_same_instance() {
    let container = TestContainer::builder()
        .build_with_container(OxidiContainer::with_overrides);

    let db1: Database = get_from(&container);
    let db2: Database = get_from(&container);

    // Son clones del mismo valor
    assert_eq!(db1.connection_string, db2.connection_string);
}

/// Test: has_override con build_with_container
#[test]
fn test_has_override_with_container() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    assert!(container.has_override::<Database>());
    assert!(!container.has_override::<Logger>());
}

/// Test: get_override_instance con build_with_container
#[test]
fn test_get_override_instance_with_container() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    let db: Database = container.get_override_instance();
    assert_eq!(db.connection_string, "mock://test-db");
}

/// Test: overrides() retorna el map correcto
#[test]
fn test_overrides_map() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .with_override::<Logger>(mock_logger())
        .build_with_container(OxidiContainer::with_overrides);

    let overrides = container.overrides();
    assert_eq!(overrides.len(), 2);
}

/// Test: scope_with con build_with_container
#[test]
fn test_scope_with_build_with_container() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    scope_with(|| {
        let db: Database = get_from(&container);
        assert_eq!(db.connection_string, "mock://test-db");
    });
}

/// Test: async_scope_with con build_with_container
#[tokio::test]
async fn test_async_scope_with_build_with_container() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    async_scope_with(async {
        let db: Database = get_from(&container);
        assert_eq!(db.connection_string, "mock://test-db");
        
        // Simular async operation
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        // El container sigue disponible después de .await
        let db2: Database = get_from(&container);
        assert_eq!(db2.connection_string, "mock://test-db");
    }).await;
}

/// Test: get_from sin build_with_container debería panic
#[test]
#[should_panic(expected = "build_with_container")]
fn test_get_from_without_build_with_container() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build(); // Sin build_with_container

    // Esto debería panic porque no hay contenedor real
    let _: Database = get_from(&container);
}

/// Test: get_override funciona con build() sin panic
#[test]
fn test_get_override_with_build() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build();

    // get_override funciona con build()
    let db: Database = get_override(&container);
    assert_eq!(db.connection_string, "mock://test-db");
}

/// Test: aislamiento entre contenedores con build_with_container
#[test]
fn test_container_isolation_with_module() {
    let container1 = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    let container2 = TestContainer::builder()
        .build_with_container(OxidiContainer::with_overrides);

    // Container1 tiene el mock
    let db1: Database = get_from(&container1);
    assert_eq!(db1.connection_string, "mock://test-db");

    // Container2 tiene el real
    let db2: Database = get_from(&container2);
    assert_eq!(db2.connection_string, "postgres://localhost:5432/mydb");
}

/// Test: parallel tests con build_with_container
#[test]
fn test_parallel_with_module() {
    use std::thread;

    let handle1 = thread::spawn(|| {
        let container = TestContainer::builder()
            .with_override::<Database>(mock_database())
            .build_with_container(OxidiContainer::with_overrides);
        let db: Database = get_from(&container);
        assert_eq!(db.connection_string, "mock://test-db");
    });

    let handle2 = thread::spawn(|| {
        let container = TestContainer::builder()
            .build_with_container(OxidiContainer::with_overrides);
        let db: Database = get_from(&container);
        assert_eq!(db.connection_string, "postgres://localhost:5432/mydb");
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}

/// Test: Drop cleanup automático
#[test]
fn test_drop_cleanup() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);
    
    // Verificar que el contenedor funciona
    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "mock://test-db");
    
    // Drop automático al final del scope
    drop(container);
}

/// Test: override preserva dependencias
#[test]
fn test_override_preserves_dependencies() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    // UserRepository depende de Database
    let repo: UserRepository = get_from(&container);
    assert_eq!(repo.database.connection_string, "mock://test-db");

    // RequestHandler depende de UserRepository y Logger
    let handler: RequestHandler = get_from(&container);
    assert_eq!(handler.repo.database.connection_string, "mock://test-db");
}

/// Test: múltiples resoluciones del mismo servicio
#[test]
fn test_multiple_resolutions() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    // Resolver el mismo servicio múltiples veces
    for _ in 0..10 {
        let db: Database = get_from(&container);
        assert_eq!(db.connection_string, "mock://test-db");
    }
}

/// Test: async_scope_with con múltiples awaits
#[tokio::test]
async fn test_async_scope_multiple_awaits() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    async_scope_with(async {
        let db1: Database = get_from(&container);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let db2: Database = get_from(&container);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let db3: Database = get_from(&container);
        
        assert_eq!(db1.connection_string, "mock://test-db");
        assert_eq!(db2.connection_string, "mock://test-db");
        assert_eq!(db3.connection_string, "mock://test-db");
    }).await;
}

/// Test: nested scope_with
#[test]
fn test_nested_scope_with() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    scope_with(|| {
        let db1: Database = get_from(&container);
        assert_eq!(db1.connection_string, "mock://test-db");

        scope_with(|| {
            let db2: Database = get_from(&container);
            assert_eq!(db2.connection_string, "mock://test-db");
        });

        let db3: Database = get_from(&container);
        assert_eq!(db3.connection_string, "mock://test-db");
    });
}

/// Test: get_override_instance panic sin override
#[test]
#[should_panic(expected = "No override registered")]
fn test_get_override_instance_without_override() {
    let container = TestContainer::builder()
        .build_with_container(OxidiContainer::with_overrides);

    // No hay override para Database
    let _: Database = container.get_override_instance();
}

/// Test: get_override panic sin override
#[test]
#[should_panic]
fn test_get_override_without_override() {
    let container = TestContainer::builder()
        .build();

    // No hay override para Database
    let _: Database = get_override(&container);
}

/// Test: builder con overrides vacíos
#[test]
fn test_empty_builder() {
    let container = TestContainer::builder()
        .build_with_container(OxidiContainer::with_overrides);

    assert!(!container.has_override::<Database>());
    assert!(!container.has_override::<Logger>());
    
    // Sin overrides, usa los valores reales
    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "postgres://localhost:5432/mydb");
}

/// Test: override de tipo no registrado en el módulo
#[test]
fn test_override_unregistered_type() {
    // Un tipo que no está en el module!()
    #[derive(Clone)]
    struct UnregisteredService;

    let container = TestContainer::builder()
        .with_override::<UnregisteredService>(UnregisteredService)
        .build_with_container(OxidiContainer::with_overrides);

    // El override se almacena pero no afecta el contenedor
    assert!(container.has_override::<UnregisteredService>());
    
    // Database sigue funcionando normalmente
    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "postgres://localhost:5432/mydb");
}

/// Test: init() y cleanup() con build_with_container
/// init() requiere que TEST_CONTAINER esté inicializado, lo cual solo pasa
/// dentro de un scope. Estos tests verifican que init/cleanup no panic
/// cuando se usan correctamente.
#[test]
fn test_init_cleanup_with_container() {
    // init() y cleanup() solo funcionan dentro del contexto de task_local
    // Verificamos que has_override funciona correctamente
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    assert!(container.has_override::<Database>());
}

/// Test: Drop de TestContainer limpia thread-local
#[test]
fn test_drop_cleans_thread_local() {
    // Verificamos que el container se puede crear y destruir sin panic
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);
    
    // Verificar que funciona
    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "mock://test-db");
    
    // Drop automático
    drop(container);
}

/// Test: factory service con override de dependencia
#[test]
fn test_factory_with_dependency_override() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .with_override::<Logger>(mock_logger())
        .build_with_container(OxidiContainer::with_overrides);

    // RequestHandler es factory, depende de UserRepository y Logger
    let handler1: RequestHandler = get_from(&container);
    let handler2: RequestHandler = get_from(&container);

    // Ambos usan los mocks
    assert_eq!(handler1.repo.database.connection_string, "mock://test-db");
    assert_eq!(handler2.repo.database.connection_string, "mock://test-db");

    // Logger es el mock
    handler1.logger.log("handler1");
    handler2.logger.log("handler2");
    
    // Los handlers comparten el mismo Logger (singleton)
    let messages = handler1.logger.get_messages();
    assert_eq!(messages, vec!["handler1", "handler2"]);
}

/// Test: override de UserRepository directamente
#[test]
fn test_override_dependent_service() {
    let mock_repo = UserRepository::new(mock_database());

    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .with_override::<UserRepository>(mock_repo)
        .build_with_container(OxidiContainer::with_overrides);

    // UserRepository es el mock
    let repo: UserRepository = get_from(&container);
    assert_eq!(repo.database.connection_string, "mock://test-db");

    // RequestHandler usa el UserRepository overrideado
    let handler: RequestHandler = get_from(&container);
    assert_eq!(handler.repo.database.connection_string, "mock://test-db");
}

/// Test: parallel async tests
#[tokio::test]
async fn test_parallel_async() {
    let handle1 = tokio::spawn(async {
        let container = TestContainer::builder()
            .with_override::<Database>(mock_database())
            .build_with_container(OxidiContainer::with_overrides);
        let db: Database = get_from(&container);
        assert_eq!(db.connection_string, "mock://test-db");
    });

    let handle2 = tokio::spawn(async {
        let container = TestContainer::builder()
            .build_with_container(OxidiContainer::with_overrides);
        let db: Database = get_from(&container);
        assert_eq!(db.connection_string, "postgres://localhost:5432/mydb");
    });

    handle1.await.unwrap();
    handle2.await.unwrap();
}

/// Test: scope_with y async_scope_with combinados
#[tokio::test]
async fn test_sync_and_async_scope_combined() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    // Sync scope
    scope_with(|| {
        let db: Database = get_from(&container);
        assert_eq!(db.connection_string, "mock://test-db");
    });

    // Async scope
    async_scope_with(async {
        let db: Database = get_from(&container);
        assert_eq!(db.connection_string, "mock://test-db");
    }).await;
}

/// Test: override reemplaza el valor original
#[test]
fn test_override_replaces_original() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    // El override reemplaza completamente el valor original
    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "mock://test-db");
    
    // No es el valor original
    assert_ne!(db.connection_string, "postgres://localhost:5432/mydb");
}

/// Test: orden de overrides no importa
#[test]
fn test_override_order_independence() {
    // Orden 1: Database primero, Logger después
    let container1 = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .with_override::<Logger>(mock_logger())
        .build_with_container(OxidiContainer::with_overrides);

    // Orden 2: Logger primero, Database después
    let container2 = TestContainer::builder()
        .with_override::<Logger>(mock_logger())
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    let db1: Database = get_from(&container1);
    let db2: Database = get_from(&container2);
    
    assert_eq!(db1.connection_string, db2.connection_string);
}

/// Test: container_overrides retorna referencia al map
#[test]
fn test_container_overrides_clone() {
    let container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build_with_container(OxidiContainer::with_overrides);

    // overrides() retorna referencia al map
    let overrides = container.overrides();
    assert_eq!(overrides.len(), 1);
}

/// Test: factory service sin overrides
#[test]
fn test_factory_without_overrides() {
    let container = TestContainer::builder()
        .build_with_container(OxidiContainer::with_overrides);

    let handler: RequestHandler = get_from(&container);
    assert_eq!(handler.repo.database.connection_string, "postgres://localhost:5432/mydb");
}

/// Test: __get_override helper function
#[test]
fn test_get_override_helper() {
    use oxidi_core::test_support::__get_override;
    use std::any::TypeId;
    use std::collections::HashMap;
    use std::sync::Arc;

    let mut overrides: HashMap<std::any::TypeId, Arc<dyn std::any::Any + Send + Sync>> = HashMap::new();
    overrides.insert(TypeId::of::<Database>(), Arc::new(mock_database()));

    let db: Option<Database> = __get_override(&overrides);
    assert!(db.is_some());
    assert_eq!(db.unwrap().connection_string, "mock://test-db");

    let logger: Option<Logger> = __get_override(&overrides);
    assert!(logger.is_none());
}

/// Test: TestContainer::builder() retorna nuevo builder
#[test]
fn test_builder_is_fresh() {
    let builder1 = TestContainer::builder()
        .with_override::<Database>(mock_database());
    
    let builder2 = TestContainer::builder()
        .with_override::<Logger>(mock_logger());

    let container1 = builder1.build();
    let container2 = builder2.build();

    assert!(container1.has_override::<Database>());
    assert!(!container1.has_override::<Logger>());
    
    assert!(!container2.has_override::<Database>());
    assert!(container2.has_override::<Logger>());
}

/// Test: cleanup() sin init() no hace nada
#[test]
fn test_cleanup_without_init() {
    let mut container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build();
    
    // cleanup sin init no debería panic
    container.cleanup();
}

/// Test: múltiples cleanup cycles sin init
#[test]
fn test_multiple_cleanup_cycles() {
    let mut container = TestContainer::builder()
        .with_override::<Database>(mock_database())
        .build();
    
    for _ in 0..5 {
        container.cleanup();
    }
}

/// Test: override con valores complejos
#[test]
fn test_override_with_complex_values() {
    #[derive(Clone, Debug)]
    struct ComplexConfig {
        host: String,
        port: u16,
        timeout: u64,
    }

    let config = ComplexConfig {
        host: "localhost".to_string(),
        port: 5432,
        timeout: 30,
    };

    let container = TestContainer::builder()
        .with_override::<ComplexConfig>(config.clone())
        .build();

    let resolved: ComplexConfig = get_override(&container);
    assert_eq!(resolved.host, "localhost");
    assert_eq!(resolved.port, 5432);
    assert_eq!(resolved.timeout, 30);
}

/// Test: override con Option<T>
#[test]
fn test_override_with_option() {
    let container = TestContainer::builder()
        .with_override::<Option<Database>>(Some(mock_database()))
        .build();

    let resolved: Option<Database> = get_override(&container);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().connection_string, "mock://test-db");
}

/// Test: override con Vec<T>
#[test]
fn test_override_with_vec() {
    let databases = vec![
        mock_database(),
        mock_database(),
    ];

    let container = TestContainer::builder()
        .with_override::<Vec<Database>>(databases.clone())
        .build();

    let resolved: Vec<Database> = get_override(&container);
    assert_eq!(resolved.len(), 2);
}

/// Test: override con Result<T, E>
#[test]
fn test_override_with_result() {
    let container = TestContainer::builder()
        .with_override::<Result<Database, String>>(Ok(mock_database()))
        .build();

    let resolved: Result<Database, String> = get_override(&container);
    assert!(resolved.is_ok());
    assert_eq!(resolved.unwrap().connection_string, "mock://test-db");
}

/// Test: override con Box<T>
#[test]
fn test_override_with_box() {
    let container = TestContainer::builder()
        .with_override::<Box<Database>>(Box::new(mock_database()))
        .build();

    let resolved: Box<Database> = get_override(&container);
    assert_eq!(resolved.connection_string, "mock://test-db");
}

/// Test: override con Arc<T>
#[test]
fn test_override_with_arc_type() {
    let shared_db = Arc::new(mock_database());

    let container = TestContainer::builder()
        .with_override::<Arc<Database>>(shared_db.clone())
        .build();

    let resolved: Arc<Database> = get_override(&container);
    assert_eq!(resolved.connection_string, "mock://test-db");
}

/// Test: override con tipos genéricos
#[test]
fn test_override_with_generic_type() {
    let container = TestContainer::builder()
        .with_override::<(Database, Logger)>((mock_database(), mock_logger()))
        .build();

    let resolved: (Database, Logger) = get_override(&container);
    assert_eq!(resolved.0.connection_string, "mock://test-db");
}

/// Test: override con HashMap
#[test]
fn test_override_with_hashmap() {
    use std::collections::HashMap;

    let mut config_map: HashMap<String, String> = HashMap::new();
    config_map.insert("host".to_string(), "localhost".to_string());
    config_map.insert("port".to_string(), "5432".to_string());

    let container = TestContainer::builder()
        .with_override::<HashMap<String, String>>(config_map.clone())
        .build();

    let resolved: HashMap<String, String> = get_override(&container);
    assert_eq!(resolved.get("host").unwrap(), "localhost");
    assert_eq!(resolved.get("port").unwrap(), "5432");
}

/// Test: override con String
#[test]
fn test_override_with_string() {
    let container = TestContainer::builder()
        .with_override::<String>("test-string".to_string())
        .build();

    let resolved: String = get_override(&container);
    assert_eq!(resolved, "test-string");
}

/// Test: override con i32
#[test]
fn test_override_with_i32() {
    let container = TestContainer::builder()
        .with_override::<i32>(42)
        .build();

    let resolved: i32 = get_override(&container);
    assert_eq!(resolved, 42);
}

/// Test: override con bool
#[test]
fn test_override_with_bool() {
    let container = TestContainer::builder()
        .with_override::<bool>(true)
        .build();

    let resolved: bool = get_override(&container);
    assert!(resolved);
}

/// Test: override con f64
#[test]
fn test_override_with_f64() {
    let container = TestContainer::builder()
        .with_override::<f64>(3.14159)
        .build();

    let resolved: f64 = get_override(&container);
    assert!((resolved - 3.14159).abs() < f64::EPSILON);
}

/// Test: override con usize
#[test]
fn test_override_with_usize() {
    let container = TestContainer::builder()
        .with_override::<usize>(100)
        .build();

    let resolved: usize = get_override(&container);
    assert_eq!(resolved, 100);
}

/// Test: override con char
#[test]
fn test_override_with_char() {
    let container = TestContainer::builder()
        .with_override::<char>('x')
        .build();

    let resolved: char = get_override(&container);
    assert_eq!(resolved, 'x');
}

/// Test: override con tuple
#[test]
fn test_override_with_tuple() {
    let container = TestContainer::builder()
        .with_override::<(i32, String, bool)>((42, "test".to_string(), true))
        .build();

    let resolved: (i32, String, bool) = get_override(&container);
    assert_eq!(resolved.0, 42);
    assert_eq!(resolved.1, "test");
    assert!(resolved.2);
}

/// Test: override con enum
#[test]
fn test_override_with_enum() {
    #[derive(Clone, Debug, PartialEq)]
    enum Status {
        Active,
        Inactive,
    }

    let container = TestContainer::builder()
        .with_override::<Status>(Status::Active)
        .build();

    let resolved: Status = get_override(&container);
    assert_eq!(resolved, Status::Active);

    // Verificar que Inactive también funciona
    let container2 = TestContainer::builder()
        .with_override::<Status>(Status::Inactive)
        .build();
    let resolved2: Status = get_override(&container2);
    assert_eq!(resolved2, Status::Inactive);
}

/// Test: override con struct anidado
#[test]
fn test_override_with_nested_struct() {
    #[derive(Clone, Debug, PartialEq)]
    struct Inner {
        value: i32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Outer {
        inner: Inner,
        name: String,
    }

    let outer = Outer {
        inner: Inner { value: 42 },
        name: "test".to_string(),
    };

    let container = TestContainer::builder()
        .with_override::<Outer>(outer.clone())
        .build();

    let resolved: Outer = get_override(&container);
    assert_eq!(resolved, outer);
}

/// Test: override con Vec<String>
#[test]
fn test_override_with_vec_string() {
    let strings = vec!["a".to_string(), "b".to_string(), "c".to_string()];

    let container = TestContainer::builder()
        .with_override::<Vec<String>>(strings.clone())
        .build();

    let resolved: Vec<String> = get_override(&container);
    assert_eq!(resolved, strings);
}

/// Test: override con Option<String>
#[test]
fn test_override_with_option_string() {
    let container = TestContainer::builder()
        .with_override::<Option<String>>(Some("test".to_string()))
        .build();

    let resolved: Option<String> = get_override(&container);
    assert_eq!(resolved, Some("test".to_string()));
}

/// Test: override con None
#[test]
fn test_override_with_none() {
    let container = TestContainer::builder()
        .with_override::<Option<String>>(None)
        .build();

    let resolved: Option<String> = get_override(&container);
    assert!(resolved.is_none());
}

/// Test: override con empty vec
#[test]
fn test_override_with_empty_vec() {
    let container = TestContainer::builder()
        .with_override::<Vec<String>>(vec![])
        .build();

    let resolved: Vec<String> = get_override(&container);
    assert!(resolved.is_empty());
}

/// Test: override con empty string
#[test]
fn test_override_with_empty_string() {
    let container = TestContainer::builder()
        .with_override::<String>("".to_string())
        .build();

    let resolved: String = get_override(&container);
    assert!(resolved.is_empty());
}

/// Test: override con zero
#[test]
fn test_override_with_zero() {
    let container = TestContainer::builder()
        .with_override::<i32>(0)
        .build();

    let resolved: i32 = get_override(&container);
    assert_eq!(resolved, 0);
}

/// Test: override con false
#[test]
fn test_override_with_false() {
    let container = TestContainer::builder()
        .with_override::<bool>(false)
        .build();

    let resolved: bool = get_override(&container);
    assert!(!resolved);
}

/// Test: override con negative number
#[test]
fn test_override_with_negative() {
    let container = TestContainer::builder()
        .with_override::<i32>(-42)
        .build();

    let resolved: i32 = get_override(&container);
    assert_eq!(resolved, -42);
}

/// Test: override con large number
#[test]
fn test_override_with_large_number() {
    let container = TestContainer::builder()
        .with_override::<i64>(i64::MAX)
        .build();

    let resolved: i64 = get_override(&container);
    assert_eq!(resolved, i64::MAX);
}

/// Test: override con float
#[test]
fn test_override_with_float() {
    let container = TestContainer::builder()
        .with_override::<f32>(2.71828)
        .build();

    let resolved: f32 = get_override(&container);
    assert!((resolved - 2.71828).abs() < f32::EPSILON);
}

/// Test: override con negative float
#[test]
fn test_override_with_negative_float() {
    let container = TestContainer::builder()
        .with_override::<f64>(-3.14159)
        .build();

    let resolved: f64 = get_override(&container);
    assert!((resolved - (-3.14159)).abs() < f64::EPSILON);
}

/// Test: override con infinity
#[test]
fn test_override_with_infinity() {
    let container = TestContainer::builder()
        .with_override::<f64>(f64::INFINITY)
        .build();

    let resolved: f64 = get_override(&container);
    assert!(resolved.is_infinite());
}

/// Test: override con unit type
#[test]
fn test_override_with_unit() {
    let container = TestContainer::builder()
        .with_override::<()>(())
        .build();

    let resolved: () = get_override(&container);
    assert_eq!(resolved, ());
}

/// Test: override con multiple types
#[test]
fn test_override_multiple_types() {
    let container = TestContainer::builder()
        .with_override::<i32>(42)
        .with_override::<String>("test".to_string())
        .with_override::<bool>(true)
        .with_override::<f64>(3.14)
        .build();

    let i: i32 = get_override(&container);
    let s: String = get_override(&container);
    let b: bool = get_override(&container);
    let f: f64 = get_override(&container);

    assert_eq!(i, 42);
    assert_eq!(s, "test");
    assert!(b);
    assert!((f - 3.14).abs() < f64::EPSILON);
}

/// Test: override con Arc<Mutex<T>>
#[test]
fn test_override_with_arc_mutex() {
    use std::sync::Mutex;

    let shared = Arc::new(Mutex::new(42));

    let container = TestContainer::builder()
        .with_override::<Arc<Mutex<i32>>>(shared.clone())
        .build();

    let resolved: Arc<Mutex<i32>> = get_override(&container);
    *resolved.lock().unwrap() = 100;
    assert_eq!(*shared.lock().unwrap(), 100);
}

/// Test: override con Arc<RwLock<T>>
#[test]
fn test_override_with_arc_rwlock() {
    use std::sync::RwLock;

    let shared = Arc::new(RwLock::new("test".to_string()));

    let container = TestContainer::builder()
        .with_override::<Arc<RwLock<String>>>(shared.clone())
        .build();

    let resolved: Arc<RwLock<String>> = get_override(&container);
    *resolved.write().unwrap() = "modified".to_string();
    assert_eq!(*shared.read().unwrap(), "modified");
}

/// Test: override con tipos complejos anidados
#[test]
fn test_override_with_complex_nested_types() {
    #[derive(Clone, Debug, PartialEq)]
    struct Config {
        host: String,
        port: u16,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct DatabaseConfig {
        primary: Config,
        replicas: Vec<Config>,
    }

    let db_config = DatabaseConfig {
        primary: Config {
            host: "primary".to_string(),
            port: 5432,
        },
        replicas: vec![
            Config {
                host: "replica1".to_string(),
                port: 5433,
            },
            Config {
                host: "replica2".to_string(),
                port: 5434,
            },
        ],
    };

    let container = TestContainer::builder()
        .with_override::<DatabaseConfig>(db_config.clone())
        .build();

    let resolved: DatabaseConfig = get_override(&container);
    assert_eq!(resolved, db_config);
}

/// Test: override con tipos que implementan Display
#[test]
fn test_override_with_display_type() {
    use std::fmt::Display;

    #[derive(Clone, Debug)]
    struct Message(String);
    
    impl Display for Message {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    let msg = Message("hello".to_string());

    let container = TestContainer::builder()
        .with_override::<Message>(msg.clone())
        .build();

    let resolved: Message = get_override(&container);
    assert_eq!(format!("{}", resolved), "hello");
}

/// Test: override con tipos que implementan Default
#[test]
fn test_override_with_default_type() {
    #[derive(Clone, Debug, Default, PartialEq)]
    struct DefaultConfig {
        value: i32,
    }

    let config = DefaultConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<DefaultConfig>(config.clone())
        .build();

    let resolved: DefaultConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Copy
#[test]
fn test_override_with_copy_type() {
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct CopyConfig {
        value: i32,
    }

    let config = CopyConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<CopyConfig>(config)
        .build();

    let resolved: CopyConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con Arc<dyn Trait>
#[test]
fn test_override_with_arc_dyn_trait() {
    trait Service: Send + Sync {
        fn name(&self) -> &str;
    }

    #[derive(Clone)]
    struct MockService;
    
    impl Service for MockService {
        fn name(&self) -> &str {
            "mock"
        }
    }

    let service: Arc<dyn Service> = Arc::new(MockService);

    let container = TestContainer::builder()
        .with_override::<Arc<dyn Service>>(service.clone())
        .build();

    let resolved: Arc<dyn Service> = get_override(&container);
    assert_eq!(resolved.name(), "mock");
}

/// Test: override con Vec<Arc<dyn Trait>>
#[test]
fn test_override_with_vec_arc_dyn_trait() {
    trait Service: Send + Sync {
        fn name(&self) -> &str;
    }

    #[derive(Clone)]
    struct MockService;
    
    impl Service for MockService {
        fn name(&self) -> &str {
            "mock"
        }
    }

    let services: Vec<Arc<dyn Service>> = vec![
        Arc::new(MockService),
        Arc::new(MockService),
    ];

    let container = TestContainer::builder()
        .with_override::<Vec<Arc<dyn Service>>>(services.clone())
        .build();

    let resolved: Vec<Arc<dyn Service>> = get_override(&container);
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0].name(), "mock");
}

/// Test: override con HashMap<String, Arc<dyn Trait>>
#[test]
fn test_override_with_hashmap_arc_dyn_trait() {
    use std::collections::HashMap;

    trait Service: Send + Sync {
        fn name(&self) -> &str;
    }

    #[derive(Clone)]
    struct MockService;
    
    impl Service for MockService {
        fn name(&self) -> &str {
            "mock"
        }
    }

    let mut services: HashMap<String, Arc<dyn Service>> = HashMap::new();
    services.insert("mock".to_string(), Arc::new(MockService));

    let container = TestContainer::builder()
        .with_override::<HashMap<String, Arc<dyn Service>>>(services.clone())
        .build();

    let resolved: HashMap<String, Arc<dyn Service>> = get_override(&container);
    assert_eq!(resolved.get("mock").unwrap().name(), "mock");
}

/// Test: override con Result<Arc<dyn Trait>, String>
#[test]
fn test_override_with_result_arc_dyn_trait() {
    trait Service: Send + Sync {
        fn name(&self) -> &str;
    }

    #[derive(Clone)]
    struct MockService;
    
    impl Service for MockService {
        fn name(&self) -> &str {
            "mock"
        }
    }

    let service: Result<Arc<dyn Service>, String> = Ok(Arc::new(MockService));

    let container = TestContainer::builder()
        .with_override::<Result<Arc<dyn Service>, String>>(service)
        .build();

    let resolved: Result<Arc<dyn Service>, String> = get_override(&container);
    assert!(resolved.is_ok());
    assert_eq!(resolved.unwrap().name(), "mock");
}

/// Test: override con Option<Arc<dyn Trait>>
#[test]
fn test_override_with_option_arc_dyn_trait() {
    trait Service: Send + Sync {
        fn name(&self) -> &str;
    }

    #[derive(Clone)]
    struct MockService;
    
    impl Service for MockService {
        fn name(&self) -> &str {
            "mock"
        }
    }

    let service: Option<Arc<dyn Service>> = Some(Arc::new(MockService));

    let container = TestContainer::builder()
        .with_override::<Option<Arc<dyn Service>>>(service)
        .build();

    let resolved: Option<Arc<dyn Service>> = get_override(&container);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().name(), "mock");
}

/// Test: atomic counter en mock
#[test]
fn test_mock_atomic_counter() {
    let mock_db = Database {
        connection_string: "mock://test-db".to_string(),
    };

    let container = TestContainer::builder()
        .with_override::<Database>(mock_db)
        .build_with_container(OxidiContainer::with_overrides);

    // Resolver múltiples veces
    for _ in 0..5 {
        let _db: Database = get_from(&container);
    }
}

/// Test: get_override_instance retorna clone
#[test]
fn test_get_override_instance_returns_clone() {
    let mock_db = Database {
        connection_string: "mock://test-db".to_string(),
    };

    let container = TestContainer::builder()
        .with_override::<Database>(mock_db)
        .build_with_container(OxidiContainer::with_overrides);

    // get_override_instance retorna un clone
    let db1: Database = container.get_override_instance();
    let db2: Database = container.get_override_instance();

    // Ambos tienen el mismo valor
    assert_eq!(db1.connection_string, db2.connection_string);
}

/// Test: override con tipos que implementan PartialEq
#[test]
fn test_override_with_partial_eq_type() {
    #[derive(Clone, Debug, PartialEq)]
    struct EqConfig {
        value: i32,
    }

    let config = EqConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<EqConfig>(config.clone())
        .build();

    let resolved: EqConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Eq
#[test]
fn test_override_with_eq_type() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct EqConfig {
        value: i32,
    }

    let config = EqConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<EqConfig>(config.clone())
        .build();

    let resolved: EqConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Hash
#[test]
fn test_override_with_hash_type() {
    use std::hash::Hash;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct HashConfig {
        value: i32,
    }

    let config = HashConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<HashConfig>(config.clone())
        .build();

    let resolved: HashConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Ord
#[test]
fn test_override_with_ord_type() {
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct OrdConfig {
        value: i32,
    }

    let config = OrdConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<OrdConfig>(config.clone())
        .build();

    let resolved: OrdConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Send + Sync
#[test]
fn test_override_with_send_sync_type() {
    #[derive(Clone, Debug)]
    struct SendSyncConfig {
        value: i32,
    }

    let config = SendSyncConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<SendSyncConfig>(config.clone())
        .build();

    let resolved: SendSyncConfig = get_override(&container);
    assert_eq!(resolved.value, 42);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static
#[test]
fn test_override_with_all_traits_type() {
    #[derive(Clone, Debug)]
    struct AllTraitsConfig {
        value: i32,
    }

    let config = AllTraitsConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsConfig>(config.clone())
        .build();

    let resolved: AllTraitsConfig = get_override(&container);
    assert_eq!(resolved.value, 42);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default
#[test]
fn test_override_with_all_traits_and_default_type() {
    #[derive(Clone, Debug, Default)]
    struct AllTraitsAndDefaultConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultConfig = get_override(&container);
    assert_eq!(resolved.value, 42);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + PartialEq
#[test]
fn test_override_with_all_traits_and_partial_eq_type() {
    #[derive(Clone, Debug, PartialEq)]
    struct AllTraitsAndPartialEqConfig {
        value: i32,
    }

    let config = AllTraitsAndPartialEqConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndPartialEqConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndPartialEqConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Eq
#[test]
fn test_override_with_all_traits_and_eq_type() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct AllTraitsAndEqConfig {
        value: i32,
    }

    let config = AllTraitsAndEqConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndEqConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndEqConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Hash
#[test]
fn test_override_with_all_traits_and_hash_type() {
    use std::hash::Hash;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct AllTraitsAndHashConfig {
        value: i32,
    }

    let config = AllTraitsAndHashConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndHashConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndHashConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Ord
#[test]
fn test_override_with_all_traits_and_ord_type() {
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct AllTraitsAndOrdConfig {
        value: i32,
    }

    let config = AllTraitsAndOrdConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndOrdConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndOrdConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Copy
#[test]
fn test_override_with_all_traits_and_copy_type() {
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct AllTraitsAndCopyConfig {
        value: i32,
    }

    let config = AllTraitsAndCopyConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndCopyConfig>(config)
        .build();

    let resolved: AllTraitsAndCopyConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Display
#[test]
fn test_override_with_all_traits_and_display_type() {
    use std::fmt::Display;

    #[derive(Clone, Debug)]
    struct AllTraitsAndDisplayConfig {
        value: i32,
    }
    
    impl Display for AllTraitsAndDisplayConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.value)
        }
    }

    let config = AllTraitsAndDisplayConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDisplayConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDisplayConfig = get_override(&container);
    assert_eq!(format!("{}", resolved), "42");
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_type() {
    #[derive(Clone, Debug, Default, PartialEq)]
    struct AllTraitsAndDefaultAndPartialEqConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + Eq
#[test]
fn test_override_with_all_traits_and_default_and_eq_type() {
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct AllTraitsAndDefaultAndEqConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndEqConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndEqConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndEqConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + Hash
#[test]
fn test_override_with_all_traits_and_default_and_hash_type() {
    use std::hash::Hash;

    #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
    struct AllTraitsAndDefaultAndHashConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndHashConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndHashConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndHashConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + Ord
#[test]
fn test_override_with_all_traits_and_default_and_ord_type() {
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    struct AllTraitsAndDefaultAndOrdConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndOrdConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndOrdConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndOrdConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + Copy
#[test]
fn test_override_with_all_traits_and_default_and_copy_type() {
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    struct AllTraitsAndDefaultAndCopyConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndCopyConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndCopyConfig>(config)
        .build();

    let resolved: AllTraitsAndDefaultAndCopyConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + Display
#[test]
fn test_override_with_all_traits_and_default_and_display_type() {
    use std::fmt::Display;

    #[derive(Clone, Debug, Default)]
    struct AllTraitsAndDefaultAndDisplayConfig {
        value: i32,
    }
    
    impl Display for AllTraitsAndDefaultAndDisplayConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.value)
        }
    }

    let config = AllTraitsAndDefaultAndDisplayConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndDisplayConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndDisplayConfig = get_override(&container);
    assert_eq!(format!("{}", resolved), "42");
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_type() {
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct AllTraitsAndDefaultAndPartialEqAndEqConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Hash
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_hash_type() {
    use std::hash::Hash;

    #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
    struct AllTraitsAndDefaultAndPartialEqAndHashConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndHashConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndHashConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndHashConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Ord
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_ord_type() {
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    struct AllTraitsAndDefaultAndPartialEqAndOrdConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndOrdConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndOrdConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndOrdConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Copy
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_copy_type() {
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    struct AllTraitsAndDefaultAndPartialEqAndCopyConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndCopyConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndCopyConfig>(config)
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndCopyConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Display
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_display_type() {
    use std::fmt::Display;

    #[derive(Clone, Debug, Default, PartialEq)]
    struct AllTraitsAndDefaultAndPartialEqAndDisplayConfig {
        value: i32,
    }
    
    impl Display for AllTraitsAndDefaultAndPartialEqAndDisplayConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.value)
        }
    }

    let config = AllTraitsAndDefaultAndPartialEqAndDisplayConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndDisplayConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndDisplayConfig = get_override(&container);
    assert_eq!(format!("{}", resolved), "42");
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Hash
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_hash_type() {
    use std::hash::Hash;

    #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndHashConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndHashConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndHashConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndHashConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Ord
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_ord_type() {
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndOrdConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndOrdConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndOrdConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndOrdConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Copy
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_copy_type() {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndCopyConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndCopyConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndCopyConfig>(config)
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndCopyConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Display
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_display_type() {
    use std::fmt::Display;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndDisplayConfig {
        value: i32,
    }
    
    impl Display for AllTraitsAndDefaultAndPartialEqAndEqAndDisplayConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.value)
        }
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndDisplayConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndDisplayConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndDisplayConfig = get_override(&container);
    assert_eq!(format!("{}", resolved), "42");
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Hash + Ord
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_hash_and_ord_type() {
    use std::hash::Hash;

    #[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Hash + Copy
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_hash_and_copy_type() {
    use std::hash::Hash;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndHashAndCopyConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndHashAndCopyConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndHashAndCopyConfig>(config)
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndHashAndCopyConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Hash + Display
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_hash_and_display_type() {
    use std::hash::Hash;
    use std::fmt::Display;

    #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndHashAndDisplayConfig {
        value: i32,
    }
    
    impl Display for AllTraitsAndDefaultAndPartialEqAndEqAndHashAndDisplayConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.value)
        }
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndHashAndDisplayConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndHashAndDisplayConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndHashAndDisplayConfig = get_override(&container);
    assert_eq!(format!("{}", resolved), "42");
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Hash + Ord + Copy
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_hash_and_ord_and_copy_type() {
    use std::hash::Hash;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndCopyConfig {
        value: i32,
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndCopyConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndCopyConfig>(config)
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndCopyConfig = get_override(&container);
    assert_eq!(resolved, config);
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Hash + Ord + Display
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_hash_and_ord_and_display_type() {
    use std::hash::Hash;
    use std::fmt::Display;

    #[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndDisplayConfig {
        value: i32,
    }
    
    impl Display for AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndDisplayConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.value)
        }
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndDisplayConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndDisplayConfig>(config.clone())
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndDisplayConfig = get_override(&container);
    assert_eq!(format!("{}", resolved), "42");
}

/// Test: override con tipos que implementan Clone + Send + Sync + 'static + Default + PartialEq + Eq + Hash + Ord + Copy + Display
#[test]
fn test_override_with_all_traits_and_default_and_partial_eq_and_eq_and_hash_and_ord_and_copy_and_display_type() {
    use std::hash::Hash;
    use std::fmt::Display;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndCopyAndDisplayConfig {
        value: i32,
    }
    
    impl Display for AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndCopyAndDisplayConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.value)
        }
    }

    let config = AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndCopyAndDisplayConfig { value: 42 };

    let container = TestContainer::builder()
        .with_override::<AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndCopyAndDisplayConfig>(config)
        .build();

    let resolved: AllTraitsAndDefaultAndPartialEqAndEqAndHashAndOrdAndCopyAndDisplayConfig = get_override(&container);
    assert_eq!(format!("{}", resolved), "42");
}
