# CBA [![Crates.io](https://img.shields.io/crates/v/cli_boilerplate_automation)](https://crates.io/crates/cli_boilerplate_automation) [![License](https://img.shields.io/github/license/squirreljetpack/cba)](https://github.com/squirreljetpack/cba/blob/main/LICENSE)

Cli Boilerplate Automation.
A small library of assorted utilities for cli tool development.

## Modules
#### Bogger
A stateful logger for displaying messages styled according to their level.
The standard initialization method writes a message like the following to stdout/stderr:
```syslog
[ERROR: tag] my_content.
```
where the tag is colored.

The logger can be configured at runtime to filter, downcast, pause and forward the messages it recieves.

Additionally, `format!`-like macros are provided: `ebog!("{my_content}"; "{optional_tag}")`.

#### Extension traits for unwrapping
The extension traits and logging macros follow a general naming pattern of `{level}{bog/log}`.
Each such function also has a variant whose name is prefixed with `_` which consumes the error, downgrading `Results` to `Options`, and `Option<T>` to `T` (or else exiting the program with code 1).
`prefix_err()` can be used to add context to the error.

A standard pattern to handle non-fatal errors under this paradigm is like so:
```rust
let x = else_default!(
	try()
	.prefix_err("Failed to copy") // {e} -> Failed to copy: {e}
	._wbog() // warn and consume the error
);
```

#### Bath/Bo/Broc/Bs
Simple wrappers around standard library modules (`path, io, process, fs`) for more ergonomic usage.
A few of these operations take the liberty of bogging pertinent errors instead of propogating them, downgrading them to Option or bool.

#### Macros/Misc/...
- macros for defining wrapper types
- debug_assertion gated functions
- more random extension traits
- `vec_!` and `prints!`
- initialization helpers

## Optional features
- text: text utilities