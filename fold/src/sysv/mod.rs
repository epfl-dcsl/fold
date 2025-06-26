//! Modules to process System V ABI compliant executables.
//! 
//! For more details, see the project's report.

pub mod collector;
pub mod error;
pub mod loader;
pub mod protect;
pub mod relocation;
pub mod start;
pub mod tls;
