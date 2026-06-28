# Expected

This scenario is based on the `marlin-gerbil-scheme` AOT probe boundary.
Agents should preserve separate Rust owners for:

- command construction and current directory/environment shaping
- path-list updates through `env::split_paths` and `env::join_paths`
- stdout/stderr/status projection into a typed receipt
- backend/helper path extraction from receipt text

The anti-pattern is a single probe function that mixes `Command::new`, string
PATH concatenation, `unwrap`/`expect`, lossy output parsing, and receipt
construction.
