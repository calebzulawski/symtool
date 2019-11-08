//! This crate provides utilities for patching symbol tables in ELF and Mach-O binaries.
//! 
//! This is the implementation behind the [symtool](https://github.com/calebzulawski/symtool)
//! utility.

pub mod elf;
pub mod error;
pub mod mach;
pub mod object;
pub mod patch;
