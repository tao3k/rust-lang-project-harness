//! Parser-owned item projection facts for compact source views.

use quote::ToTokens;
use syn::spanned::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustItemProjectionNodeSyntax {
    pub line: usize,
    pub end_line: usize,
    pub kind: &'static str,
    pub role: &'static str,
    pub label: String,
    pub depth: usize,
}

pub(super) fn item_projection_nodes(item: &syn::Item) -> Vec<RustItemProjectionNodeSyntax> {
    let mut nodes = vec![RustItemProjectionNodeSyntax {
        line: item.span().start().line.max(1),
        end_line: item.span().end().line.max(item.span().start().line.max(1)),
        kind: item_kind(item),
        role: "declaration",
        label: item_projection_header(item),
        depth: 0,
    }];
    if let Some(block) = item_projection_block(item) {
        append_block_projection_nodes(&mut nodes, block, 1);
    }
    nodes
        .into_iter()
        .filter(|node| !node.label.trim().is_empty())
        .take(64)
        .collect()
}

fn item_projection_header(item: &syn::Item) -> String {
    match item {
        syn::Item::Fn(item_fn) => {
            format!(
                "{}{}",
                visibility_prefix(&item_fn.vis),
                compact_tokens(&item_fn.sig)
            )
        }
        syn::Item::Const(item) => format!("const {}", item.ident),
        syn::Item::Enum(item) => format!("{}enum {}", visibility_prefix(&item.vis), item.ident),
        syn::Item::ExternCrate(item) => format!("extern crate {}", item.ident),
        syn::Item::Macro(item) => format!("macro {}", compact_tokens(&item.mac.path)),
        syn::Item::Mod(item) => format!("mod {}", item.ident),
        syn::Item::Static(item) => format!("static {}", item.ident),
        syn::Item::Struct(item) => {
            format!("{}struct {}", visibility_prefix(&item.vis), item.ident)
        }
        syn::Item::Trait(item) => format!("{}trait {}", visibility_prefix(&item.vis), item.ident),
        syn::Item::TraitAlias(item) => {
            format!("{}trait {}", visibility_prefix(&item.vis), item.ident)
        }
        syn::Item::Type(item) => format!("{}type {}", visibility_prefix(&item.vis), item.ident),
        syn::Item::Union(item) => format!("{}union {}", visibility_prefix(&item.vis), item.ident),
        _ => compact_tokens(item),
    }
}

fn item_projection_block(item: &syn::Item) -> Option<&syn::Block> {
    match item {
        syn::Item::Fn(item_fn) => Some(&item_fn.block),
        _ => None,
    }
}

fn append_block_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    block: &syn::Block,
    depth: usize,
) {
    for statement in &block.stmts {
        append_statement_projection_nodes(nodes, statement, depth);
    }
}

fn append_statement_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    statement: &syn::Stmt,
    depth: usize,
) {
    match statement {
        syn::Stmt::Local(local) => {
            let low_value_local = is_low_value_local(local);
            if !low_value_local {
                push_projection_node(
                    nodes,
                    local,
                    "let",
                    "mutation",
                    local_projection_label(local),
                    depth,
                );
            }
            if let Some(init) = &local.init {
                if low_value_local || local_initializer_needs_expansion(&init.expr) {
                    append_expression_projection_nodes(nodes, &init.expr, depth);
                }
            }
        }
        syn::Stmt::Item(item) => push_projection_node(
            nodes,
            item,
            "item",
            "declaration",
            item_projection_header(item),
            depth,
        ),
        syn::Stmt::Expr(expr, _) => append_tail_expression_projection_nodes(nodes, expr, depth),
        syn::Stmt::Macro(mac) => push_projection_node(
            nodes,
            &mac.mac,
            "macro",
            "call",
            macro_call_projection_label(&mac.mac),
            depth,
        ),
    }
}

