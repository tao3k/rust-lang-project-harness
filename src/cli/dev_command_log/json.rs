use super::command::NormalizedCommand;

pub(super) fn push_command_field(
    output: &mut String,
    first: &mut bool,
    command: &NormalizedCommand,
) {
    push_key(output, first, "command");
    let mut nested_first = true;
    output.push('{');
    push_string_field(output, &mut nested_first, "namespace", &command.namespace);
    push_string_field(output, &mut nested_first, "method", &command.method);
    if let Some(view) = &command.view {
        push_string_field(output, &mut nested_first, "view", view);
    }
    if let Some(render_mode) = &command.render_mode {
        push_string_field(output, &mut nested_first, "renderMode", render_mode);
    }
    if let Some(query) = &command.query {
        push_string_field(output, &mut nested_first, "query", query);
    }
    push_u64_field(
        output,
        &mut nested_first,
        "querySetCount",
        command.query_set_count as u64,
    );
    push_array_field(output, &mut nested_first, "pipes", &command.pipes);
    output.push('}');
}

pub(super) fn push_result_field(
    output: &mut String,
    first: &mut bool,
    exit_code: i32,
    elapsed_ms: u64,
) {
    push_key(output, first, "result");
    let mut nested_first = true;
    output.push('{');
    push_i32_field(output, &mut nested_first, "exitCode", exit_code);
    push_u64_field(output, &mut nested_first, "elapsedMs", elapsed_ms);
    push_u64_field(output, &mut nested_first, "stdoutBytes", 0);
    push_u64_field(output, &mut nested_first, "stderrBytes", 0);
    push_string_field(
        output,
        &mut nested_first,
        "status",
        if exit_code == 0 { "success" } else { "failure" },
    );
    output.push('}');
}

pub(super) fn push_fields_field(output: &mut String, first: &mut bool, context_source: &str) {
    push_key(output, first, "fields");
    let mut nested_first = true;
    output.push('{');
    push_bool_field(output, &mut nested_first, "outputBytesMeasured", false);
    push_string_field(
        output,
        &mut nested_first,
        "logFileNaming",
        "utc-second-session-ordinal-event",
    );
    push_string_field(output, &mut nested_first, "sequenceScope", "session");
    push_string_field(output, &mut nested_first, "contextSource", context_source);
    output.push('}');
}

pub(super) fn push_string_field(output: &mut String, first: &mut bool, key: &str, value: &str) {
    push_key(output, first, key);
    push_json_string(output, value);
}

pub(super) fn push_i32_field(output: &mut String, first: &mut bool, key: &str, value: i32) {
    push_key(output, first, key);
    output.push_str(&value.to_string());
}

pub(super) fn push_u64_field(output: &mut String, first: &mut bool, key: &str, value: u64) {
    push_key(output, first, key);
    output.push_str(&value.to_string());
}

pub(super) fn push_bool_field(output: &mut String, first: &mut bool, key: &str, value: bool) {
    push_key(output, first, key);
    output.push_str(if value { "true" } else { "false" });
}

pub(super) fn push_array_field(
    output: &mut String,
    first: &mut bool,
    key: &str,
    values: &[String],
) {
    push_key(output, first, key);
    output.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_json_string(output, value);
    }
    output.push(']');
}

pub(super) fn push_key(output: &mut String, first: &mut bool, key: &str) {
    if *first {
        *first = false;
    } else {
        output.push(',');
    }
    push_json_string(output, key);
    output.push(':');
}

pub(super) fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push('"');
}
