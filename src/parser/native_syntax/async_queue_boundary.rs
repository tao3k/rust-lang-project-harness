//! Async queue backpressure facts.

use syn::visit::{self, Visit};

pub(crate) fn unbounded_async_queue_call_count(block: &syn::Block) -> usize {
    let mut collector = AsyncQueueBoundaryCollector::default();
    collector.visit_block(block);
    collector.unbounded_queue_calls
}

pub(crate) fn backpressure_boundary_signal_count(function_name: &str, block: &syn::Block) -> usize {
    let mut collector = AsyncQueueBoundaryCollector::default();
    collector.visit_block(block);
    collector.backpressure_boundary_signals
        + usize::from(function_name_is_backpressure_boundary(function_name))
}

#[derive(Default)]
struct AsyncQueueBoundaryCollector {
    unbounded_queue_calls: usize,
    backpressure_boundary_signals: usize,
}

impl<'ast> Visit<'ast> for AsyncQueueBoundaryCollector {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if expr_path_ends_with(call.func.as_ref(), &["unbounded_channel"]) {
            self.unbounded_queue_calls += 1;
        }
        if expr_path_last_segment(call.func.as_ref())
            .as_deref()
            .is_some_and(is_backpressure_boundary_name)
        {
            self.backpressure_boundary_signals += 1;
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, method_call: &'ast syn::ExprMethodCall) {
        if is_backpressure_boundary_name(&method_call.method.to_string()) {
            self.backpressure_boundary_signals += 1;
        }
        visit::visit_expr_method_call(self, method_call);
    }
}

fn function_name_is_backpressure_boundary(name: &str) -> bool {
    matches!(
        name,
        "poll_ready" | "ready" | "try_send" | "can_send" | "is_ready"
    )
}

fn is_backpressure_boundary_name(name: &str) -> bool {
    matches!(
        name,
        "poll_ready"
            | "try_send"
            | "can_send"
            | "poll_want"
            | "reserve"
            | "try_reserve"
            | "acquire"
            | "try_acquire"
            | "poll_acquire"
    )
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

fn expr_path_last_segment(expr: &syn::Expr) -> Option<String> {
    let syn::Expr::Path(path) = expr else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}