fn append_expression_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    expression: &syn::Expr,
    depth: usize,
) {
    match expression {
        syn::Expr::Return(return_expr) => {
            push_projection_node(
                nodes,
                return_expr,
                "return",
                "terminal",
                return_projection_label(return_expr),
                depth,
            );
        }
        syn::Expr::Break(break_expr) => {
            push_projection_node(
                nodes,
                break_expr,
                "break",
                "terminal",
                break_projection_label(break_expr),
                depth,
            );
        }
        syn::Expr::Continue(continue_expr) => {
            push_projection_node(
                nodes,
                continue_expr,
                "continue",
                "terminal",
                "continue",
                depth,
            );
        }
        syn::Expr::If(if_expr) => {
            push_projection_node(
                nodes,
                if_expr,
                "if",
                "control-flow",
                format!("if {}", compact_tokens(&if_expr.cond)),
                depth,
            );
            append_block_projection_nodes(nodes, &if_expr.then_branch, depth + 1);
            if let Some((_, else_expr)) = &if_expr.else_branch {
                push_projection_node(nodes, else_expr, "else", "control-flow", "else", depth);
                append_expression_projection_nodes(nodes, else_expr, depth + 1);
            }
        }
        syn::Expr::Match(match_expr) => {
            push_projection_node(
                nodes,
                match_expr,
                "match",
                "control-flow",
                format!("match {}", compact_tokens(&match_expr.expr)),
                depth,
            );
            for arm in &match_expr.arms {
                push_projection_node(
                    nodes,
                    &arm.pat,
                    "case",
                    "control-flow",
                    format!("case {}", compact_tokens(&arm.pat)),
                    depth + 1,
                );
                append_expression_projection_nodes(nodes, &arm.body, depth + 2);
            }
        }
        syn::Expr::ForLoop(for_loop) => {
            push_projection_node(
                nodes,
                for_loop,
                "for",
                "control-flow",
                format!(
                    "for {} in {}",
                    compact_tokens(&for_loop.pat),
                    compact_tokens(&for_loop.expr)
                ),
                depth,
            );
            append_block_projection_nodes(nodes, &for_loop.body, depth + 1);
        }
        syn::Expr::While(while_expr) => {
            push_projection_node(
                nodes,
                while_expr,
                "while",
                "control-flow",
                format!("while {}", compact_tokens(&while_expr.cond)),
                depth,
            );
            append_block_projection_nodes(nodes, &while_expr.body, depth + 1);
        }
        syn::Expr::Loop(loop_expr) => {
            push_projection_node(nodes, loop_expr, "loop", "control-flow", "loop", depth);
            append_block_projection_nodes(nodes, &loop_expr.body, depth + 1);
        }
        syn::Expr::Call(call) => push_projection_node(
            nodes,
            call,
            "call",
            "call",
            format!("call {}", callable_projection_name(&call.func)),
            depth,
        ),
        syn::Expr::MethodCall(method_call) => push_projection_node(
            nodes,
            method_call,
            "call",
            "call",
            format!("call {}", method_call.method),
            depth,
        ),
        syn::Expr::Macro(macro_expr) => push_projection_node(
            nodes,
            &macro_expr.mac,
            "macro",
            "call",
            macro_call_projection_label(&macro_expr.mac),
            depth,
        ),
        syn::Expr::Assign(assign) => {
            push_projection_node(
                nodes,
                assign,
                "assign",
                "mutation",
                format!("assign {}", compact_tokens(&assign.left)),
                depth,
            );
            append_expression_projection_nodes(nodes, &assign.right, depth + 1);
        }
        syn::Expr::Await(await_expr) => {
            push_projection_node(
                nodes,
                await_expr,
                "await",
                "effect",
                format!("await {}", compact_tokens(&await_expr.base)),
                depth,
            );
            append_expression_projection_nodes(nodes, &await_expr.base, depth + 1);
        }
        syn::Expr::Try(try_expr) => {
            push_projection_node(
                nodes,
                try_expr,
                "try",
                "effect",
                format!("try {}", compact_tokens(&try_expr.expr)),
                depth,
            );
            append_expression_projection_nodes(nodes, &try_expr.expr, depth + 1);
        }
        syn::Expr::Block(block) => append_block_projection_nodes(nodes, &block.block, depth),
        syn::Expr::Closure(closure) => {
            append_expression_projection_nodes(nodes, &closure.body, depth)
        }
        syn::Expr::Binary(binary) => {
            if let Some(label) = binary_assignment_projection_label(binary) {
                push_projection_node(nodes, binary, "assign", "mutation", label, depth);
                append_expression_projection_nodes(nodes, &binary.right, depth + 1);
            } else {
                append_expression_projection_nodes(nodes, &binary.left, depth);
                append_expression_projection_nodes(nodes, &binary.right, depth);
            }
        }
        syn::Expr::Unary(unary) => append_expression_projection_nodes(nodes, &unary.expr, depth),
        syn::Expr::Reference(reference) => {
            append_expression_projection_nodes(nodes, &reference.expr, depth);
        }
        syn::Expr::Paren(paren) => append_expression_projection_nodes(nodes, &paren.expr, depth),
        syn::Expr::Cast(cast) => append_expression_projection_nodes(nodes, &cast.expr, depth),
        syn::Expr::Index(index) => {
            append_expression_projection_nodes(nodes, &index.expr, depth);
            append_expression_projection_nodes(nodes, &index.index, depth);
        }
        syn::Expr::Field(field) => append_expression_projection_nodes(nodes, &field.base, depth),
        syn::Expr::Tuple(tuple) => {
            for element in &tuple.elems {
                append_expression_projection_nodes(nodes, element, depth);
            }
        }
        syn::Expr::Array(array) => {
            for element in &array.elems {
                append_expression_projection_nodes(nodes, element, depth);
            }
        }
        _ => {}
    }
}

