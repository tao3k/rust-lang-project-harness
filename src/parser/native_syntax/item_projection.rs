//! Parser-owned item projection facts for compact source views.

use quote::ToTokens;
use syn::parse::Parser;
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
    fn append_child_nodes(
        nodes: &mut Vec<RustItemProjectionNodeSyntax>,
        item: &syn::Item,
        depth: usize,
    ) {
        match item {
            syn::Item::Fn(item_fn) => {
                append_block_projection_nodes(nodes, &item_fn.block, depth);
            }
            syn::Item::Impl(item_impl) => {
                append_impl_projection_nodes(nodes, item_impl, depth);
            }
            syn::Item::Struct(item_struct) => {
                append_struct_field_projection_nodes(nodes, &item_struct.fields, depth);
            }
            syn::Item::Mod(item_mod) => {
                append_module_nodes(nodes, item_mod, depth);
            }
            _ => {}
        }
    }

    fn append_module_nodes(
        nodes: &mut Vec<RustItemProjectionNodeSyntax>,
        item_mod: &syn::ItemMod,
        depth: usize,
    ) {
        if let Some((_, items)) = &item_mod.content {
            for item in items {
                push_projection_node(
                    nodes,
                    item,
                    item_kind(item),
                    "declaration",
                    item_projection_header(item),
                    depth,
                );
                append_child_nodes(nodes, item, depth + 1);
            }
        }
    }

    let mut nodes = vec![RustItemProjectionNodeSyntax {
        line: item.span().start().line.max(1),
        end_line: item.span().end().line.max(item.span().start().line.max(1)),
        kind: item_kind(item),
        role: "declaration",
        label: item_projection_header(item),
        depth: 0,
    }];
    append_child_nodes(&mut nodes, item, 1);
    nodes
        .into_iter()
        .filter(|node| !node.label.trim().is_empty())
        .take(64)
        .collect()
}

fn item_projection_header(item: &syn::Item) -> String {
    match item {
        syn::Item::Fn(item_fn) => fn_signature_projection_label(&item_fn.sig),
        syn::Item::Const(item) => format!("const {}", item.ident),
        syn::Item::Enum(item) => format!("{}enum {} {{", visibility_text(&item.vis), item.ident),
        syn::Item::ExternCrate(item) => format!("extern crate {};", item.ident),
        syn::Item::Macro(item) => macro_call_projection_label(&item.mac),
        syn::Item::Mod(item) => format!("mod {} {{", item.ident),
        syn::Item::Static(item) => format!("static {}", item.ident),
        syn::Item::Struct(item) => {
            format!("{}struct {} {{", visibility_text(&item.vis), item.ident)
        }
        syn::Item::Impl(item) => impl_projection_header(item),
        syn::Item::Trait(item) => format!("{}trait {} {{", visibility_text(&item.vis), item.ident),
        syn::Item::TraitAlias(item) => {
            format!("{}trait {}", visibility_text(&item.vis), item.ident)
        }
        syn::Item::Type(item) => format!("type {};", item.ident),
        syn::Item::Union(item) => format!("{}union {} {{", visibility_text(&item.vis), item.ident),
        _ => compact_tokens(item),
    }
}

fn impl_projection_header(item: &syn::ItemImpl) -> String {
    if let Some((_, trait_path, _)) = &item.trait_ {
        format!(
            "impl {} for {} {{",
            compact_tokens(trait_path),
            compact_tokens(&item.self_ty)
        )
    } else {
        format!("impl {} {{", compact_tokens(&item.self_ty))
    }
}

fn fn_signature_projection_label(signature: &syn::Signature) -> String {
    format!("{} {{", compact_tokens(signature))
}

fn struct_field_projection_label(field: &syn::Field, index: usize) -> String {
    let name = field
        .ident
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("#{index}"));
    format!(
        "{}{}: {},",
        visibility_text(&field.vis),
        name,
        compact_tokens(&field.ty)
    )
}

