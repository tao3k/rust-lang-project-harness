//! Tokio timeout cancellation-safety syntax facts.

use syn::visit::{self, Visit};

use super::select_cancellation_safety::is_cancel_unsafe_io_name;

pub(crate) fn tokio_timeout_cancel_unsafe_io_count(block: &syn::Block) -> usize {
    let mut collector = TimeoutCancellationSafetyCollector::default();
    collector.visit_block(block);
    collector.cancel_unsafe_io_calls
}

#[derive(Default)]
struct TimeoutCancellationSafetyCollector {
    cancel_unsafe_io_calls: usize,
}

impl<'ast> Visit<'ast> for TimeoutCancellationSafetyCollector {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if expr_path_is_tokio_timeout(call.func.as_ref()) {
            self.cancel_unsafe_io_calls += call
                .args
                .iter()
                .skip(1)
                .map(cancel_unsafe_io_expr_count)
                .sum::<usize>();
        }

        visit::visit_expr_call(self, call);
    }
}

fn expr_path_is_tokio_timeout(expr: &syn::Expr) -> bool {
    let syn::Expr::Path(path) = expr else {
        return false;
    };
    path_is_tokio_timeout(&path.path)
}

fn path_is_tokio_timeout(path: &syn::Path) -> bool {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();

    match segments.as_slice() {
        [name] => matches!(name.as_str(), "timeout" | "timeout_at"),
        [tokio, time, name] => {
            tokio.as_str() == "tokio"
                && time.as_str() == "time"
                && matches!(name.as_str(), "timeout" | "timeout_at")
        }
        _ => false,
    }
}

fn cancel_unsafe_io_expr_count(expr: &syn::Expr) -> usize {
    let mut collector = CancelUnsafeIoExprCollector::default();
    collector.visit_expr(expr);
    collector.count
}

#[derive(Default)]
struct CancelUnsafeIoExprCollector {
    count: usize,
}

impl<'ast> Visit<'ast> for CancelUnsafeIoExprCollector {
    fn visit_expr_method_call(&mut self, method_call: &'ast syn::ExprMethodCall) {
        if is_cancel_unsafe_io_name(&method_call.method.to_string()) {
            self.count += 1;
        }

        visit::visit_expr_method_call(self, method_call);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if expr_call_name_is_cancel_unsafe_io(call) {
            self.count += 1;
        }

        visit::visit_expr_call(self, call);
    }
}

fn expr_call_name_is_cancel_unsafe_io(call: &syn::ExprCall) -> bool {
    let syn::Expr::Path(path) = call.func.as_ref() else {
        return false;
    };

    path.path
        .segments
        .last()
        .map(|segment| is_cancel_unsafe_io_name(&segment.ident.to_string()))
        .unwrap_or(false)
}
