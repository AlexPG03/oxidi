//! Ejemplos reales de testing con Oxidi test_support.
//!
//! Este archivo demuestra cómo usar cada feature del módulo `test_support`
//! en escenarios realistas de una aplicación e-commerce.
//!
//! # Features cubiertas
//!
//! 1. **Override básico**: Reemplazar un servicio con un mock
//! 2. **Servicios dependientes**: Override que afecta a servicios que dependen de él
//! 3. **Factory services**: Testing de servicios factory con overrides
//! 4. **Múltiples overrides**: Reemplazar varios servicios simultáneamente
//! 5. **Aislamiento**: Cada test usa su propio contenedor independiente
//! 6. **Tests paralelos**: Ejecutar tests en paralelo sin interferencia
//! 7. **has_override**: Verificar si un override existe
//! 8. **get_override_instance**: Acceder directamente al override
//! 9. **build() vs build_with_container()**: Diferencias entre ambos métodos
//! 10. **Edge cases**: Comportamiento en casos límite
//! 11. **Override con Arc**: Compartir estado entre mock y test
//! 12. **Flujo completo**: Test de integración con todos los mocks

use oxidi_core::test_support::{
    TestContainer, get_from, get_override
};
use oxidi_core::get;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

// ============================================================================
// Definición de servicios de ejemplo: aplicación e-commerce
// ============================================================================

/// Servicio de base de datos
#[derive(Clone, Debug)]
pub struct Database {
    pub connection_string: String,
}

impl Database {
    pub fn production() -> Self {
        Self {
            connection_string: "postgres://user:pass@prod-db:5432/ecommerce".to_string(),
        }
    }

    pub fn is_production(&self) -> bool {
        self.connection_string.contains("prod")
    }
}

/// Servicio de caché Redis
#[derive(Clone, Debug)]
pub struct Cache {
    pub enabled: bool,
    pub call_count: Arc<AtomicU32>,
}