fn append_struct_field_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    fields: &syn::Fields,
    depth: usize,
) {
    match fields {
        syn::Fields::Named(fields) => {
            for (index, field) in fields.named.iter().enumerate() {
                push_projection_node(
                    nodes,
                    field,
                    "field",
                    "field",
                    struct_field_projection_label(field, index),
                    depth,
                );
            }
        }
        syn::Fields::Unnamed(fields) => {
            for (index, field) in fields.unnamed.iter().enumerate() {
                push_projection_node(
                    nodes,
                    field,
                    "field",
                    "field",
                    struct_field_projection_label(field, index),
                    depth,
                );
            }
        }
        syn::Fields::Unit => {}
    }
}

fn append_impl_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    item_impl: &syn::ItemImpl,
    depth: usize,
) {
    for impl_item in &item_impl.items {
        match impl_item {
            syn::ImplItem::Fn(method) => {
                push_projection_node(
                    nodes,
                    method,
                    "fn",
                    "declaration",
                    fn_signature_projection_label(&method.sig),
                    depth,
                );
                append_block_projection_nodes(nodes, &method.block, depth + 1);
            }
            syn::ImplItem::Const(item_const) => push_projection_node(
                nodes,
                item_const,
                "const",
                "declaration",
                format!("const {}", item_const.ident),
                depth,
            ),
            syn::ImplItem::Type(item_type) => push_projection_node(
                nodes,
                item_type,
                "type",
                "declaration",
                format!("type {};", item_type.ident),
                depth,
            ),
            _ => {}
        }
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
                "continue;",
                depth,
            );
        }
        syn::Expr::If(if_expr) => {
            push_projection_node(
                nodes,
                if_expr,
                "if",
                "control-flow",
                format!("if {} {{", condition_projection_summary(&if_expr.cond)),
                depth,
            );
            append_block_projection_nodes(nodes, &if_expr.then_branch, depth + 1);
            if let Some((_, else_expr)) = &if_expr.else_branch {
                push_projection_node(nodes, else_expr, "else", "control-flow", "else {", depth);
                append_expression_projection_nodes(nodes, else_expr, depth + 1);
            }
        }
        syn::Expr::Match(match_expr) => {
            push_projection_node(
                nodes,
                match_expr,
                "match",
                "control-flow",
                format!(
                    "match {} {{",
                    condition_projection_summary(&match_expr.expr)
                ),
                depth,
            );
            for arm in &match_expr.arms {
                push_projection_node(
                    nodes,
                    &arm.pat,
                    "case",
                    "control-flow",
                    format!("{} => {{", compact_tokens(&arm.pat)),
                    depth + 1,
                );
                append_tail_expression_projection_nodes(nodes, &arm.body, depth + 2);
            }
        }
        syn::Expr::ForLoop(for_loop) => {
            push_projection_node(
                nodes,
                for_loop,
                "for",
                "control-flow",
                format!(
                    "for {} in {} {{",
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
                format!(
                    "while {} {{",
                    condition_projection_summary(&while_expr.cond)
                ),
                depth,
            );
            append_block_projection_nodes(nodes, &while_expr.body, depth + 1);
        }
        syn::Expr::Loop(loop_expr) => {
            push_projection_node(nodes, loop_expr, "loop", "control-flow", "loop {", depth);
            append_block_projection_nodes(nodes, &loop_expr.body, depth + 1);
        }
        syn::Expr::Call(call) => push_projection_node(
            nodes,
            call,
            "call",
            "call",
            format!("{};", compact_tokens(call)),
            depth,
        ),
        syn::Expr::MethodCall(method_call) => push_projection_node(
            nodes,
            method_call,
            "call",
            "call",
            format!("{};", compact_tokens(method_call)),
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
                format!(
                    "{} = {};",
                    compact_tokens(&assign.left),
                    compact_tokens(&assign.right)
                ),
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
                format!("{}.await;", compact_tokens(&await_expr.base)),
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
                format!("{}?;", compact_tokens(&try_expr.expr)),
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
        syn::Expr::Binary(binary) if binary_assignment_projection_label(binary).is_some() => {
            append_expression_projection_nodes(nodes, expression, depth);
        }
        syn::Expr::Call(_)
        | syn::Expr::Macro(_)
        | syn::Expr::MethodCall(_)
        | syn::Expr::Binary(_)
        | syn::Expr::Path(_)
        | syn::Expr::Lit(_)
        | syn::Expr::Struct(_)
        | syn::Expr::Tuple(_) => {
            push_projection_node(
                nodes,
                expression,
                "return",
                "terminal",
                tail_expression_projection_label(expression),
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
            "let {} = {};",
            local_pattern_projection_label(&local.pat),
            value_expression_projection_summary(&init.expr)
        ),
        None => format!("let {};", local_pattern_projection_label(&local.pat)),
    }
}

fn local_pattern_projection_label(pattern: &syn::Pat) -> String {
    compact_tokens(pattern)
}

fn expression_projection_summary(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Call(call) => compact_tokens(call),
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
        syn::Expr::Binary(binary) => binary_expression_projection_summary(binary),
        _ => compact_tokens(expression),
    }
}

