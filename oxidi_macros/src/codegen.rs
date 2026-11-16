use quote::{quote, format_ident};
use syn::{Type, Expr};

use crate::dsl_parser::{ModuleDef, ServiceType};
use crate::analyzer::AnalysisResult;

/// Generate the container struct and implementation.
/// Returns a tuple: (struct + impl code, OxidiResolve trait implementations).
/// The resolve impls are separate so they can be emitted at module level for IDE indexing.
pub fn generate(module: &ModuleDef, analysis: &AnalysisResult) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let ordered = &analysis.ordered_services;
    let services = &module.services;
    
    if services.is_empty() {
        return (quote! {
            pub struct OxidiContainer;
            
            impl OxidiContainer {
                pub fn new() -> Self {
                    OxidiContainer
                }
            }
        }, quote! {});
    }
    
    // Collect field info: (field_name, return_type, service_type, body)
    let mut field_info: Vec<(syn::Ident, Type, ServiceType, &Box<Expr>)> = Vec::new();
    
    for &idx in ordered {
        let service = &services[idx];
        let field_name = format_ident!("__service_{}", idx);
        let return_type = infer_type_from_expr(&service.body);
        field_info.push((field_name, return_type, service.service_type, &service.body as &Box<Expr>));
    }
    
    // Build type-to-field mapping
    let type_to_field: std::collections::HashMap<String, syn::Ident> = field_info.iter()
        .map(|(name, ty, _, _)| (type_to_string(ty), name.clone()))
        .collect();
    
    // Generate struct fields - only singletons are stored; factories are methods
    let struct_fields: Vec<_> = field_info.iter().filter_map(|(name, ty, stype, _body)| {
        match stype {
            ServiceType::Single => Some(quote! {
                #name: #ty,
            }),
            ServiceType::Factory => None,
        }
    }).collect();
    
    // Generate constructor steps - only for singletons
    let constructor_steps: Vec<_> = field_info.iter().filter_map(|(name, _ty, stype, body)| {
        match stype {
            ServiceType::Single => {
                let resolved_body = replace_get_calls(body, &type_to_field);
                Some(quote! {
                    let #name = { #resolved_body };
                })
            }
            ServiceType::Factory => None,
        }
    }).collect();
    
    // Generate struct initialization - only singleton fields
    let field_names: Vec<_> = field_info.iter()
        .filter(|(_, _, stype, _)| matches!(stype, ServiceType::Single))
        .map(|(name, _, _, _)| name.clone())
        .collect();
    
    // Generate factory methods
    let factory_methods: Vec<_> = field_info.iter().filter_map(|(name, ty, stype, body)| {
        match stype {
            ServiceType::Factory => {
                let resolved_body = replace_get_calls(body, &type_to_field);
                // For factory methods, replace __service_N with self.__service_N
                let body_tokens = quote::quote!(#resolved_body);
                let body_str = body_tokens.to_string();
                
                // Build self-aware replacements
                let mut fixed = body_str.clone();
                for field_name in type_to_field.values() {
                    let f = field_name.to_string();
                    // Replace standalone __service_N.clone() with self.__service_N.clone()
                    let old = format!("{} . clone ( )", f);
                    let new = format!("self . {} . clone ( )", f);
                    fixed = fixed.replace(&old, &new);
                    // Also try without spaces
                    let old2 = format!("{}.clone()", f);
                    let new2 = format!("self.{}.clone()", f);
                    fixed = fixed.replace(&old2, &new2);
                }
                
                let resolved_body2 = match fixed.parse::<proc_macro2::TokenStream>() {
                    Ok(ts) => ts,
                    Err(_) => body_tokens, // fallback to original
                };
                
                let name_str = name.to_string();
                // Extract the index from __service_N and create a valid identifier
                let method_name = name_str.strip_prefix("__service_")
                    .map(|n| format!("service_{}", n))
                    .unwrap_or_else(|| "service".to_string());
                let method_ident = syn::Ident::new(&method_name, proc_macro2::Span::call_site());
                Some(quote! {
                    pub fn #method_ident(&self) -> #ty {
                        #resolved_body2
                    }
                })
            }
            ServiceType::Single => None,
        }
    }).collect();
    
    // Generate OxidiResolve implementations for each service type
    let resolve_impls: Vec<_> = field_info.iter().map(|(name, ty, stype, _body)| {
        match stype {
            ServiceType::Single => quote! {
                impl oxidi_core::OxidiResolve for #ty {
                    fn resolve(container: &std::boxed::Box<dyn std::any::Any + Send + Sync>) -> Self {
                        let c = container.downcast_ref::<OxidiContainer>()
                            .expect("Container type mismatch");
                        c.#name.clone()
                    }
                }
            },
            ServiceType::Factory => {
                let name_str = name.to_string();
                let method_name = name_str.trim_start_matches("__").to_string();
                let method_ident = format_ident!("{}", method_name);
                quote! {
                    impl oxidi_core::OxidiResolve for #ty {
                        fn resolve(container: &std::boxed::Box<dyn std::any::Any + Send + Sync>) -> Self {
                            let c = container.downcast_ref::<OxidiContainer>()
                                .expect("Container type mismatch");
                            c.#method_ident()
                        }
                    }
                }
            }
        }
    }).collect();
    
    // Generate the struct and impl (goes inside the block)
    let struct_and_impl = quote! {
        pub struct OxidiContainer {
            #(#struct_fields)*
        }
        
        impl OxidiContainer {
            pub fn new() -> Self {
                #(#constructor_steps)*
                Self {
                    #(#field_names),*
                }
            }
            
            #(#factory_methods)*
        }
    };
    
    // Generate OxidiResolve implementations (emitted at module level for IDE indexing)
    let resolve_impls = quote! {
        #(#resolve_impls)*
    };
    
    (struct_and_impl, resolve_impls)
}

