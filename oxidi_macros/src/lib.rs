mod dsl_parser;
mod analyzer;
mod codegen;
mod plugin;

use proc_macro::TokenStream;
use quote::quote;

use dsl_parser::parse_module;
use analyzer::analyze;
use codegen::generate;

/// Define a dependency injection module with Koin-style DSL.
///
/// This macro parses the DSL inside the block and generates a typed container
/// struct with all services registered. The container supports:
///
/// - `single { ... }` — Singleton service (one instance, shared)
/// - `factory { ... }` — Factory service (new instance each time)
/// - `scoped { ... }` — Scoped service (one instance per scope)
/// - `get::<T>()` — Resolve a dependency inside the block
///
/// The macro must be placed at the **module level** (outside any function).
/// It generates:
/// - A container struct (`OxidiContainer`)
/// - `OxidiResolve` trait implementations for each service type
/// - A `__oxidi_init__()` function to initialize the global container
///
/// # Example
///
/// ```rust,ignore
/// use oxidi_core::module;
/// use oxidi_core::get;
///
/// // 1. Define the module at the top level (outside any function)
/// module! {
///     single {
///         Database::new("postgres://localhost")
///     }
///
///     single {
///         let db: Database = get();
///         UserRepository::new(db)
///     }
///
///     factory {
///         let repo: UserRepository = get();
///         RequestHandler::new(repo)
///     }
/// }
///
/// fn main() {
///     // 2. Initialize the container
///     __oxidi_init__();
///
///     // 3. Access services from anywhere
///     let db: Database = get();
///     let handler = get::<RequestHandler>();
/// }
/// ```
///
/// # Scoped Services
///
/// Scoped services are created once per scope. Use `scope(|| { ... })` to
/// activate a scope:
///
/// ```rust,ignore
/// use oxidi_core::{module, get, scope};
///
/// module! {
///     single { Database::new("postgres://localhost") }
///
///     scoped {
///         let db: Database = get();
///         Transaction::new(db)
///     }
/// }
///
/// fn main() {
///     __oxidi_init__();
///
///     scope(|| {
///         let tx: Transaction = get(); // new instance, cached in this scope
///     }); // tx is dropped here
/// }
/// ```
#[proc_macro]
pub fn module(input: TokenStream) -> TokenStream {
    // Parse the DSL
    let module_def = match parse_module(input) {
        Ok(def) => def,
        Err(err) => {
            return err.to_compile_error().into();
        }
    };
    
    // Analyze dependencies
    let analysis = analyze(&module_def);
    
    // Report any errors
    if !analysis.errors.is_empty() {
        let errors: Vec<_> = analysis.errors.iter()
            .map(|e| quote! { compile_error!(#e); })
            .collect();
        
        return quote! {
            #(#errors)*
        }.into();
    }
    
    // Generate code: struct definition + impl + OxidiResolve traits
    let (struct_and_impl, resolve_impls) = generate(&module_def, &analysis);
    
    // Emit all items at module level so rust-analyzer can index the trait impls.
    // The struct, impl blocks, and OxidiResolve impls are all top-level items.
    // A helper function __oxidi_init__() is generated to build and store the container.
    let output = quote! {
        #struct_and_impl
        #resolve_impls

        /// Internal function to initialize the Oxidi container.
        pub fn __oxidi_init__() {
            let container = OxidiContainer::new();
            oxidi_core::__private_init(container);
        }
    };

    output.into()
}
