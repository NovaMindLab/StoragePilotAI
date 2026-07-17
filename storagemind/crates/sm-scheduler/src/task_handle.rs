use tokio_util::sync::CancellationToken;

use crate::priority::Priority;

/// An opaque handle to a spawned scheduler task.
///
/// Obtained from [`crate::Scheduler::spawn`]. Dropping the handle does **not**
/// cancel the task; call [`TaskHandle::cancel`] explicitly if cancellation is
/// desired.
pub struct TaskHandle {
    /// Unique task identifier (UUID v4).
    pub id: String,
    /// Priority the task was submitted with.
    pub priority: Priority,
    /// Token that can be used to request cancellation of the task.
    pub cancel_token: CancellationToken,
}

impl TaskHandle {
    /// Signal the task to cancel. The task itself must honour the token by
    /// checking [`CancellationToken::is_cancelled`] or `select!`-ing on
    /// [`CancellationToken::cancelled`].
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Returns `true` if cancellation has been requested for this task.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
}

impl std::fmt::Debug for TaskHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskHandle")
            .field("id", &self.id)
            .field("priority", &self.priority)
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}
