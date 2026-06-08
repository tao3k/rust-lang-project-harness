//! Public API owner.

/// Classifies rows.
pub fn classify(rows: &[usize], enabled: bool) -> usize {
    let mut total = 0;
    for row in rows {
        if enabled {
            if *row > 10 {
                if *row < 20 {
                    total += *row;
                }
            }
        }
    }
    total
}

/// Routes a kind.
pub fn route(kind: &str) -> usize {
    if kind == "alpha" {
        1
    } else if kind == "beta" {
        2
    } else if kind == "gamma" {
        3
    } else {
        0
    }
}

/// Summarizes values.
pub fn summarize(value: usize) -> usize {
    let step_0 = value + 0;
    let step_1 = value + 1;
    let step_2 = value + 2;
    let step_3 = value + 3;
    let step_4 = value + 4;
    let step_5 = value + 5;
    let step_6 = value + 6;
    let step_7 = value + 7;
    let step_8 = value + 8;
    let step_9 = value + 9;
    let step_10 = value + 10;
    let step_11 = value + 11;
    let step_12 = value + 12;
    let step_13 = value + 13;
    let step_14 = value + 14;
    step_0
        + step_1
        + step_2
        + step_3
        + step_4
        + step_5
        + step_6
        + step_7
        + step_8
        + step_9
        + step_10
        + step_11
        + step_12
        + step_13
        + step_14
}

/// Summarizes values with explicit loops.
pub fn summarize_loop(values: &[usize]) -> bool {
    let mut doubled = Vec::new();
    for value in values {
        if *value > 0 {
            doubled.push(*value * 2);
        }
    }
    for value in values {
        if *value > 100 {
            return true;
        }
    }
    let mut count = 0;
    for value in values {
        if *value > 10 {
            count += 1;
        }
    }
    let mut total = 0;
    for value in values {
        total += *value;
    }
    let _ = (doubled, count, total);
    false
}