fn condition_projection_summary(expression: &syn::Expr) -> String {
    value_expression_projection_summary(expression)
}

fn value_expression_projection_summary(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Binary(binary) => binary_expression_projection_summary(binary),
        syn::Expr::Reference(reference) => value_expression_projection_summary(&reference.expr),
        syn::Expr::Paren(paren) => value_expression_projection_summary(&paren.expr),
        syn::Expr::Cast(cast) => value_expression_projection_summary(&cast.expr),
        _ => compact_tokens(expression),
    }
}

fn binary_expression_projection_summary(binary: &syn::ExprBinary) -> String {
    format!(
        "{} {} {}",
        compact_tokens(&binary.left),
        compact_tokens(&binary.op),
        compact_tokens(&binary.right)
    )
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
}

fn return_projection_label(return_expr: &syn::ExprReturn) -> String {
    return_expr.expr.as_deref().map_or_else(
        || "return;".to_string(),
        |expr| format!("return {};", value_expression_projection_summary(expr)),
    )
}

fn break_projection_label(break_expr: &syn::ExprBreak) -> String {
    break_expr.expr.as_deref().map_or_else(
        || "break;".to_string(),
        |expr| format!("break {};", value_expression_projection_summary(expr)),
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
            "{} {} {};",
            compact_tokens(&binary.left),
            operator,
            expression_projection_summary(&binary.right)
        )
    })
}

fn macro_call_projection_label(mac: &syn::Macro) -> String {
    let arguments = compact_limited(&macro_arguments_projection(mac), 96);
    if arguments.is_empty() {
        format!("{}!();", compact_tokens(&mac.path))
    } else {
        format!("{}!({});", compact_tokens(&mac.path), arguments)
    }
}

fn macro_arguments_projection(mac: &syn::Macro) -> String {
    let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
    parser
        .parse2(mac.tokens.clone())
        .map(|arguments| {
            arguments
                .iter()
                .map(macro_argument_projection)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|_| compact_tokens(&mac.tokens))
}

fn macro_argument_projection(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Lit(_) => compact_tokens(expression),
        _ => value_expression_projection_summary(expression),
    }
}

fn tail_expression_projection_label(expression: &syn::Expr) -> String {
    value_expression_projection_summary(expression)
}

