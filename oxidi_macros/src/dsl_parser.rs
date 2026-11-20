use syn::{parse::Parser, Expr, Block, Ident};

/// Represents a service definition in the DSL
#[derive(Debug, Clone)]
pub struct ServiceDef {
    pub name: Ident,
    pub service_type: ServiceType,
    pub body: Box<Expr>,
    /// Types referenced via `get::<T>()` inside the body
    pub dependencies: Vec<syn::Type>,
}

/// Type of service: singleton (one instance) or factory (new instance each time) or scoped (one per scope)
#[derive(Debug, Clone, Copy)]
pub enum ServiceType {
    Single,
    Factory,
    Scoped,
}

/// Parsed result from the `module!()` macro
#[derive(Debug, Default)]
pub struct ModuleDef {
    pub services: Vec<ServiceDef>,
}

/// Parse the DSL inside `module! { ... }`
///
/// The DSL supports:
/// - `single { ... }` — singleton service
/// - `factory { ... }` — factory service (new instance each time)
/// - `scoped { ... }` — scoped service (one instance per scope)
pub fn parse_module(input: proc_macro::TokenStream) -> syn::Result<ModuleDef> {
    let parser = |input: syn::parse::ParseStream| {
        let mut services = Vec::new();
        
        while !input.is_empty() {
            let service_type;
            let block;
            
            // Look ahead to determine the service type
            let lookahead = input.lookahead1();
            
            if lookahead.peek(Ident) {
                let temp_ident: Ident = input.parse()?;
                
                if temp_ident == "single" {
                    service_type = ServiceType::Single;
                } else if temp_ident == "factory" {
                    service_type = ServiceType::Factory;
                } else if temp_ident == "scoped" {
                    service_type = ServiceType::Scoped;
                } else {
                    return Err(input.error(format!(
                        "Expected `single`, `factory`, or `scoped`, found `{}`. \
                        Use `single {{ ... }}` for singletons, `factory {{ ... }}` for factories, \
                        or `scoped {{ ... }}` for per-scope services.",
                        temp_ident
                    )));
                }
                
                // Parse the block
                block = input.parse()?;
            } else {
                return Err(lookahead.error());
            }
            
            // Generate a unique name for the service based on index
            let name = syn::Ident::new(
                &format!("__service_{}", services.len()),
                proc_macro2::Span::call_site(),
            );
            
            // Extract dependencies from the block by scanning for `get::<T>()` calls
            let dependencies = extract_get_dependencies(&block);
            
            // Convert block to expression
            let body = Box::new(syn::Expr::Block(syn::ExprBlock {
                attrs: vec![],
                label: None,
                block: block,
            }));
            
            services.push(ServiceDef {
                name,
                service_type,
                body,
                dependencies,
            });
        }
        
        Ok(ModuleDef { services })
    };
    
    parser.parse(input)
}

/// Extract types from `get::<T>()` calls in the expression
fn extract_get_dependencies(block: &Block) -> Vec<syn::Type> {
    let mut deps = Vec::new();
    
    for stmt in &block.stmts {
        if let syn::Stmt::Expr(expr, _) = stmt {
            extract_types_from_expr(expr, &mut deps);
        } else if let syn::Stmt::Local(local) = stmt {
            if let Some(init) = &local.init {
                extract_types_from_expr(&init.expr, &mut deps);
            }
        }
    }
    
    deps
}

/// Recursively extract type arguments from `get::<T>()` calls
fn extract_types_from_expr(expr: &Expr, deps: &mut Vec<syn::Type>) {
    match expr {
        Expr::Call(call) => {
            // Check if this is a `get::<T>()` call
            if let Expr::Path(path) = &*call.func {
                if let Some(ident) = path.path.get_ident() {
                    if ident == "get" {
                        // Extract type from turbofish: get::<T>()
                        if let Some(segment) = path.path.segments.last() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                for arg in &args.args {
                                    if let syn::GenericArgument::Type(ty) = arg {
                                        deps.push(ty.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Recurse into arguments
            for arg in &call.args {
                extract_types_from_expr(arg, deps);
            }
        }
        Expr::Block(block) => {
            for stmt in &block.block.stmts {
                if let syn::Stmt::Expr(e, _) = stmt {
                    extract_types_from_expr(e, deps);
                } else if let syn::Stmt::Local(local) = stmt {
                    if let Some(init) = &local.init {
                        extract_types_from_expr(&init.expr, deps);
                    }
                }
            }
        }
        Expr::Binary(binary) => {
            extract_types_from_expr(&binary.left, deps);
            extract_types_from_expr(&binary.right, deps);
        }
        Expr::MethodCall(method_call) => {
            extract_types_from_expr(&method_call.receiver, deps);
            for arg in &method_call.args {
                extract_types_from_expr(arg, deps);
            }
        }
        _ => {}
    }
}
