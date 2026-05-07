//! Native Rust function control-flow facts.

use std::collections::BTreeMap;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use quote::ToTokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionControlFlowSyntax {
    pub line: usize,
    pub function_name: String,
    pub line_span: usize,
    pub statement_count: usize,
    pub max_block_statement_count: usize,
    pub branch_count: usize,
    pub loop_count: usize,
    pub max_nesting_depth: usize,
    pub max_loop_nesting_depth: usize,
    pub match_count: usize,
    pub literal_dispatch_chain_count: usize,
    pub manual_collection_loop_count: usize,
    pub manual_predicate_loop_count: usize,
    pub manual_numeric_accumulator_loop_count: usize,
    pub manual_count_loop_count: usize,
    pub repeated_iterator_source_loop_count: usize,
    pub is_test_context: bool,
}

pub(crate) fn public_function_control_flow_syntax(
    item: &syn::Item,
) -> Option<RustFunctionControlFlowSyntax> {
    let syn::Item::Fn(item_fn) = item else {
        return None;
    };
    if !is_public_visibility(&item_fn.vis) {
        return None;
    }

    let mut collector = FunctionControlFlowCollector {
        facts: RustFunctionControlFlowSyntax {
            line: item_fn.sig.ident.span().start().line.max(1),
            function_name: item_fn.sig.ident.to_string(),
            line_span: item_line_span(item_fn),
            statement_count: 0,
            max_block_statement_count: 0,
            branch_count: 0,
            loop_count: 0,
            max_nesting_depth: 0,
            max_loop_nesting_depth: 0,
            match_count: 0,
            literal_dispatch_chain_count: literal_dispatch_chain_count(item_fn),
            manual_collection_loop_count: 0,
            manual_predicate_loop_count: 0,
            manual_numeric_accumulator_loop_count: 0,
            manual_count_loop_count: 0,
            repeated_iterator_source_loop_count: repeated_iterator_source_loop_count(item_fn),
            is_test_context: attrs_have_cfg_test(&item_fn.attrs),
        },
        control_depth: 0,
        loop_depth: 0,
    };
    collector.visit_block(&item_fn.block);
    Some(collector.facts)
}

struct FunctionControlFlowCollector {
    facts: RustFunctionControlFlowSyntax,
    control_depth: usize,
    loop_depth: usize,
}

impl<'ast> Visit<'ast> for FunctionControlFlowCollector {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.facts.max_block_statement_count =
            self.facts.max_block_statement_count.max(block.stmts.len());
        visit::visit_block(self, block);
    }

    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        self.facts.statement_count += 1;
        visit::visit_stmt(self, statement);
    }

    fn visit_expr_if(&mut self, expr_if: &'ast syn::ExprIf) {
        self.facts.branch_count += 1;
        self.enter_control_scope(|collector| visit::visit_expr_if(collector, expr_if));
    }

    fn visit_expr_match(&mut self, expr_match: &'ast syn::ExprMatch) {
        self.facts.match_count += 1;
        self.facts.branch_count += expr_match.arms.len();
        self.enter_control_scope(|collector| visit::visit_expr_match(collector, expr_match));
    }

    fn visit_expr_for_loop(&mut self, expr_for_loop: &'ast syn::ExprForLoop) {
        self.record_manual_loop_signals(&expr_for_loop.body);
        self.enter_loop_scope(|collector| visit::visit_expr_for_loop(collector, expr_for_loop));
    }

    fn visit_expr_loop(&mut self, expr_loop: &'ast syn::ExprLoop) {
        self.enter_loop_scope(|collector| visit::visit_expr_loop(collector, expr_loop));
    }

    fn visit_expr_while(&mut self, expr_while: &'ast syn::ExprWhile) {
        self.enter_loop_scope(|collector| visit::visit_expr_while(collector, expr_while));
    }
}

impl FunctionControlFlowCollector {
    fn enter_control_scope(&mut self, visit: impl FnOnce(&mut Self)) {
        self.control_depth += 1;
        self.facts.max_nesting_depth = self.facts.max_nesting_depth.max(self.control_depth);
        visit(self);
        self.control_depth -= 1;
    }

