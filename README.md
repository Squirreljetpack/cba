# CBA [![Crates.io](https://img.shields.io/crates/v/cli_boilerplate_automation)](https://crates.io/crates/cli_boilerplate_automation) [![License](https://img.shields.io/github/license/squirreljetpack/cba)](https://github.com/squirreljetpack/cba/blob/main/LICENSE)

Cli Boilerplate Automation.
A small library of assorted utilities for cli tool development.

## Modules

### Bogger

A stateful logger for displaying messages styled according to their level.
The standard initialization method writes a message like the following to stdout/stderr:

```syslog
[ERROR: tag] my_content.
```

where the tag is colored.

The logger can be configured at runtime to filter, downcast, pause and forward the messages it recieves.

Additionally, `format!`-like macros are provided: `ebog!("optional_tag"; "{my_content}")`.

#### Extension traits for unwrapping

The extension traits and logging macros follow a general naming pattern of `{level}{bog/log}`. For example, `result.elog()` calls `log::error!` on the error.
Each such function also has a variant whose name is prefixed with `_` which consumes the error, downgrading `Results` to `Options`, and `Option<T>` to `T` (or else exiting the program with code 1).
Additionally, `prefix()` and `context()` can be used to add context to the error.

A standard pattern to handle non-fatal errors under this paradigm is like so:

```rust
let x = try()
	.prefix("Failed to copy") // {e} -> Failed to copy: {e}
	._wbog() // pretty print the warning and consume the error
```

There is also a flexible `unwrap!` macro, which can be slightly more convenient than `let/else`. It unwraps Result and Option, or otherwise calls `return`/`continue` for a never type.

```rust
// Unwrap Option or return Default::default()
pub fn check_should_template(path: &Path) -> bool {
	let err_prefix = format!("Failed to read {path:?} for templating");
	let file = unwrap!(fs::File::open(path).prefix(&err_prefix)._ebog());

	// process file

	true
}

// A closure in the second argument unwraps Result:
for binname in list {
	let Action {
		name,
		alias,
		..
	} = unwrap!(
    	binname.parse();
    	|e| { err_count += 1; ebog!("Scan"; "Failed to parse filename of {}: {e}", path.to_string_lossy()) };
    	continue
	);

	println!("{name}");
}

// (This macro also supports a few other sensible input patterns).
```

### Bath/Bo/Broc/Bs

Simple wrappers around standard library modules (`path, io, process, fs`) for more ergonomic usage.
A few of these operations take the liberty of logging (or bogging) errors instead of propogating them, downgrading them to Option or bool.

### Macros

- for defining wrapper types:

```rust
// Define a newtype wrapper which can only be be created or modified by methods you implement.
define_restricted_wrapper!(Prefix: u8 = 0);

// Define a newtype wrapper with a custom default.
define_transparent_wrapper!(
	#[derive(Copy)]
	Count: u16 = 1
);

// Define a collection wrapper with a new function and many standard traits such as IntoIterator/Deref/etc. implemented.
define_collection_wrapper!(
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    Modules: HashMap<String, Module>
);
```

- `vec_!`: map functions over the elements of a `vec![]` call.
- `prints!`: write elements to stdout or a buffer directly, without dynamic dispatch.
- ... and a few others

### Others

- Some extension traits
  - Float, Option, Result, Bool
  - Mutex: `_lock` (ignore poisoning)
  - Transform
- initialization helpers

For example:

```rust
// transform elements
err_count += curr
	.update_from_config_path(dir, lib, &name, targets)?
	.transform(|s| if s == usize::MAX { 0 } else { s });
```

## Optional features

- text: text utilities (string padding, table formatting, etc.)
- serde: serde derivations on some types, and some serde utils