impl Cache {
    pub fn production() -> Self {
        Self {
            enabled: true,
            call_count: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn mock() -> Self {
        Self {
            enabled: false,
            call_count: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        if self.enabled {
            Some(format!("cached:{}", key))
        } else {
            None
        }
    }
}

/// Repositorio de productos
#[derive(Clone, Debug)]
pub struct ProductRepository {
    pub database: Database,
    pub cache: Cache,
}

impl ProductRepository {
    pub fn new(database: Database, cache: Cache) -> Self {
        Self { database, cache }
    }

    pub fn find_by_id(&self, id: u32) -> Option<String> {
        // Primero intenta caché
        if let Some(cached) = self.cache.get(&format!("product:{}", id)) {
            return Some(cached);
        }
        // Luego consulta la DB
        Some(format!("Product {} from {}", id, self.database.connection_string))
    }
}

/// Servicio de envío (shipping)
#[derive(Clone, Debug)]
pub struct ShippingService {
    pub api_key: String,
    pub call_count: Arc<AtomicU32>,
}

impl ShippingService {
    pub fn production() -> Self {
        Self {
            api_key: "sk-prod-12345".to_string(),
            call_count: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn mock() -> Self {
        Self {
            api_key: "sk-test-mock".to_string(),
            call_count: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn calculate_rate(&self, weight: f64) -> f64 {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        if self.api_key.contains("mock") {
            10.0 // Tarifa fija para tests
        } else {
            weight * 2.5 // Cálculo real
        }
    }

    pub fn is_mock(&self) -> bool {
        self.api_key.contains("mock")
    }
}

/// Servicio de notificaciones
#[derive(Clone, Debug)]
pub struct NotificationService {
    pub messages: Arc<std::sync::Mutex<Vec<String>>>,
}

impl NotificationService {
    pub fn production() -> Self {
        Self {
            messages: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn mock() -> Self {
        Self {
            messages: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn send_email(&self, to: &str, subject: &str) {
        self.messages.lock().unwrap().push(format!(
            "EMAIL: {} - {}", to, subject
        ));
    }

    pub fn send_sms(&self, to: &str, message: &str) {
        self.messages.lock().unwrap().push(format!(
            "SMS: {} - {}", to, message
        ));
    }

    pub fn get_messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }

    pub fn message_count(&self) -> usize {
        self.messages.lock().unwrap().len()
    }
}

/// Servicio de pagos
#[derive(Clone, Debug)]
pub struct PaymentService {
    pub sandbox_mode: bool,
    pub processed_payments: Arc<std::sync::Mutex<Vec<(String, f64)>>>,
}

impl PaymentService {
    pub fn production() -> Self {
        Self {
            sandbox_mode: false,
            processed_payments: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn mock() -> Self {
        Self {
            sandbox_mode: true,
            processed_payments: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn process_payment(&self, order_id: &str, amount: f64) -> bool {
        self.processed_payments.lock().unwrap().push((order_id.to_string(), amount));
        // En sandbox siempre exitoso
        true
    }

    pub fn payment_count(&self) -> usize {
        self.processed_payments.lock().unwrap().len()
    }
}

/// Servicio de logging
#[derive(Clone, Debug)]
pub struct Logger {
    pub logs: Arc<std::sync::Mutex<Vec<String>>>,
}

impl Logger {
    pub fn production() -> Self {
        Self {
            logs: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn mock() -> Self {
        Self {
            logs: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn info(&self, message: &str) {
        self.logs.lock().unwrap().push(format!("[INFO] {}", message));
    }

    pub fn error(&self, message: &str) {
        self.logs.lock().unwrap().push(format!("[ERROR] {}", message));
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.lock().unwrap().clone()
    }
}

// ============================================================================
// DI Module
// ============================================================================

oxidi_core::module! {
    // Singletons
    single {
        Logger::production()
    }

    single {
        Database::production()
    }

    single {
        Cache::production()
    }

    single {
        ShippingService::production()
    }

    single {
        NotificationService::production()
    }

    single {
        PaymentService::production()
    }

    // Servicio dependiente
    single {
        let db: Database = get();
        let cache: Cache = get();
        ProductRepository::new(db, cache)
    }

}

// ============================================================================
// EJEMPLO 1: Override básico
// ============================================================================

/// El caso más simple: reemplazar un servicio con un mock.
///
/// En producción, `Database` usa la conexión real. En tests,
/// usamos una conexión mock para no depender de una base de datos real.
#[test]
fn ejemplo_01_override_basico() {
    // Creamos el contenedor de prueba con el override
    let container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "sqlite://:memory:".to_string(),
        })
        .build_with_container(OxidiContainer::with_overrides);

    // Resolvemos el servicio desde el contenedor de prueba
    let db: Database = get_from(&container);

    // Verificamos que es el mock, no la producción
    assert!(!db.is_production());
    assert_eq!(db.connection_string, "sqlite://:memory:");
}

// ============================================================================
// EJEMPLO 2: Servicios dependientes
// ============================================================================

/// Cuando un servicio depende de otro, el override se propaga automáticamente.
///
/// `ProductRepository` depende de `Database` y `Cache`. Al overridear `Database`,
/// el repositorio construido usará automáticamente la base de datos mock.
#[test]
fn ejemplo_02_servicios_dependientes() {
    let mock_db = Database {
        connection_string: "mock://test-db".to_string(),
    };

    let container = TestContainer::builder()
        .with_override::<Database>(mock_db)
        .build_with_container(OxidiContainer::with_overrides);

    // ProductRepository depende de Database
    let repo: ProductRepository = get_from(&container);

    // El repositorio usa la DB mock, no la de producción
    assert_eq!(repo.database.connection_string, "mock://test-db");
    assert!(!repo.database.is_production());
}

// ============================================================================
// EJEMPLO 3: Override de ShippingService
// ============================================================================

/// Overridear el servicio de envío para usar tarifas de prueba.
///
/// En producción, ShippingService calcula tarifas reales. En tests,
/// usamos una tarifa fija para hacer los resultados predecibles.
#[test]
fn ejemplo_03_override_shipping() {
    let mock_shipping = ShippingService::mock();
    let call_count = mock_shipping.call_count.clone();

    let container = TestContainer::builder()
        .with_override::<ShippingService>(mock_shipping)
        .build_with_container(OxidiContainer::with_overrides);

    // Resolvemos el shipping service
    let shipping: ShippingService = get_from(&container);

    // Verificamos que es el mock
    assert!(shipping.is_mock());
    assert_eq!(shipping.api_key, "sk-test-mock");

    // Calculamos una tarifa
    let rate = shipping.calculate_rate(5.0);
    assert_eq!(rate, 10.0); // Tarifa fija del mock

    // Verificamos que fue llamado
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

// ============================================================================
// EJEMPLO 4: Verificación de llamadas con PaymentService
// ============================================================================

/// Usar mocks para verificar que el servicio de pagos fue llamado.
///
/// Este patrón es esencial para tests donde necesitas asegurarte
/// que se procesa el pago sin cargar dinero real.
#[test]
fn ejemplo_04_verificacion_de_pagos() {
    let mock_payment = PaymentService::mock();
    let payment_records = mock_payment.processed_payments.clone();

    let container = TestContainer::builder()
        .with_override::<PaymentService>(mock_payment)
        .build_with_container(OxidiContainer::with_overrides);

    // Resolvemos el servicio de pagos
    let payment: PaymentService = get_from(&container);

    // Procesamos un pago
    let success = payment.process_payment("ORD-42", 99.99);
    assert!(success);

    // Verificamos que el pago fue registrado
    let records = payment_records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, "ORD-42");
    assert_eq!(records[0].1, 99.99);
}

// ============================================================================
// EJEMPLO 5: Aislamiento entre tests
// ============================================================================

/// Cada TestContainer es completamente independiente.
/// Los overrides de un test no afectan a otro test.
#[test]
fn ejemplo_05_aislamiento_entre_tests() {
    // Contenedor 1: con shipping mock
    let container1 = TestContainer::builder()
        .with_override::<ShippingService>(ShippingService::mock())
        .build_with_container(OxidiContainer::with_overrides);

    // Contenedor 2: sin overrides (valores de producción)
    let container2 = TestContainer::builder()
        .build_with_container(OxidiContainer::with_overrides);

    // Shipping 1 es mock
    let shipping1: ShippingService = get_from(&container1);
    assert!(shipping1.is_mock());

    // Shipping 2 es de producción
    let shipping2: ShippingService = get_from(&container2);
    assert!(!shipping2.is_mock());
}

// ============================================================================
// EJEMPLO 6: Tests paralelos
// ============================================================================

/// Los tests pueden ejecutarse en paralelo sin interferencia.
/// Cada thread tiene su propio contenedor aislado.
#[test]
fn ejemplo_06_tests_paralelos() {
    use std::thread;

    let results: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));

    let handle1 = {
        let results = results.clone();
        thread::spawn(move || {
            let container = TestContainer::builder()
                .with_override::<Database>(Database {
                    connection_string: "thread-1-mock".to_string(),
                })
                .build_with_container(OxidiContainer::with_overrides);

            let db: Database = get_from(&container);
            results.lock().unwrap().push(format!("Thread 1: {}", db.connection_string));
        })
    };

    let handle2 = {
        let results = results.clone();
        thread::spawn(move || {
            let container = TestContainer::builder()
                .with_override::<Database>(Database {
                    connection_string: "thread-2-mock".to_string(),
                })
                .build_with_container(OxidiContainer::with_overrides);

            let db: Database = get_from(&container);
            results.lock().unwrap().push(format!("Thread 2: {}", db.connection_string));
        })
    };

    let handle3 = {
        let results = results.clone();
        thread::spawn(move || {
            // Este thread no usa overrides
            let container = TestContainer::builder()
                .build_with_container(OxidiContainer::with_overrides);

            let db: Database = get_from(&container);
            results.lock().unwrap().push(format!("Thread 3: {}", db.connection_string));
        })
    };

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    let results = results.lock().unwrap();
    assert!(results.iter().any(|r| r.contains("thread-1-mock")));
    assert!(results.iter().any(|r| r.contains("thread-2-mock")));
    assert!(results.iter().any(|r| r.contains("prod")));
}

// ============================================================================
// EJEMPLO 7: Override de Cache para tests determinísticos
// ============================================================================

/// Overridear la caché para hacer los tests determinísticos.
///
/// En producción, la caché puede tener datos aleatorios. En tests,
/// desactivamos la caché para que siempre se consulte la DB.
#[test]
fn ejemplo_07_cache_desactivada() {
    let container = TestContainer::builder()
        .with_override::<Cache>(Cache::mock())
        .build_with_container(OxidiContainer::with_overrides);

    let repo: ProductRepository = get_from(&container);

    // Con cache mock, get() retorna None, simulando cache miss
    let result = repo.cache.get("product:1");
    assert!(result.is_none());

    // El repositorio sigue funcionando, solo sin caché
    let product = repo.find_by_id(1);
    assert!(product.is_some());
}

// ============================================================================
// EJEMPLO 8: Test async con contenedor
// ============================================================================

/// Los TestContainer funcionan correctamente en contextos async.
///
/// El contenedor es thread-safe y puede usarse desde código async
/// sin problemas, manteniendo los overrides a través de .await.
#[tokio::test]
async fn ejemplo_08_test_async() {
    let container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "async-test-db".to_string(),
        })
        .with_override::<ShippingService>(ShippingService::mock())
        .build_with_container(OxidiContainer::with_overrides);

    // Simulamos trabajo async
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // El contenedor sigue funcionando después de .await
    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "async-test-db");

    // Los overrides también se mantienen
    let shipping: ShippingService = get_from(&container);
    assert!(shipping.is_mock());
}

// ============================================================================
// EJEMPLO 9: has_override y get_override_instance
// ============================================================================

/// Verificar si un override existe y acceder directamente a él.
///
/// Útil para tests condicionales o para verificar la configuración del contenedor.
#[test]
fn ejemplo_09_has_override_y_get_instance() {
    let container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "mock-db".to_string(),
        })
        .with_override::<Cache>(Cache::mock())
        .build_with_container(OxidiContainer::with_overrides);

    // Verificamos qué overrides existen
    assert!(container.has_override::<Database>());
    assert!(container.has_override::<Cache>());
    assert!(!container.has_override::<ShippingService>());

    // Accedemos directamente al override
    let db: Database = container.get_override_instance();
    assert_eq!(db.connection_string, "mock-db");

    // get_override() también funciona
    let cache: Cache = get_override(&container);
    assert!(!cache.enabled);
}

// ============================================================================
// EJEMPLO 10: build() vs build_with_container()
// ============================================================================

/// Diferencia entre `build()` y `build_with_container()`.
///
/// - `build()`: solo almacena los overrides, usa `get_override()` para resolver
/// - `build_with_container()`: construye el contenedor real con overrides, usa `get_from()`
#[test]
fn ejemplo_10_build_vs_build_with_container() {
    // build() - solo overrides, sin contenedor real
    let simple_container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "simple-mock".to_string(),
        })
        .build();

    // Con build() usas get_override()
    let db: Database = get_override(&simple_container);
    assert_eq!(db.connection_string, "simple-mock");

    // build_with_container() - contenedor real con overrides
    let full_container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "full-mock".to_string(),
        })
        .build_with_container(OxidiContainer::with_overrides);

    // Con build_with_container() usas get_from()
    let db2: Database = get_from(&full_container);
    assert_eq!(db2.connection_string, "full-mock");

    // get_from() también funciona con servicios dependientes
    let repo: ProductRepository = get_from(&full_container);
    assert_eq!(repo.database.connection_string, "full-mock");
}

// ============================================================================
// EJEMPLO 11: Override de Logger para verificar logs
// ============================================================================

/// Un patrón común: overridear el logger para verificar los mensajes.
///
/// En producción, el logger escribe a stdout/file. En tests, capturamos
/// los logs para verificar que el código registra los mensajes correctos.
#[test]
fn ejemplo_11_override_logger() {
    let mock_logger = Logger::mock();
    let logs = mock_logger.logs.clone();

    let container = TestContainer::builder()
        .with_override::<Logger>(mock_logger)
        .build_with_container(OxidiContainer::with_overrides);

    // Obtenemos el logger del contenedor
    let logger: Logger = get_from(&container);

    // Simulamos operaciones que generan logs
    logger.info("Pedido creado");
    logger.error("Error de conexión");
    logger.info("Pedido enviado");

    // Verificamos los logs
    let logs = logs.lock().unwrap();
    assert_eq!(logs.len(), 3);
    assert!(logs[0].contains("[INFO]"));
    assert!(logs[1].contains("[ERROR]"));
    assert!(logs[2].contains("[INFO]"));
}

// ============================================================================
// EJEMPLO 12: Override de NotificationService
// ============================================================================

/// Overridear el servicio de notificaciones para verificar
/// que se envían los mensajes correctos sin enviar emails reales.
#[test]
fn ejemplo_12_override_notificaciones() {
    let mock_notifications = NotificationService::mock();
    let messages = mock_notifications.messages.clone();

    let container = TestContainer::builder()
        .with_override::<NotificationService>(mock_notifications)
        .build_with_container(OxidiContainer::with_overrides);

    // Resolvemos el servicio y enviamos notificaciones
    let notifications: NotificationService = get_from(&container);
    notifications.send_email("user@test.com", "Pedido ORD-7");
    notifications.send_sms("+1234567890", "Tu pedido fue enviado");

    // Verificamos que se registraron las notificaciones
    let msgs = messages.lock().unwrap();
    assert_eq!(msgs.len(), 2);
    assert!(msgs[0].contains("EMAIL"));
    assert!(msgs[1].contains("SMS"));
}

// ============================================================================
// EJEMPLO 13: Flujo completo con múltiples overrides
// ============================================================================

/// Test de integración completo: usar todos los mocks juntos.
///
/// Este ejemplo muestra cómo combinar múltiples overrides para probar
/// un flujo completo sin depender de servicios externos.
#[test]
fn ejemplo_13_flujo_completo() {
    // Configuramos todos los mocks
    let mock_db = Database {
        connection_string: "test-db".to_string(),
    };
    let mock_cache = Cache::mock();
    let mock_shipping = ShippingService::mock();
    let mock_payment = PaymentService::mock();
    let mock_notifications = NotificationService::mock();
    let mock_logger = Logger::mock();

    let shipping_count = mock_shipping.call_count.clone();
    let payment_records = mock_payment.processed_payments.clone();
    let notification_msgs = mock_notifications.messages.clone();

    let container = TestContainer::builder()
        .with_override::<Database>(mock_db)
        .with_override::<Cache>(mock_cache)
        .with_override::<ShippingService>(mock_shipping)
        .with_override::<PaymentService>(mock_payment)
        .with_override::<NotificationService>(mock_notifications)
        .with_override::<Logger>(mock_logger)
        .build_with_container(OxidiContainer::with_overrides);

    // Simulamos un flujo de pedido usando los servicios directamente
    let shipping: ShippingService = get_from(&container);
    let payment: PaymentService = get_from(&container);
    let notifications: NotificationService = get_from(&container);
    let logger: Logger = get_from(&container);

    // Paso 1: Calcular tarifa de envío
    let rate = shipping.calculate_rate(5.0);
    assert_eq!(rate, 10.0);

    // Paso 2: Procesar pago
    let order_id = "ORD-100";
    let success = payment.process_payment(order_id, rate);
    assert!(success);

    // Paso 3: Enviar notificación
    notifications.send_email("customer@example.com", &format!("Pedido {}", order_id));

    // Paso 4: Log del flujo
    logger.info(&format!("Pedido {} completado", order_id));

    // Verificaciones finales
    assert_eq!(shipping_count.load(Ordering::SeqCst), 1);
    assert_eq!(payment_records.lock().unwrap().len(), 1);
    assert_eq!(notification_msgs.lock().unwrap().len(), 1);
}

// ============================================================================
// EJEMPLO 14: Edge case - panic en get_from sin build_with_container
// ============================================================================

/// Si usas `build()` en lugar de `build_with_container()`,
/// `get_from()` panikea con un mensaje claro.
#[test]
#[should_panic(expected = "build_with_container")]
fn ejemplo_14_get_from_sin_build_with_container() {
    let container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "mock".to_string(),
        })
        .build(); // Sin build_with_container

    // Esto panikea porque no hay contenedor real
    let _: Database = get_from(&container);
}

// ============================================================================
// EJEMPLO 15: Edge case - panic en get_override sin override
// ============================================================================

/// Si intentas obtener un override que no existe,
/// `get_override()` panikea con un mensaje claro.
#[test]
#[should_panic(expected = "No override found")]
fn ejemplo_15_get_override_sin_override() {
    let container = TestContainer::builder()
        .build(); // Sin overrides

    // Esto panikea porque no hay override para Database
    let _: Database = get_override(&container);
}

// ============================================================================
// EJEMPLO 16: Override con Arc para compartir estado
// ============================================================================

/// Usar Arc para compartir estado entre el mock y el test.
///
/// Este patrón permite verificar el estado del mock después de
/// que el código bajo prueba lo ha usado.
#[test]
fn ejemplo_16_override_con_arc() {
    // Creamos el mock con Arc para compartir el estado
    let shared_count = Arc::new(AtomicU32::new(0));
    let mock_shipping = ShippingService {
        api_key: "sk-test".to_string(),
        call_count: shared_count.clone(),
    };

    let container = TestContainer::builder()
        .with_override::<ShippingService>(mock_shipping)
        .build_with_container(OxidiContainer::with_overrides);

    // Resolvemos el shipping service múltiples veces
    let shipping1: ShippingService = get_from(&container);
    let shipping2: ShippingService = get_from(&container);

    // Ambos son el mismo singleton
    shipping1.calculate_rate(1.0);
    shipping2.calculate_rate(2.0);

    // Verificamos el conteo compartido
    assert_eq!(shared_count.load(Ordering::SeqCst), 2);
}

// ============================================================================
// EJEMPLO 17: Async test con múltiples awaits
// ============================================================================

/// Test async que verifica que el contenedor sobrevive múltiples awaits.
#[tokio::test]
async fn ejemplo_17_async_multiple_awaits() {
    let container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "async-multi".to_string(),
        })
        .with_override::<ShippingService>(ShippingService::mock())
        .build_with_container(OxidiContainer::with_overrides);

    // Primer acceso
    let db1: Database = get_from(&container);
    assert_eq!(db1.connection_string, "async-multi");

    // Simular trabajo async
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Segundo acceso después de await
    let db2: Database = get_from(&container);
    assert_eq!(db2.connection_string, "async-multi");

    // Más trabajo async
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Tercer acceso
    let db3: Database = get_from(&container);
    assert_eq!(db3.connection_string, "async-multi");

    // Los overrides se mantienen en async
    let shipping: ShippingService = get_from(&container);
    let rate = shipping.calculate_rate(5.0);
    assert_eq!(rate, 10.0);
}

// ============================================================================
// EJEMPLO 18: Singleton retorna la misma instancia
// ============================================================================

/// Verificar que los servicios singleton comparten el mismo estado
/// cuando se resuelven múltiples veces.
#[test]
fn ejemplo_18_singleton_mismo_estado() {
    let mock_logger = Logger::mock();
    let logs = mock_logger.logs.clone();

    let container = TestContainer::builder()
        .with_override::<Logger>(mock_logger)
        .build_with_container(OxidiContainer::with_overrides);

    // Resolvemos el logger múltiples veces
    let logger1: Logger = get_from(&container);
    let logger2: Logger = get_from(&container);
    let logger3: Logger = get_from(&container);

    // Todos comparten el mismo estado (singleton)
    logger1.info("mensaje 1");
    logger2.info("mensaje 2");
    logger3.info("mensaje 3");

    // Los logs están en el mismo buffer
    let all_logs = logs.lock().unwrap();
    assert_eq!(all_logs.len(), 3);
    assert!(all_logs[0].contains("mensaje 1"));
    assert!(all_logs[1].contains("mensaje 2"));
    assert!(all_logs[2].contains("mensaje 3"));
}

// ============================================================================
// EJEMPLO 19: Override condicional
// ============================================================================

/// Usar has_override para tests condicionales.
///
/// Algunos tests pueden necesitar verificar si un override fue aplicado
/// antes de ejecutar lógica específica.
#[test]
fn ejemplo_19_override_condicional() {
    let container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "conditional".to_string(),
        })
        .build_with_container(OxidiContainer::with_overrides);

    // Test condicional basado en overrides
    if container.has_override::<Database>() {
        let db: Database = get_from(&container);
        assert!(!db.is_production());
    }

    if !container.has_override::<ShippingService>() {
        // Sin override, usa el valor de producción
        let shipping: ShippingService = get_from(&container);
        assert!(!shipping.is_mock());
    }
}

