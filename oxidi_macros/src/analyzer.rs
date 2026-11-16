use std::collections::VecDeque;
use syn::Type;

use crate::dsl_parser::{ModuleDef, ServiceDef};

/// Result of dependency analysis
pub struct AnalysisResult {
    /// Services in topological order (dependencies first)
    pub ordered_services: Vec<usize>,
    /// Any errors found
    pub errors: Vec<String>,
}

/// Build a dependency graph and analyze it for cycles and missing dependencies
pub fn analyze(module: &ModuleDef) -> AnalysisResult {
    let mut errors = Vec::new();
    
    // Build a map of service index -> service type string for error messages
    let service_count = module.services.len();
    
    // Build adjacency list: service_i depends on service_j means edge i -> j
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); service_count];
    
    for (i, service) in module.services.iter().enumerate() {
        for dep_type in &service.dependencies {
            // Find which service provides this type
            match find_service_by_type(module, dep_type) {
                Some(dep_idx) => {
                    adj[i].push(dep_idx);
                }
                None => {
                    let type_str = type_to_string(dep_type);
                    errors.push(format!(
                        "Dependency error: Service at position {} depends on `{}`, \
                        but no service provides this type. Make sure you have a `single {{ {}::new(...) }}` or `factory {{ {}::new(...) }}` block.",
                        i, type_str, type_str, type_str
                    ));
                }
            }
        }
    }
    
    // Detect cycles using DFS
    let cycles = detect_cycles(&adj, &module.services);
    for cycle in cycles {
        let path = cycle.iter()
            .map(|&i| service_name(&module.services[i]))
            .collect::<Vec<_>>()
            .join(" -> ");
        errors.push(format!(
            "Circular dependency detected: {} -> {}. \
            Remove one of the dependencies to break the cycle.",
            path,
            service_name(&module.services[cycle[0]])
        ));
    }
    
    // If there are errors, return them without topological sort
    if !errors.is_empty() {
        return AnalysisResult {
            ordered_services: vec![],
            errors,
        };
    }
    
    // Topological sort (Kahn's algorithm)
    let ordered = topological_sort(&adj, service_count);
    
    AnalysisResult {
        ordered_services: ordered,
        errors,
    }
}

/// Find the service index that provides the given type
fn find_service_by_type(module: &ModuleDef, target_type: &Type) -> Option<usize> {
    for (i, service) in module.services.iter().enumerate() {
        // Check if the return type of this service matches the target
        // We infer the return type from the body expression
        if types_match(&infer_return_type(&service.body), target_type) {
            return Some(i);
        }
    }
    None
}

/// Infer the return type from an expression.
/// For `Type::new(...)` calls, extracts just `Type` (the struct type, not the full path including `new`).
fn infer_return_type(expr: &syn::Expr) -> Type {
    match expr {
        syn::Expr::Block(block) => {
            // Look at the last expression in the block
            if let Some(last_stmt) = block.block.stmts.last() {
                match last_stmt {
                    syn::Stmt::Expr(e, _) => infer_return_type(e),
                    syn::Stmt::Local(local) => {
                        if let Some(init) = &local.init {
                            infer_return_type(&init.expr)
                        } else {
                            Type::Verbatim(quote::quote! { () })
                        }
                    }
                    _ => Type::Verbatim(quote::quote! { () }),
                }
            } else {
                Type::Verbatim(quote::quote! { () })
            }
        }
        syn::Expr::Call(call) => {
            // Extract type from `Type::new(...)` pattern
            // The path is `Type::new` — we want just `Type`
            if let syn::Expr::Path(path) = &*call.func {
                let mut segments = path.path.segments.clone();
                
                // Remove the last segment (`new`, `default`, etc.)
                if segments.len() > 1 {
                    segments.pop();
                    return Type::Path(syn::TypePath {
                        qself: None,
                        path: syn::Path {
                            leading_colon: path.path.leading_colon,
                            segments,
                        },
                    });
                }
            }
            Type::Verbatim(quote::quote! { () })
        }
        _ => Type::Verbatim(quote::quote! { () }),
    }
}

/// Check if two types match (simple string comparison)
fn types_match(t1: &Type, t2: &Type) -> bool {
    type_to_string(t1) == type_to_string(t2)
}

/// Convert a type to a string for comparison
fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Path(tp) => tp.path.segments.iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        _ => quote::quote!(#ty).to_string(),
    }
}

/// Get a human-readable name for a service
fn service_name(service: &ServiceDef) -> String {
    match &*service.body {
        syn::Expr::Block(block) => {
            if let Some(last_stmt) = block.block.stmts.last() {
                if let syn::Stmt::Expr(expr, _) = last_stmt {
                    if let syn::Expr::Call(call) = expr {
                        if let syn::Expr::Path(path) = &*call.func {
                            if let Some(segment) = path.path.segments.first() {
                                return segment.ident.to_string();
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    service.name.to_string()
}

/// Detect cycles in the dependency graph using DFS
fn detect_cycles(adj: &[Vec<usize>], services: &[ServiceDef]) -> Vec<Vec<usize>> {
    let mut cycles = Vec::new();
    let n = adj.len();
    
    // 0 = unvisited, 1 = in current path, 2 = fully processed
    let mut state = vec![0u8; n];
    let mut path = Vec::new();
    
    for start in 0..n {
        if state[start] == 0 {
            dfs_cycle(start, adj, &mut state, &mut path, &mut cycles, services);
        }
    }
    
    cycles
}

fn dfs_cycle(
    node: usize,
    adj: &[Vec<usize>],
    state: &mut Vec<u8>,
    path: &mut Vec<usize>,
    cycles: &mut Vec<Vec<usize>>,
    _services: &[ServiceDef],
) {
    state[node] = 1;
    path.push(node);
    
    for &neighbor in &adj[node] {
        if state[neighbor] == 1 {
            // Found a cycle: extract it from the path
            let start = path.iter().position(|&p| p == neighbor).unwrap();
            let cycle = path[start..].to_vec();
            cycles.push(cycle);
        } else if state[neighbor] == 0 {
            dfs_cycle(neighbor, adj, state, path, cycles, _services);
        }
    }
    
    path.pop();
    state[node] = 2;
}

/// Topological sort using Kahn's algorithm
fn topological_sort(adj: &[Vec<usize>], n: usize) -> Vec<usize> {
    let mut in_degree = vec![0usize; n];
    for neighbors in adj {
        for &neighbor in neighbors {
            in_degree[neighbor] += 1;
        }
    }
    
    let mut queue = VecDeque::new();
    for i in 0..n {
        if in_degree[i] == 0 {
            queue.push_back(i);
        }
    }
    
    let mut result = Vec::new();
    while let Some(node) = queue.pop_front() {
        result.push(node);
        for &neighbor in &adj[node] {
            in_degree[neighbor] -= 1;
            if in_degree[neighbor] == 0 {
                queue.push_back(neighbor);
            }
        }
    }
    
    result
}