fn visibility_text(visibility: &syn::Visibility) -> String {
    let value = compact_tokens(visibility);
    if value.is_empty() {
        String::new()
    } else {
        format!("{value} ")
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
    #[derive(Clone, Copy)]
    enum LiteralKind {
        String,
        ByteString,
        RawString,
        RawByteString,
        Char,
        ByteChar,
    }

    fn literal_kind_label(kind: LiteralKind) -> &'static str {
        match kind {
            LiteralKind::String => "string",
            LiteralKind::ByteString => "byte-string",
            LiteralKind::RawString => "raw-string",
            LiteralKind::RawByteString => "raw-byte-string",
            LiteralKind::Char => "char",
            LiteralKind::ByteChar => "byte-char",
        }
    }

    fn literal_hash(value: &str) -> String {
        let mut hash = 2_166_136_261u32;
        for byte in value.as_bytes() {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(16_777_619);
        }
        format!("{hash:x}")
    }

    fn literal_projection(kind: LiteralKind, token: &str) -> String {
        if !token.chars().any(char::is_whitespace) {
            return token.to_string();
        }
        let lines = token.bytes().filter(|byte| *byte == b'\n').count() + 1;
        format!(
            "{}[lines={},bytes={},hash={}]",
            literal_kind_label(kind),
            lines,
            token.len(),
            literal_hash(token)
        )
    }

    fn take_raw_literal(
        rest: &str,
        prefix_len: usize,
        kind: LiteralKind,
    ) -> Option<(usize, LiteralKind)> {
        let bytes = rest.as_bytes();
        let mut cursor = prefix_len;
        while bytes.get(cursor).is_some_and(|byte| *byte == b'#') {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'"') {
            return None;
        }
        let hashes = cursor.saturating_sub(prefix_len);
        cursor += 1;
        while cursor < bytes.len() {
            if bytes[cursor] == b'"'
                && cursor + 1 + hashes <= bytes.len()
                && bytes[cursor + 1..cursor + 1 + hashes]
                    .iter()
                    .all(|byte| *byte == b'#')
            {
                return Some((cursor + 1 + hashes, kind));
            }
            cursor += 1;
        }
        None
    }

    fn take_quoted_literal(
        rest: &str,
        prefix_len: usize,
        quote: u8,
        kind: LiteralKind,
    ) -> Option<(usize, LiteralKind)> {
        if rest.as_bytes().get(prefix_len) != Some(&quote) {
            return None;
        }
        let mut cursor = prefix_len + 1;
        let mut escaped = false;
        while cursor < rest.len() {
            let ch = rest[cursor..].chars().next()?;
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch.len_utf8() == 1 && ch as u8 == quote {
                return Some((cursor + 1, kind));
            }
            cursor += ch.len_utf8();
        }
        None
    }

    fn take_char_literal(
        rest: &str,
        prefix_len: usize,
        kind: LiteralKind,
    ) -> Option<(usize, LiteralKind)> {
        let (literal_len, kind) = take_quoted_literal(rest, prefix_len, b'\'', kind)?;
        let body = &rest[prefix_len + 1..literal_len - 1];
        (body.starts_with('\\') || body.chars().count() == 1).then_some((literal_len, kind))
    }

    fn take_literal(rest: &str) -> Option<(usize, LiteralKind)> {
        take_raw_literal(rest, 2, LiteralKind::RawByteString)
            .filter(|_| rest.starts_with("br"))
            .or_else(|| {
                take_raw_literal(rest, 2, LiteralKind::RawString).filter(|_| rest.starts_with("cr"))
            })
            .or_else(|| {
                take_raw_literal(rest, 1, LiteralKind::RawString).filter(|_| rest.starts_with('r'))
            })
            .or_else(|| {
                take_quoted_literal(rest, 1, b'"', LiteralKind::ByteString)
                    .filter(|_| rest.starts_with('b'))
            })
            .or_else(|| {
                take_quoted_literal(rest, 1, b'"', LiteralKind::String)
                    .filter(|_| rest.starts_with('c'))
            })
            .or_else(|| {
                take_char_literal(rest, 1, LiteralKind::ByteChar).filter(|_| rest.starts_with('b'))
            })
            .or_else(|| take_quoted_literal(rest, 0, b'"', LiteralKind::String))
            .or_else(|| take_char_literal(rest, 0, LiteralKind::Char))
    }

    let mut literal_safe = String::with_capacity(value.len());
    let mut literals = Vec::<String>::new();
    let mut cursor = 0;
    while cursor < value.len() {
        let rest = &value[cursor..];
        if let Some((literal_len, kind)) = take_literal(rest) {
            let placeholder = format!("__ASP_LITERAL_{}__", literals.len());
            literals.push(literal_projection(kind, &rest[..literal_len]));
            literal_safe.push_str(&placeholder);
            cursor += literal_len;
        } else if let Some(ch) = rest.chars().next() {
            literal_safe.push(ch);
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }

    let mut compacted = literal_safe
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
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
        ("->&", "-> &"),
        ("->(", "-> ("),
    ] {
        compacted = compacted.replace(from, to);
    }
    for (index, literal) in literals.iter().enumerate() {
        compacted = compacted.replace(&format!("__ASP_LITERAL_{index}__"), literal);
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
