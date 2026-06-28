# Native ABI Contract Surface

The expected shape follows the Marlin native ABI owner pattern: the `repr(C)` layout, ABI version, ABI id, header path, and embedded header source live in the same Rust owner.