    fn enter_loop_scope(&mut self, visit: impl FnOnce(&mut Self)) {
        self.facts.loop_count += 1;
        self.loop_depth += 1;
        self.facts.max_loop_nesting_depth = self.facts.max_loop_nesting_depth.max(self.loop_depth);
        self.enter_control_scope(visit);
        self.loop_depth -= 1;
    }

    fn record_manual_loop_signals(&mut self, block: &syn::Block) {
        let signals = manual_loop_body_signals(block);
        if signals.collection_accumulator {
            self.facts.manual_collection_loop_count += 1;
        }
        if signals.predicate_return {
            self.facts.manual_predicate_loop_count += 1;
        }
        if signals.numeric_accumulator {
            self.facts.manual_numeric_accumulator_loop_count += 1;
        }
        if signals.count_accumulator {
            self.facts.manual_count_loop_count += 1;
        }
    }
}

#[derive(Default)]
struct ManualLoopBodySignals {
    collection_accumulator: bool,
    predicate_return: bool,
    numeric_accumulator: bool,
    count_accumulator: bool,
}

fn manual_loop_body_signals(block: &syn::Block) -> ManualLoopBodySignals {
    let mut collector = ManualLoopBodySignalCollector::default();
    for statement in &block.stmts {
        collector.visit_stmt(statement);
    }
    collector.signals
}

#[derive(Default)]
struct ManualLoopBodySignalCollector {
    signals: ManualLoopBodySignals,
}

impl<'ast> Visit<'ast> for ManualLoopBodySignalCollector {
    fn visit_expr_method_call(&mut self, method_call: &'ast syn::ExprMethodCall) {
        if method_call_receiver_ident(method_call).is_some()
            && matches!(method_call.method.to_string().as_str(), "push" | "insert")
        {
            self.signals.collection_accumulator = true;
        }
        visit::visit_expr_method_call(self, method_call);
    }

    fn visit_expr_binary(&mut self, binary: &'ast syn::ExprBinary) {
        if matches!(binary.op, syn::BinOp::AddAssign(_)) && expr_path_ident(&binary.left).is_some()
        {
            if expr_is_integer_one(&binary.right) {
                self.signals.count_accumulator = true;
            } else {
                self.signals.numeric_accumulator = true;
            }
        }
        visit::visit_expr_binary(self, binary);
    }

    fn visit_expr_return(&mut self, return_expr: &'ast syn::ExprReturn) {
        if return_expr
            .expr
            .as_deref()
            .is_some_and(expr_is_bool_literal)
        {
            self.signals.predicate_return = true;
        }
        visit::visit_expr_return(self, return_expr);
    }

