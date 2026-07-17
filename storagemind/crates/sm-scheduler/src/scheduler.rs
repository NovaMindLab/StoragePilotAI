use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

use tokio::sync::{Notify, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{SchedulerError, SchedulerResult};
use crate::priority::Priority;
use crate::task_handle::TaskHandle;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the global scheduler.
///
/// Created once at application start and passed to [`Scheduler::new`].
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum number of background tasks that may run concurrently.
    ///
    /// P0/P1 (UI + Watcher) tasks are **not** counted against this limit;
    /// they always run immediately. Defaults to 4.
    pub max_concurrent: usize,

    /// Start the scheduler in the paused state (useful for tests).
    pub paused: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            paused: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

/// The **sole** global task scheduler for StorageMind.
///
/// # Usage
///
/// ```rust,no_run
/// use sm_scheduler::{Scheduler, SchedulerConfig, Priority};
///
/// #[tokio::main]
/// async fn main() {
///     let scheduler = Scheduler::new(SchedulerConfig::default());
///
///     let handle = scheduler.spawn(Priority::Scanner, |token| async move {
///         // ... do scan work, honour token ...
///     }).unwrap();
///
///     // Later:
///     handle.cancel();
///     scheduler.shutdown();
/// }
/// ```
///
/// # Architecture notes
///
/// - **Single instance**: wrap in `Arc` (returned by [`Scheduler::new`]) and
///   share across the application via `tauri::State` or a `once_cell`.
/// - **Priority gates**: Tasks with priority ≥ P2 (`Scanner`) automatically
///   wait behind a [`tokio::sync::Semaphore`] (bounded by `max_concurrent`)
///   and respect the pause state. P0/P1 tasks bypass both.
/// - **Pause/Resume**: [`Scheduler::pause`] stops P2+ tasks from starting
///   (already-running tasks finish their current await point). Calling
///   [`Scheduler::resume`] wakes all waiters via a [`Notify`].
/// - **Shutdown**: After [`Scheduler::shutdown`] no new tasks are accepted.
///   In-flight tasks are not forcibly killed — callers should cancel their
///   [`TaskHandle`] if an immediate stop is needed.
pub struct Scheduler {
    /// Mutable config (rarely changed after init).
    config: Mutex<SchedulerConfig>,
    /// Limits concurrent background (P2+) tasks.
    semaphore: Arc<Semaphore>,
    /// Pause gate for P2+ tasks.
    paused: Arc<AtomicBool>,
    /// Running task counter (informational).
    active_tasks: Arc<AtomicUsize>,
    /// Set to `true` after [`Scheduler::shutdown`]; rejects new submissions.
    shutdown: Arc<AtomicBool>,
    /// Notified when pause is lifted so waiting tasks can proceed.
    unpause_notify: Arc<Notify>,
}

impl Scheduler {
    /// Create a new scheduler and return it wrapped in an `Arc`.
    pub fn new(config: SchedulerConfig) -> Arc<Self> {
        let max = config.max_concurrent;
        let initially_paused = config.paused;

        let scheduler = Arc::new(Self {
            config: Mutex::new(config),
            semaphore: Arc::new(Semaphore::new(max)),
            paused: Arc::new(AtomicBool::new(initially_paused)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
            unpause_notify: Arc::new(Notify::new()),
        });

        info!(max_concurrent = max, "Scheduler initialised");
        scheduler
    }

    // -----------------------------------------------------------------------
    // Task submission
    // -----------------------------------------------------------------------

    /// Spawn a background task with the given [`Priority`].
    ///
    /// The closure receives a [`CancellationToken`] and **must** honour it —
    /// either by polling [`CancellationToken::is_cancelled`] between work
    /// units or by `select!`-ing on [`CancellationToken::cancelled`].
    ///
    /// Returns a [`TaskHandle`] immediately; the actual task is submitted to
    /// the tokio runtime and may not start running until a semaphore slot
    /// becomes available.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::Shutdown`] if the scheduler has been shut down.
    pub fn spawn<F, Fut>(
        self: &Arc<Self>,
        priority: Priority,
        task_fn: F,
    ) -> SchedulerResult<TaskHandle>
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        if self.shutdown.load(Ordering::Relaxed) {
            warn!("Rejected task submission — scheduler is shut down");
            return Err(SchedulerError::Shutdown);
        }

        let id = Uuid::new_v4().to_string();
        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();

        // Clone Arcs for the async block
        let semaphore = self.semaphore.clone();
        let paused = self.paused.clone();
        let unpause_notify = self.unpause_notify.clone();
        let active = self.active_tasks.clone();
        let task_id = id.clone();
        let prio_val = priority.as_u8();
        let prio_name = priority.tokio_task_name();

        tokio::spawn(async move {
            // ----------------------------------------------------------------
            // High-priority tasks (P0/P1) skip the semaphore and pause gate.
            // ----------------------------------------------------------------
            let _permit = if prio_val >= Priority::Scanner.as_u8() {
                // Acquire a semaphore slot — this blocks until one is free.
                let permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        // Semaphore closed (scheduler shutdown path).
                        debug!(task = %task_id, "Semaphore closed, dropping task");
                        return;
                    }
                };

                // Respect the pause gate (spin-wait with notification).
                loop {
                    if cancel_clone.is_cancelled() {
                        debug!(task = %task_id, "Task cancelled while waiting for unpause");
                        return;
                    }
                    if !paused.load(Ordering::Relaxed) {
                        break;
                    }
                    debug!(task = %task_id, priority = prio_name, "Task waiting — scheduler paused");
                    tokio::select! {
                        biased;
                        _ = cancel_clone.cancelled() => {
                            debug!(task = %task_id, "Task cancelled while paused");
                            return;
                        }
                        _ = unpause_notify.notified() => {
                            // Loop around to re-check the pause flag.
                        }
                    }
                }

                Some(permit)
            } else {
                // P0/P1: run immediately, do not hold a semaphore slot.
                None
            };

            if cancel_clone.is_cancelled() {
                debug!(task = %task_id, priority = prio_name, "Task cancelled before start");
                return;
            }

            active.fetch_add(1, Ordering::Relaxed);
            debug!(task = %task_id, priority = prio_name, "Task started");

            task_fn(cancel_clone).await;

            active.fetch_sub(1, Ordering::Relaxed);
            debug!(task = %task_id, priority = prio_name, "Task completed");
            // _permit is dropped here, releasing the semaphore slot.
        });

        debug!(task = %id, priority = prio_name, "Task submitted");
        Ok(TaskHandle {
            id,
            priority,
            cancel_token,
        })
    }

    // -----------------------------------------------------------------------
    // Control surface
    // -----------------------------------------------------------------------

    /// Pause all background tasks with priority ≥ P2 (`Scanner`).
    ///
    /// Tasks that are already running will continue until their next
    /// cooperative yield point. Newly submitted P2+ tasks will queue up
    /// behind the pause gate until [`Scheduler::resume`] is called.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
        info!("Scheduler paused — P2+ tasks will wait");
    }

    /// Resume all paused P2+ tasks.
    ///
    /// All tasks that were blocked on the pause gate are notified and will
    /// compete for semaphore slots in the normal way.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
        self.unpause_notify.notify_waiters();
        info!("Scheduler resumed");
    }

    /// Returns `true` if the scheduler is currently paused.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Returns the number of tasks that are currently executing (past the
    /// semaphore gate and past any pause delay).
    pub fn active_count(&self) -> usize {
        self.active_tasks.load(Ordering::Relaxed)
    }

    /// Return the current [`SchedulerConfig`] snapshot.
    pub fn config(&self) -> SchedulerConfig {
        self.config.lock().expect("scheduler config lock poisoned").clone()
    }

    /// Initiate a graceful shutdown.
    ///
    /// After this call:
    /// - New task submissions are rejected with [`SchedulerError::Shutdown`].
    /// - The pause gate is lifted so that any paused tasks can observe their
    ///   cancellation tokens and exit cleanly.
    /// - In-flight tasks are **not** forcibly killed; cancel their
    ///   [`TaskHandle`]s if you need them to stop immediately.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        // Lift pause so paused tasks can observe their cancel tokens.
        self.paused.store(false, Ordering::Relaxed);
        self.unpause_notify.notify_waiters();
        info!("Scheduler shutdown initiated");
    }

    /// Returns `true` if [`Scheduler::shutdown`] has been called.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
}

