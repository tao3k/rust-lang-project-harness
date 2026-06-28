//! Async boundary facts for synchronous lock guards.

use std::collections::BTreeSet;

use syn::visit::{self, Visit};

pub(crate) fn sync_lock_guard_across_await_count(
    signature: &syn::Signature,
    block: &syn::Block,
) -> usize {
    let mut collector = SyncLockGuardAcrossAwaitCollector {
        sync_lock_receivers: sync_lock_receiver_bindings(signature),
        ..Default::default()
    };
    collector.visit_block(block);
    collector.count
}

#[derive(Default)]
struct SyncLockGuardAcrossAwaitCollector {
    count: usize,
    sync_lock_receivers: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for SyncLockGuardAcrossAwaitCollector {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let mut active_guards = BTreeSet::<String>::new();
        for statement in &block.stmts {
            if let Some(guard) = sync_lock_guard_binding(statement, &self.sync_lock_receivers) {
                active_guards.insert(guard);
            }
            if !active_guards.is_empty() && statement_contains_await(statement) {
                self.count += active_guards.len();
                active_guards.clear();
            }
            if let Some(guard) = explicit_drop_guard(statement) {
                active_guards.remove(&guard);
            }
            visit::visit_stmt(self, statement);
        }
    }
}

fn sync_lock_guard_binding(
    statement: &syn::Stmt,
    sync_lock_receivers: &BTreeSet<String>,
) -> Option<String> {
    let syn::Stmt::Local(local) = statement else {
        return None;
    };
    let syn::Pat::Ident(binding) = &local.pat else {
        return None;
    };
    let initializer = local.init.as_ref()?;
    let binding_name = binding.ident.to_string();
    sync_lock_guard_initializer(&initializer.expr, &binding_name, sync_lock_receivers)
        .then_some(binding_name)
}

fn sync_lock_guard_initializer(
    expr: &syn::Expr,
    binding_name: &str,
    sync_lock_receivers: &BTreeSet<String>,
) -> bool {
    if expr_contains_await(expr) {
        return false;
    }
    match expr {
        syn::Expr::MethodCall(method_call)
            if matches!(method_call.method.to_string().as_str(), "lock") =>
        {
            true
        }
        syn::Expr::MethodCall(method_call)
            if matches!(method_call.method.to_string().as_str(), "read" | "write") =>
        {
            binding_name_suggests_guard(binding_name)
                || receiver_is_known_sync_lock(method_call.receiver.as_ref(), sync_lock_receivers)
        }
        syn::Expr::MethodCall(method_call)
            if matches!(method_call.method.to_string().as_str(), "unwrap" | "expect") =>
        {
            sync_lock_guard_initializer(&method_call.receiver, binding_name, sync_lock_receivers)
        }
        syn::Expr::Try(try_expr) => {
            sync_lock_guard_initializer(&try_expr.expr, binding_name, sync_lock_receivers)
        }
        syn::Expr::Paren(paren) => {
            sync_lock_guard_initializer(&paren.expr, binding_name, sync_lock_receivers)
        }
        _ => false,
    }
}

fn binding_name_suggests_guard(binding_name: &str) -> bool {
    binding_name.contains("guard")
}

fn receiver_is_known_sync_lock(expr: &syn::Expr, sync_lock_receivers: &BTreeSet<String>) -> bool {
    match expr {
        syn::Expr::Path(path) if path.path.segments.len() == 1 => {
            sync_lock_receivers.contains(&path.path.segments[0].ident.to_string())
        }
        syn::Expr::Field(field) => receiver_is_known_sync_lock(&field.base, sync_lock_receivers),
        syn::Expr::Paren(paren) => receiver_is_known_sync_lock(&paren.expr, sync_lock_receivers),
        syn::Expr::Reference(reference) => {
            receiver_is_known_sync_lock(&reference.expr, sync_lock_receivers)
        }
        _ => false,
    }
}

fn sync_lock_receiver_bindings(signature: &syn::Signature) -> BTreeSet<String> {
    signature
        .inputs
        .iter()
        .filter_map(|input| {
            let syn::FnArg::Typed(argument) = input else {
                return None;
            };
            type_contains_sync_lock(&argument.ty)
                .then(|| pat_ident(&argument.pat).map(ToString::to_string))
                .flatten()
        })
        .collect()
}

fn type_contains_sync_lock(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(path) => path_contains_sync_lock(&path.path),
        syn::Type::Reference(reference) => type_contains_sync_lock(&reference.elem),
        syn::Type::Paren(paren) => type_contains_sync_lock(&paren.elem),
        syn::Type::Group(group) => type_contains_sync_lock(&group.elem),
        syn::Type::Tuple(tuple) => tuple.elems.iter().any(type_contains_sync_lock),
        _ => false,
    }
}

fn path_contains_sync_lock(path: &syn::Path) -> bool {
    path.segments.iter().any(|segment| {
        matches!(segment.ident.to_string().as_str(), "Mutex" | "RwLock")
            || path_arguments_contain_sync_lock(&segment.arguments)
    })
}

fn path_arguments_contain_sync_lock(arguments: &syn::PathArguments) -> bool {
    let syn::PathArguments::AngleBracketed(arguments) = arguments else {
        return false;
    };
    arguments.args.iter().any(|argument| match argument {
        syn::GenericArgument::Type(ty) => type_contains_sync_lock(ty),
        _ => false,
    })
}

fn pat_ident(pat: &syn::Pat) -> Option<&syn::Ident> {
    let syn::Pat::Ident(binding) = pat else {
        return None;
    };
    Some(&binding.ident)
}

fn explicit_drop_guard(statement: &syn::Stmt) -> Option<String> {
    let syn::Stmt::Expr(expr, _) = statement else {
        return None;
    };
    let syn::Expr::Call(call) = expr else {
        return None;
    };
    if !expr_path_ends_with(call.func.as_ref(), &["drop"]) || call.args.len() != 1 {
        return None;
    }
    call.args
        .first()
        .and_then(expr_path_ident)
        .map(ToString::to_string)
}

fn statement_contains_await(statement: &syn::Stmt) -> bool {
    let mut collector = AwaitExpressionCollector::default();
    collector.visit_stmt(statement);
    collector.found
}

fn expr_contains_await(expr: &syn::Expr) -> bool {
    let mut collector = AwaitExpressionCollector::default();
    collector.visit_expr(expr);
    collector.found
}

#[derive(Default)]
struct AwaitExpressionCollector {
    found: bool,
}

impl<'ast> Visit<'ast> for AwaitExpressionCollector {
    fn visit_expr_await(&mut self, _await: &'ast syn::ExprAwait) {
        self.found = true;
    }
}

fn expr_path_ends_with(expr: &syn::Expr, expected: &[&str]) -> bool {
    let syn::Expr::Path(path) = expr else {
        return false;
    };
    if path.path.segments.len() < expected.len() {
        return false;
    }
    path.path
        .segments
        .iter()
        .rev()
        .zip(expected.iter().rev())
        .all(|(segment, expected)| segment.ident == *expected)
}

fn expr_path_ident(expr: &syn::Expr) -> Option<&syn::Ident> {
    let syn::Expr::Path(path) = expr else {
        return None;
    };
    if path.path.segments.len() != 1 {
        return None;
    }
    Some(&path.path.segments[0].ident)
}
