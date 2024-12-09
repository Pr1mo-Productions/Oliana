
// The cli module holds functions related to command-line,
// environment-variable, and other configuration structures.
// Most of this is implemented in structs.rs, so we expose the namespace
// here so calling code can refer to eg crate::cli::structs::Args as cli::Args.

pub mod structs;
pub use structs::*;