/// Replace `get::<T>()` and `let x: T = get()` patterns with local variable references.
/// Uses string-based replacement on the token output for simplicity.
fn replace_get_calls(
    body: &Box<Expr>,
    type_to_field: &std::collections::HashMap<String, syn::Ident>,
) -> proc_macro2::TokenStream {
    let tokens = quote::quote!(#body);
    let mut result = tokens.to_string();
    
    
    // Step 1: Replace `let x: Type = get()` with `let x = field.clone()`
    // The token output has spaces: "let x : Type = get()"
    for (type_str, field_name) in type_to_field {
        // Try with spaces around colon (quote! adds spaces)
        let search_pattern = format!(" : {} = get()", type_str);
        if let Some(pos) = result.find(&search_pattern) {
            // Find the variable name before ": Type"
            let before = &result[..pos];
            if let Some(let_pos) = before.rfind("let ") {
                let var_name = before[let_pos + 4..].trim().to_string();
                let old = format!("let {} : {} = get()", var_name, type_str);
                let new = format!("let {} = {}.clone()", var_name, field_name);
                result = result.replace(&old, &new);
            }
        }
    }
    
    // Step 2: Replace `get::<Type>()` with `field.clone()`
    for (type_str, field_name) in type_to_field {
        let pattern = format!("get::<{}>()", type_str);
        let replacement = format!("{}.clone()", field_name);
        result = result.replace(&pattern, &replacement);
    }
    
    
    // Parse back to tokens
    match result.parse::<proc_macro2::TokenStream>() {
        Ok(ts) => ts,
        Err(e) => {
            quote::quote!(#body)
        }
    }
}

fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Path(tp) => tp.path.segments.iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        _ => quote::quote!(#ty).to_string(),
    }
}

/// Infer the return type from an expression.
/// For `Type::new(...)` calls, extracts just `Type` (the struct type, not the full path including `new`).
fn infer_type_from_expr(expr: &Expr) -> Type {
    match expr {
        Expr::Block(block) => {
            // Look at the last expression in the block
            if let Some(last_stmt) = block.block.stmts.last() {
                match last_stmt {
                    syn::Stmt::Expr(e, _) => infer_type_from_expr(e),
                    syn::Stmt::Local(local) => {
                        if let Some(init) = &local.init {
                            infer_type_from_expr(&init.expr)
                        } else {
                            Type::Verbatim(quote! { () })
                        }
                    }
                    _ => Type::Verbatim(quote! { () }),
                }
            } else {
                Type::Verbatim(quote! { () })
            }
        }
        Expr::Call(call) => {
            // Extract type from `Type::new(...)` pattern
            // The path is `Type::new` — we want just `Type`
            if let Expr::Path(path) = &*call.func {
                let mut segments = path.path.segments.clone();
                
                // Remove the last segment (`new`, `default`, etc.)
                if segments.len() > 1 {
                    segments.pop();
                }
                // Build a clean path without leading_colon issues
                let path_str = segments.iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                
                
                if segments.is_empty() {
                    return Type::Verbatim(quote! { () });
                }
                
                // Use Verbatim to avoid path construction issues
                let ident = segments.last().unwrap().ident.clone();
                if segments.len() == 1 {
                    return Type::Verbatim(quote! { #ident });
                }
                return Type::Path(syn::TypePath {
                    qself: None,
                    path: syn::Path {
                        leading_colon: None,
                        segments,
                    },
                });
            }
            Type::Verbatim(quote! { () })
        }
        _ => Type::Verbatim(quote! { () }),
    }
}
