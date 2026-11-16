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
/// - `get::<T>()` — Resolve a dependency inside the block
///
/// # Example
///
/// ```rust,ignore
/// use oxidi_macros::module;
/// use oxidi_core::get;
///
/// fn main() {
///     oxidi::init(module! {
///         single {
///             Database::new("postgres://localhost")
///         }
///
///         single {
///             let db: Database = get();
///             UserRepository::new(db)
///         }
///
///         factory {
///             let repo: UserRepository = get();
///             RequestHandler::new(repo)
///         }
///     });
///
///     // Access services from anywhere
///     let db: Database = get();
///     let handler = get::<RequestHandler>();
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
    
    // Everything goes inside a single block so it's a valid expression.
    // The struct, impl, and resolve impls are all defined inside, then we return the instance.
    let output = quote! {
        {
            #struct_and_impl
            #resolve_impls
            OxidiContainer::new()
        }
    };
    
    output.into()
}