fn append_tail_expression_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    expression: &syn::Expr,
    depth: usize,
) {
    match expression {
        syn::Expr::Path(_) | syn::Expr::Lit(_) | syn::Expr::Struct(_) | syn::Expr::Tuple(_) => {
            push_projection_node(
                nodes,
                expression,
                "tail",
                "terminal",
                format!("tail {}", expression_projection_summary(expression)),
                depth,
            );
        }
        _ => append_expression_projection_nodes(nodes, expression, depth),
    }
}

fn push_projection_node(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    syntax: &impl Spanned,
    kind: &'static str,
    role: &'static str,
    label: impl Into<String>,
    depth: usize,
) {
    let line = syntax.span().start().line.max(1);
    nodes.push(RustItemProjectionNodeSyntax {
        line,
        end_line: syntax.span().end().line.max(line),
        kind,
        role,
        label: label.into(),
        depth,
    });
}

fn is_low_value_local(local: &syn::Local) -> bool {
    matches!(&local.pat, syn::Pat::Wild(_))
}

fn local_initializer_needs_expansion(expression: &syn::Expr) -> bool {
    matches!(
        expression,
        syn::Expr::If(_)
            | syn::Expr::Match(_)
            | syn::Expr::ForLoop(_)
            | syn::Expr::While(_)
            | syn::Expr::Loop(_)
            | syn::Expr::Block(_)
            | syn::Expr::Closure(_)
    )
}

fn local_projection_label(local: &syn::Local) -> String {
    match &local.init {
        Some(init) => format!(
            "let {} = {}",
            compact_tokens(&local.pat),
            expression_projection_summary(&init.expr)
        ),
        None => format!("let {}", compact_tokens(&local.pat)),
    }
}

