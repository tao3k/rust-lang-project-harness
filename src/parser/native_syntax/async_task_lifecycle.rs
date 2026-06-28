//! Async task lifecycle facts.

use syn::visit::{self, Visit};

pub(crate) fn discarded_tokio_spawn_count(block: &syn::Block) -> usize {
    let mut collector = AsyncTaskLifecycleCollector::default();
    collector.visit_block(block);
    collector.discarded_spawn_calls
}

#[derive(Default)]
struct AsyncTaskLifecycleCollector {
    discarded_spawn_calls: usize,
}

impl<'ast> Visit<'ast> for AsyncTaskLifecycleCollector {
    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        match statement {
            syn::Stmt::Expr(expr, Some(_)) if expr_is_tokio_spawn_call(expr) => {
                self.discarded_spawn_calls += 1;
            }
            syn::Stmt::Local(local) if local_discards_tokio_spawn(local) => {
                self.discarded_spawn_calls += 1;
            }
            _ => {}
        }
        visit::visit_stmt(self, statement);
    }
}

fn local_discards_tokio_spawn(local: &syn::Local) -> bool {
    if !matches!(&local.pat, syn::Pat::Wild(_)) {
        return false;
    }
    local
        .init
        .as_ref()
        .is_some_and(|init| expr_is_tokio_spawn_call(&init.expr))
}

fn expr_is_tokio_spawn_call(expr: &syn::Expr) -> bool {
    let syn::Expr::Call(call) = expr else {
        return false;
    };
    call_path_is_tokio_spawn(call.func.as_ref())
}

fn call_path_is_tokio_spawn(expr: &syn::Expr) -> bool {
    let syn::Expr::Path(path) = expr else {
        return false;
    };
    let segments = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    path_segments_are_tokio_spawn(&segments)
}

fn path_segments_are_tokio_spawn(segments: &[String]) -> bool {
    matches!(
        segments,
        [name] if is_spawn_name(name)
    ) || matches!(
        segments,
        [owner, name] if owner == "task" && is_spawn_name(name)
    ) || matches!(
        segments,
        [owner, name] if owner == "tokio" && is_spawn_name(name)
    ) || matches!(
        segments,
        [owner, module, name] if owner == "tokio" && module == "task" && is_spawn_name(name)
    )
}

fn is_spawn_name(name: &str) -> bool {
    matches!(name, "spawn" | "spawn_local")
}
