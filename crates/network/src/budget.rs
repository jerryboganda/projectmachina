//! The caller-supplied request/byte budget hook. `network` does not depend
//! on `machina-session`; `native-core` supplies an adapter over `Session`
//! (calling `Page::account(PageResourceKind::NetworkRequests, 1)` /
//! `Page::account(PageResourceKind::NetworkBytes, n)`, which already
//! accumulates incrementally -- no additive change to `machina-session` was
//! required for this task; see the evidence file for the full rationale).

use crate::error::NetworkError;

/// Narrow trait a caller implements to enforce request/byte budgets. Both
/// methods are synchronous and must fail closed: if accounting cannot be
/// performed (e.g. the underlying store is unavailable), returning `Err`
/// aborts the request rather than proceeding un-metered.
pub trait RequestBudget: Send + Sync {
    /// Called once per logical request (including once per redirect hop).
    fn reserve_request(&self) -> Result<(), NetworkError>;
    /// Called after every body chunk is read, with the chunk's decompressed
    /// length. The client aborts the instant this would exceed budget, not
    /// after buffering the rest of the body.
    fn account_bytes(&self, n: u64) -> Result<(), NetworkError>;
}

/// A budget that never rejects. Used by tests and by callers that enforce
/// budgets entirely outside this crate; not used as an implicit default
/// anywhere a real caller is expected to supply one.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnlimitedBudget;

impl RequestBudget for UnlimitedBudget {
    fn reserve_request(&self) -> Result<(), NetworkError> {
        Ok(())
    }

    fn account_bytes(&self, _n: u64) -> Result<(), NetworkError> {
        Ok(())
    }
}
