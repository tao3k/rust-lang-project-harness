use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RustTokioRuntimeOperationSyntax {
    pub(crate) function_name: String,
    pub(crate) function_line: usize,
    pub(crate) line: usize,
    pub(crate) operation: String,
    pub(crate) call_expr: String,
    pub(crate) is_test_context: bool,
}

pub(crate) fn tokio_runtime_operation_syntax(
    item: &syn::Item,
) -> Vec<RustTokioRuntimeOperationSyntax> {
    let is_test_context = item_attrs(item).is_some_and(attrs_have_test_context);
    let mut visitor = TokioRuntimeOperationVisitor {
        operations: Vec::new(),
        function_context: None,
        is_test_context,
    };
    visitor.visit_item(item);
    visitor.operations
}

struct TokioRuntimeOperationVisitor {
    operations: Vec<RustTokioRuntimeOperationSyntax>,
    function_context: Option<(String, usize)>,
    is_test_context: bool,
}

impl<'ast> Visit<'ast> for TokioRuntimeOperationVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let parent_context = self.function_context.replace((
            node.sig.ident.to_string(),
            node.sig.ident.span().start().line,
        ));
        let parent_test_context = self.is_test_context;
        self.is_test_context = self.is_test_context || attrs_have_test_context(&node.attrs);
        syn::visit::visit_item_fn(self, node);
        self.function_context = parent_context;
        self.is_test_context = parent_test_context;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let parent_context = self.function_context.replace((
            node.sig.ident.to_string(),
            node.sig.ident.span().start().line,
        ));
        let parent_test_context = self.is_test_context;
        self.is_test_context = self.is_test_context || attrs_have_test_context(&node.attrs);
        syn::visit::visit_impl_item_fn(self, node);
        self.function_context = parent_context;
        self.is_test_context = parent_test_context;
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Some((operation, call_expr)) = classify_call(&node.func) {
            self.record(node.span(), operation, call_expr);
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == "spawn"
            || node.method == "spawn_blocking"
            || node.method == "block_in_place"
        {
            let receiver = expr_to_string(&node.receiver);
            if is_tokio_runtime_receiver(&receiver) {
                self.record(
                    node.span(),
                    node.method.to_string(),
                    format!("{receiver}.{}", node.method),
                );
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

impl TokioRuntimeOperationVisitor {
    fn record(&mut self, span: Span, operation: String, call_expr: String) {
        let (function_name, function_line) = self
            .function_context
            .clone()
            .unwrap_or_else(|| ("<module>".to_string(), 1));
        self.operations.push(RustTokioRuntimeOperationSyntax {
            function_name,
            function_line,
            line: span.start().line,
            operation,
            call_expr,
            is_test_context: self.is_test_context,
        });
    }
}

fn classify_call(func: &syn::Expr) -> Option<(String, String)> {
    let syn::Expr::Path(path) = func else {
        return None;
    };
    let segments: Vec<String> = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let expr = segments.join("::");
    if expr == "tokio::spawn" || expr == "tokio::task::spawn" {
        return Some(("spawn".to_string(), expr));
    }
    if expr == "tokio::task::spawn_blocking" {
        return Some(("spawn_blocking".to_string(), expr));
    }
    if expr == "tokio::task::block_in_place" {
        return Some(("block_in_place".to_string(), expr));
    }
    if matches_path_segments(&segments, &["tokio", "runtime", "Runtime", "new"])
        || matches_path_segments(&segments, &["runtime", "Runtime", "new"])
        || matches_path_segments(&segments, &["Runtime", "new"])
    {
        return Some(("runtime_new".to_string(), expr));
    }
    if matches_path_segments(
        &segments,
        &["tokio", "runtime", "Builder", "new_multi_thread"],
    ) || matches_path_segments(&segments, &["runtime", "Builder", "new_multi_thread"])
        || matches_path_segments(&segments, &["Builder", "new_multi_thread"])
    {
        return Some(("new_multi_thread".to_string(), expr));
    }
    if matches_path_segments(
        &segments,
        &["tokio", "runtime", "Builder", "new_current_thread"],
    ) || matches_path_segments(&segments, &["runtime", "Builder", "new_current_thread"])
        || matches_path_segments(&segments, &["Builder", "new_current_thread"])
    {
        return Some(("new_current_thread".to_string(), expr));
    }
    None
}

fn matches_path_segments(segments: &[String], expected: &[&str]) -> bool {
    segments.len() == expected.len()
        && segments
            .iter()
            .zip(expected)
            .all(|(segment, expected)| segment == expected)
}

fn is_tokio_runtime_receiver(receiver: &str) -> bool {
    receiver.starts_with("tokio::")
        || receiver.contains("tokio::runtime::")
        || receiver.starts_with("Handle::current")
        || receiver.starts_with("Runtime::new")
        || receiver.starts_with("Builder::new_")
}

fn item_attrs(item: &syn::Item) -> Option<&[syn::Attribute]> {
    match item {
        syn::Item::Const(item) => Some(&item.attrs),
        syn::Item::Enum(item) => Some(&item.attrs),
        syn::Item::Fn(item) => Some(&item.attrs),
        syn::Item::Impl(item) => Some(&item.attrs),
        syn::Item::Mod(item) => Some(&item.attrs),
        syn::Item::Static(item) => Some(&item.attrs),
        syn::Item::Struct(item) => Some(&item.attrs),
        syn::Item::Trait(item) => Some(&item.attrs),
        syn::Item::Type(item) => Some(&item.attrs),
        _ => None,
    }
}

fn attrs_have_test_context(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let text = attr.to_token_stream().to_string();
        text.contains("test") || text.contains("cfg ( test )") || text.contains("tokio :: test")
    })
}

fn expr_to_string(expr: &syn::Expr) -> String {
    expr.to_token_stream().to_string().replace(' ', "")
}