impl std::fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scheduler")
            .field("active_tasks", &self.active_count())
            .field("paused", &self.is_paused())
            .field("shutdown", &self.is_shutdown())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    /// Helper to build a paused scheduler with small concurrency for tests.
    fn test_scheduler() -> Arc<Scheduler> {
        Scheduler::new(SchedulerConfig {
            max_concurrent: 4,
            paused: false,
        })
    }

    #[tokio::test]
    async fn spawn_and_execute_task() {
        let scheduler = test_scheduler();
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();

        let _handle = scheduler
            .spawn(Priority::Scanner, move |_token| async move {
                c.fetch_add(1, Ordering::Relaxed);
            })
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn cancel_token_works() {
        let scheduler = test_scheduler();
        let done = Arc::new(AtomicBool::new(false));
        let d = done.clone();

        let handle = scheduler
            .spawn(Priority::Hash, move |token| async move {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => {}
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
                        d.store(true, Ordering::Relaxed);
                    }
                }
            })
            .unwrap();

        handle.cancel();
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        // `done` must still be false because the long sleep was pre-empted.
        assert!(!done.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn pause_blocks_p2_plus_tasks() {
        let scheduler = test_scheduler();
        scheduler.pause();

        let started = Arc::new(AtomicBool::new(false));
        let s = started.clone();

        let _handle = scheduler
            .spawn(Priority::Metadata, move |_token| async move {
                s.store(true, Ordering::Relaxed);
            })
            .unwrap();

        // Task should not have started yet.
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(!started.load(Ordering::Relaxed), "task should be paused");

        // Resume — task should now run.
        scheduler.resume();
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        assert!(started.load(Ordering::Relaxed), "task should have run after resume");
    }

    #[tokio::test]
    async fn high_priority_tasks_bypass_pause() {
        let scheduler = test_scheduler();
        scheduler.pause();

        let started = Arc::new(AtomicBool::new(false));
        let s = started.clone();

        // P0 / P1 tasks must run even while paused.
        let _handle = scheduler
            .spawn(Priority::UiIpc, move |_token| async move {
                s.store(true, Ordering::Relaxed);
            })
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        assert!(started.load(Ordering::Relaxed), "P0 task must bypass pause");

        scheduler.resume();
    }

    #[tokio::test]
    async fn shutdown_rejects_new_tasks() {
        let scheduler = test_scheduler();
        scheduler.shutdown();

        let result = scheduler.spawn(Priority::Scanner, |_token| async {});
        assert!(matches!(result, Err(SchedulerError::Shutdown)));
    }

    #[tokio::test]
    async fn active_count_tracks_running_tasks() {
        let scheduler = test_scheduler();
        let gate = Arc::new(Notify::new());
        let gate_clone = gate.clone();

        let _handle = scheduler
            .spawn(Priority::Thumbnail, move |_token| async move {
                gate_clone.notified().await;
            })
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert_eq!(scheduler.active_count(), 1);

        gate.notify_one();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert_eq!(scheduler.active_count(), 0);
    }
}
