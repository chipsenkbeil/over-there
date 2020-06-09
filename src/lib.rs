#[cfg(feature = "cli")]
/// Contains CLI-specific code for the binary
pub mod cli;

/// Contains necessary structures and code for client/server interaction
pub mod core;

/// Contains transport-related functionality used in core
pub mod transport;

/// Contains miscellaneous code used throughout the project
pub mod utils;
