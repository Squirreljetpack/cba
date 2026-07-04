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

### Bath/Bo/Broc/Bs

Simple wrappers around standard library modules (`path, io, process, fs`) for more ergonomic usage.
A few of these operations take the liberty of wrappping errors into StringErrors which can be directly printed. Rarely, the errors are logged (or bogged) instead of propogated, downgrading the return type to Option or bool.

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

// Define a collection wrapper with a new function and many standard traits (such as IntoIterator and DerefMut) pre-implemented.
define_collection_wrapper!(
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    Modules: HashMap<String, Module>
);
```

- `vec_!`: map functions over the elements of a `vec![]` call.
- `prints!`: write elements to stdout or a buffer directly, without dynamic dispatch.
- `_dbg!`: Without a prefix: dbg! if debug_assertions. With a prefix, log::debug. The value is returned.	
- `_info!`: flexible macro for logging string literals, named or unnamed expressions only in debug builds.
- ... and a couple more

### Others

- Some extension traits
  - Float, Option, Result, Bool
  - Mutex: `_lock` (ignore poisoning)
  - Transform: Blanket impl of some post-fix methods. These are sometimes nice to use particularly with the builder pattern.
- initialization helpers

```rust
// transform elements
err_count += curr
	.update_from_config_path(dir, lib, &name, targets)?
	.transform(|s| if s == usize::MAX { 0 } else { s });

// modify elements
config.binds = default_binds().modify(|x| x.extend(config.binds));

```

## Optional features

- text: text utilities (string padding, table formatting, string splitting, etc.)
- serde: serde derivations on some types, and some serde utils
