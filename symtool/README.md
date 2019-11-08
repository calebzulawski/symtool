# symtool
Static symbol manipulation tool for ELF and Mach-O objects

[![Build Status](https://github.com/calebzulawski/symtool/workflows/Continuous%20Integration/badge.svg)](https://github.com/calebzulawski/symtool)

## Installation
### Manual installation
Download the [latest release for your operating system](https://github.com/calebzulawski/symtool/releases).

### Homebrew (macOS, Linux, Windows Subsystem for Linux)
```bash
brew tap calebzulawski/symtool http://github.com/calebzulawski/symtool.git
brew install symtool
```

### Cargo
```bash
cargo install symtool
```

## File type support
Supports ELF and Mach-O objects, and archives of objects.

## Capability
* Changing symbol visibility
* Renaming symbols
* Actions are performed in-place, leaving the rest of the binary untouched

## Examples
### Change symbol visibility
Hide all symbols starting with `foo` and expose all symbols ending in `bar`.
```sh
symtool --hidden "^foo" --default "bar$" input.o output.o
```
### Rename a symbol
Rename the symbol `foo` to `bar`.
```sh
symtool --rename foo bar input.o output.o
```
Note: symbols are renamed in-place so the new name cannot be longer than the original.

## Why use symtool?
* Pretty fast (objects are simply patched, no regeneration or relocations necessary)
* Supports a wide variety of unusual object formats (for example, Intel's ICC merges string tables)
* Cross-platform method of adjusting symbol visibility of existing objects and archives (GNU ld can do this when linking, but Apple's ld64 cannot)

## License
symtool is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