fn expression_projection_summary(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Call(call) => callable_projection_name(&call.func),
        syn::Expr::MethodCall(method_call) => method_call.method.to_string(),
        syn::Expr::Macro(macro_expr) => format!("{}!", compact_tokens(&macro_expr.mac.path)),
        syn::Expr::Struct(struct_expr) => compact_tokens(&struct_expr.path),
        syn::Expr::Array(_) => "array".to_string(),
        syn::Expr::Tuple(_) => "tuple".to_string(),
        syn::Expr::Try(try_expr) => {
            format!("{}?", expression_projection_summary(&try_expr.expr))
        }
        syn::Expr::Await(await_expr) => {
            format!("{}.await", expression_projection_summary(&await_expr.base))
        }
        syn::Expr::Reference(reference) => expression_projection_summary(&reference.expr),
        syn::Expr::Paren(paren) => expression_projection_summary(&paren.expr),
        syn::Expr::Cast(cast) => expression_projection_summary(&cast.expr),
        _ => compact_tokens(expression),
    }
}

fn return_projection_label(return_expr: &syn::ExprReturn) -> String {
    return_expr.expr.as_deref().map_or_else(
        || "return".to_string(),
        |expr| format!("return {}", compact_tokens(expr)),
    )
}

fn break_projection_label(break_expr: &syn::ExprBreak) -> String {
    break_expr.expr.as_deref().map_or_else(
        || "break".to_string(),
        |expr| format!("break {}", compact_tokens(expr)),
    )
}

fn binary_assignment_projection_label(binary: &syn::ExprBinary) -> Option<String> {
    let operator = compact_tokens(&binary.op);
    matches!(
        operator.as_str(),
        "+=" | "-=" | "*=" | "/=" | "%=" | "^=" | "&=" | "|=" | "<<=" | ">>="
    )
    .then(|| {
        format!(
            "assign {} {} {}",
            compact_tokens(&binary.left),
            operator,
            expression_projection_summary(&binary.right)
        )
    })
}

fn callable_projection_name(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Path(path) => compact_tokens(&path.path),
        syn::Expr::MethodCall(method_call) => method_call.method.to_string(),
        _ => compact_tokens(expression),
    }
}

fn macro_call_projection_label(mac: &syn::Macro) -> String {
    let arguments = compact_limited(&compact_tokens(&mac.tokens), 96);
    if arguments.is_empty() {
        format!("call {}!", compact_tokens(&mac.path))
    } else {
        format!("call {}!({})", compact_tokens(&mac.path), arguments)
    }
}

fn visibility_prefix(visibility: &syn::Visibility) -> &'static str {
    if is_public_visibility(visibility) {
        "pub "
    } else {
        ""
    }
}

fn compact_tokens(value: &impl ToTokens) -> String {
    compact_rust_tokens(&value.to_token_stream().to_string())
}

fn compact_limited(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    let mut truncated = value
        .chars()
        .take(max_len.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn compact_rust_tokens(value: &str) -> String {
    let mut compacted = value.split_whitespace().collect::<Vec<_>>().join(" ");
    for (from, to) in [
        (" :: ", "::"),
        (" (", "("),
        ("( ", "("),
        (" )", ")"),
        (" [", "["),
        ("[ ", "["),
        (" ]", "]"),
        (" ,", ","),
        (",)", ")"),
        (" ;", ";"),
        (" :", ":"),
        (" & ", "&"),
        ("& ", "&"),
        (" * ", " *"),
        (" !", "!"),
        (" . ", "."),
        (" <", "<"),
        ("< ", "<"),
        (" >", ">"),
        (" < ", "<"),
        (" > ", ">"),
        (":&", ": &"),
    ] {
        compacted = compacted.replace(from, to);
    }
    compacted
}

fn item_kind(item: &syn::Item) -> &'static str {
    match item {
        syn::Item::Const(_) => "const",
        syn::Item::Enum(_) => "enum",
        syn::Item::Fn(_) => "fn",
        syn::Item::Impl(_) => "impl",
        syn::Item::Macro(_) => "macro",
        syn::Item::Mod(_) => "mod",
        syn::Item::Static(_) => "static",
        syn::Item::Struct(_) => "struct",
        syn::Item::Trait(_) => "trait",
        syn::Item::TraitAlias(_) => "trait_alias",
        syn::Item::Type(_) => "type",
        syn::Item::Union(_) => "union",
        syn::Item::Use(_) => "use",
        _ => "item",
    }
}

fn is_public_visibility(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}
