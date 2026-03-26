//! Result type alias for the agent-harness library.

use crate::error::HarnessError;

/// Type alias for Result with HarnessError
///
/// This is used throughout the library for consistent error handling.
///
/// # Example
///
/// ```rust
/// use agent_harness::error::Result;
///
/// async fn do_something() -> Result<String> {
///     Ok("success".to_string())
/// }
/// ```
pub type Result<T, E = HarnessError> = std::result::Result<T, E>;

/// Type alias for boxed errors that can be sent across threads
///
/// Used when the error type needs to be dynamic but still Send + Sync.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