    fn visit_expr_for_loop(&mut self, _loop: &'ast syn::ExprForLoop) {}

    fn visit_expr_loop(&mut self, _loop: &'ast syn::ExprLoop) {}

    fn visit_expr_while(&mut self, _loop: &'ast syn::ExprWhile) {}
}

fn method_call_receiver_ident(method_call: &syn::ExprMethodCall) -> Option<&syn::Ident> {
    expr_path_ident(&method_call.receiver)
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

fn expr_is_integer_one(expr: &syn::Expr) -> bool {
    let syn::Expr::Lit(literal) = expr else {
        return false;
    };
    let syn::Lit::Int(int) = &literal.lit else {
        return false;
    };
    int.base10_digits() == "1"
}

fn expr_is_bool_literal(expr: &syn::Expr) -> bool {
    let syn::Expr::Lit(literal) = expr else {
        return false;
    };
    matches!(literal.lit, syn::Lit::Bool(_))
}

fn repeated_iterator_source_loop_count(item_fn: &syn::ItemFn) -> usize {
    let mut collector = ForLoopIteratorCollector::default();
    collector.visit_block(&item_fn.block);
    collector.repeated_loop_count()
}

#[derive(Default)]
struct ForLoopIteratorCollector {
    iterator_counts: BTreeMap<String, usize>,
}

impl<'ast> Visit<'ast> for ForLoopIteratorCollector {
    fn visit_expr_for_loop(&mut self, loop_expr: &'ast syn::ExprForLoop) {
        if let Some(source) = simple_iterator_source(&loop_expr.expr) {
            *self.iterator_counts.entry(source).or_default() += 1;
        }
        visit::visit_expr_for_loop(self, loop_expr);
    }
}

impl ForLoopIteratorCollector {
    fn repeated_loop_count(&self) -> usize {
        self.iterator_counts
            .values()
            .filter(|count| **count > 1)
            .map(|count| count - 1)
            .sum()
    }
}

fn simple_iterator_source(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) if path.path.segments.len() == 1 => {
            Some(path.path.segments[0].ident.to_string())
        }
        syn::Expr::MethodCall(method_call)
            if matches!(
                method_call.method.to_string().as_str(),
                "iter" | "iter_mut" | "into_iter"
            ) =>
        {
            expr_path_ident(&method_call.receiver).map(ToString::to_string)
        }
        _ => None,
    }
}

fn literal_dispatch_chain_count(item_fn: &syn::ItemFn) -> usize {
    let mut collector = LiteralDispatchChainCollector::default();
    collector.visit_block(&item_fn.block);
    collector.chain_count
}

#[derive(Default)]
struct LiteralDispatchChainCollector {
    chain_count: usize,
}

impl<'ast> Visit<'ast> for LiteralDispatchChainCollector {
    fn visit_expr_if(&mut self, expr_if: &'ast syn::ExprIf) {
        if literal_dispatch_chain_subject(expr_if).is_some() {
            self.chain_count += 1;
        }
        self.visit_if_chain_bodies(expr_if);
    }
}

impl LiteralDispatchChainCollector {
    fn visit_if_chain_bodies(&mut self, expr_if: &syn::ExprIf) {
        self.visit_block(&expr_if.then_branch);
        if let Some((_, else_branch)) = &expr_if.else_branch {
            match else_branch.as_ref() {
                syn::Expr::If(next_if) => self.visit_if_chain_bodies(next_if),
                other => self.visit_expr(other),
            }
        }
    }
}

fn literal_dispatch_chain_subject(expr_if: &syn::ExprIf) -> Option<String> {
    const MIN_DISPATCH_ARMS: usize = 3;

    let mut subject_counts = BTreeMap::<String, usize>::new();
    let mut current = Some(expr_if);
    while let Some(branch) = current {
        if let Some(subject) = literal_condition_subject(&branch.cond) {
            *subject_counts.entry(subject).or_default() += 1;
        }
        current =
            branch
                .else_branch
                .as_ref()
                .and_then(|(_, else_branch)| match else_branch.as_ref() {
                    syn::Expr::If(next_if) => Some(next_if),
                    _ => None,
                });
    }
    subject_counts
        .into_iter()
        .find_map(|(subject, count)| (count >= MIN_DISPATCH_ARMS).then_some(subject))
}

fn literal_condition_subject(expr: &syn::Expr) -> Option<String> {
    let syn::Expr::Binary(binary) = expr else {
        return None;
    };
    if !matches!(binary.op, syn::BinOp::Eq(_)) {
        return None;
    }
    match (
        condition_subject_name(&binary.left),
        expr_is_literal(&binary.right),
        condition_subject_name(&binary.right),
        expr_is_literal(&binary.left),
    ) {
        (Some(subject), true, _, _) | (_, _, Some(subject), true) => Some(subject),
        _ => None,
    }
}

fn condition_subject_name(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) if path.path.segments.len() == 1 => {
            Some(path.path.segments[0].ident.to_string())
        }
        syn::Expr::MethodCall(method_call)
            if matches!(method_call.method.to_string().as_str(), "as_str" | "as_ref") =>
        {
            expr_path_ident(&method_call.receiver).map(ToString::to_string)
        }
        _ => None,
    }
}

fn expr_is_literal(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Lit(_))
}

fn item_line_span(item_fn: &syn::ItemFn) -> usize {
    let span = item_fn.span();
    let start = span.start().line.max(1);
    let end = span.end().line.max(start);
    end - start + 1
}

fn attrs_have_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attribute_has_cfg_test)
}

fn attribute_has_cfg_test(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("cfg") && attr.to_token_stream().to_string().contains("test")
}

fn is_public_visibility(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}
