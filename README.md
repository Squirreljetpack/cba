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
The extension traits and logging macros follow a general naming pattern of `{level}{bog/log}`. For example, `result.elog()` calls `log::error!` on the error.
Each such function also has a variant whose name is prefixed with `_` which consumes the error, downgrading `Results` to `Options`, and `Option<T>` to `T` (or else exiting the program with code 1).
`prefix()` and `context()` can be used to add context to the error.

A standard pattern to handle non-fatal errors under this paradigm is like so:
```rust
let x = try()
	.prefix("Failed to copy") // {e} -> Failed to copy: {e}
	._wbog() // pretty print the warning and consume the error
```

#### Bath/Bo/Broc/Bs
Simple wrappers around standard library modules (`path, io, process, fs`) for more ergonomic usage.
A few of these operations take the liberty of logging (or bogging) errors instead of propogating them, downgrading them to Option or bool.

#### Macros/Traits/Other/...
- macros for defining wrapper types
- debug_assertion gated functions
- more random extension traits
- `vec_!` and `prints!`
- initialization helpers

## Optional features
- text: text utilities