use thiserror::Error;

/// An error indicating that session locking failed because the compositor doesn't support the
/// required ext-session-lock-v1 protocol.
#[derive(Debug, Error)]
#[error("Compositor doesn't support ext_session_lock_v1")]
pub struct SessionLockNotSupportedError;