// ============================================================================
// EJEMPLO 20: Drop y cleanup automático
// ============================================================================

/// El TestContainer limpia automáticamente al ser dropped.
///
/// No necesitas llamar manualmente a cleanup().
#[test]
fn ejemplo_20_drop_automatico() {
    let container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "auto-cleanup".to_string(),
        })
        .build_with_container(OxidiContainer::with_overrides);

    let db: Database = get_from(&container);
    assert_eq!(db.connection_string, "auto-cleanup");

    // El container se limpia automáticamente al salir del scope
}

// ============================================================================
// EJEMPLO 21: overrides() para inspección
// ============================================================================

/// El método `overrides()` retorna el mapa de overrides para inspección.
#[test]
fn ejemplo_21_overrides_map() {
    let container = TestContainer::builder()
        .with_override::<Database>(Database {
            connection_string: "inspect".to_string(),
        })
        .with_override::<Cache>(Cache::mock())
        .with_override::<Logger>(Logger::mock())
        .build_with_container(OxidiContainer::with_overrides);

    let overrides = container.overrides();
    assert_eq!(overrides.len(), 3);
}

// ============================================================================
// EJEMPLO 22: Testing de errores con PaymentService
// ============================================================================

/// Usar mocks para simular el procesamiento de pagos
/// y verificar que se registran correctamente.
#[test]
fn ejemplo_22_testing_de_pagos() {
    let mock_payment = PaymentService::mock();
    let payment_records = mock_payment.processed_payments.clone();

    let container = TestContainer::builder()
        .with_override::<PaymentService>(mock_payment)
        .build_with_container(OxidiContainer::with_overrides);

    let payment: PaymentService = get_from(&container);

    // Procesamos múltiples pagos
    payment.process_payment("ORD-1", 50.0);
    payment.process_payment("ORD-2", 75.5);
    payment.process_payment("ORD-3", 120.0);

    // Verificamos que todos se registraron
    let records = payment_records.lock().unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0], ("ORD-1".to_string(), 50.0));
    assert_eq!(records[1], ("ORD-2".to_string(), 75.5));
    assert_eq!(records[2], ("ORD-3".to_string(), 120.0));
}
