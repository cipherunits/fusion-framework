//! Process-wide Tokio background tasks.
//!
//! Bindings spawn host-language callbacks as sync `FnOnce` jobs or async futures
//! on a dedicated multi-thread runtime (independent of HTTP `listen`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

/// How many terminal (done/cancelled/failed) tasks to retain for the monitor.
const MAX_TERMINAL_RETAINED: usize = 100;

/// Lifecycle of a background task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Cancelled,
    Failed,
}

impl TaskStatus {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Running,
            2 => Self::Done,
            3 => Self::Cancelled,
            4 => Self::Failed,
            _ => Self::Pending,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            Self::Pending => 0,
            Self::Running => 1,
            Self::Done => 2,
            Self::Cancelled => 3,
            Self::Failed => 4,
        }
    }

    /// Stable string for language bindings.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    /// Whether the task is still scheduled or executing.
    fn is_active(self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }

    /// Parse a status name (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "done" => Some(Self::Done),
            "cancelled" | "canceled" => Some(Self::Cancelled),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

struct TaskEntry {
    status: Arc<AtomicU8>,
    handle: JoinHandle<()>,
    created_at_ms: u64,
    delay_ms: Option<u64>,
}

struct TaskRegistry {
    next_id: AtomicU64,
    entries: Mutex<HashMap<String, TaskEntry>>,
}

impl TaskRegistry {
    fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn alloc_id(&self) -> String {
        let n = self.next_id.fetch_add(1, Ordering::Relaxed);
        format!("task-{n}")
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("fusion-tasks")
            .build()
            .expect("fusion background task runtime")
    })
}

fn registry() -> &'static TaskRegistry {
    static REG: OnceLock<TaskRegistry> = OnceLock::new();
    REG.get_or_init(TaskRegistry::new)
}

/// Drop oldest terminal tasks so the registry cannot grow without bound.
fn prune_terminal(entries: &mut HashMap<String, TaskEntry>) {
    let mut terminal: Vec<(String, u64)> = entries
        .iter()
        .filter_map(|(id, e)| {
            let st = TaskStatus::from_u8(e.status.load(Ordering::SeqCst));
            if st.is_active() {
                None
            } else {
                Some((id.clone(), e.created_at_ms))
            }
        })
        .collect();
    if terminal.len() <= MAX_TERMINAL_RETAINED {
        return;
    }
    terminal.sort_by_key(|(_, created)| *created);
    let remove_n = terminal.len() - MAX_TERMINAL_RETAINED;
    for (id, _) in terminal.into_iter().take(remove_n) {
        entries.remove(&id);
    }
}

/// Spawn a job immediately on the background Tokio runtime. Returns task id.
pub fn spawn_fn(job: impl FnOnce() + Send + 'static) -> String {
    spawn_inner(None, Box::new(job))
}

/// Spawn a job after `delay_ms` milliseconds. Returns task id.
pub fn spawn_after_ms(delay_ms: u64, job: impl FnOnce() + Send + 'static) -> String {
    spawn_inner(Some(Duration::from_millis(delay_ms)), Box::new(job))
}

/// Spawn an async job on the background Tokio runtime (for bindings that await host callbacks).
pub fn spawn_future<Fut>(fut: Fut) -> String
where
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    spawn_inner_future(None, fut)
}

/// Spawn an async job after `delay_ms` milliseconds.
pub fn spawn_after_ms_future<Fut>(delay_ms: u64, fut: Fut) -> String
where
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    spawn_inner_future(Some(Duration::from_millis(delay_ms)), fut)
}

fn spawn_inner(delay: Option<Duration>, job: Box<dyn FnOnce() + Send + 'static>) -> String {
    let reg = registry();
    let id = reg.alloc_id();
    let status = Arc::new(AtomicU8::new(TaskStatus::Pending.as_u8()));
    let status_run = Arc::clone(&status);
    let created_at_ms = now_ms();
    let delay_ms = delay.map(|d| d.as_millis() as u64);

    let Ok(mut guard) = reg.entries.lock() else {
        let _ = runtime().spawn(async move {
            if let Some(d) = delay {
                tokio::time::sleep(d).await;
            }
            let _ = job();
        });
        return id;
    };

    let handle = runtime().spawn(async move {
        if let Some(d) = delay {
            tokio::time::sleep(d).await;
        }
        if TaskStatus::from_u8(status_run.load(Ordering::SeqCst)) == TaskStatus::Cancelled {
            return;
        }
        status_run.store(TaskStatus::Running.as_u8(), Ordering::SeqCst);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
        let final_status = match result {
            Ok(()) => TaskStatus::Done,
            Err(_) => TaskStatus::Failed,
        };
        // Do not overwrite Cancelled if cancel raced mid-job.
        let _ = status_run.compare_exchange(
            TaskStatus::Running.as_u8(),
            final_status.as_u8(),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    });

    guard.insert(
        id.clone(),
        TaskEntry {
            status,
            handle,
            created_at_ms,
            delay_ms,
        },
    );
    prune_terminal(&mut guard);
    id
}

fn spawn_inner_future<Fut>(delay: Option<Duration>, fut: Fut) -> String
where
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let reg = registry();
    let id = reg.alloc_id();
    let status = Arc::new(AtomicU8::new(TaskStatus::Pending.as_u8()));
    let status_run = Arc::clone(&status);
    let created_at_ms = now_ms();
    let delay_ms = delay.map(|d| d.as_millis() as u64);

    let Ok(mut guard) = reg.entries.lock() else {
        let _ = runtime().spawn(async move {
            if let Some(d) = delay {
                tokio::time::sleep(d).await;
            }
            fut.await;
        });
        return id;
    };

    let handle = runtime().spawn(async move {
        if let Some(d) = delay {
            tokio::time::sleep(d).await;
        }
        if TaskStatus::from_u8(status_run.load(Ordering::SeqCst)) == TaskStatus::Cancelled {
            return;
        }
        status_run.store(TaskStatus::Running.as_u8(), Ordering::SeqCst);
        // Host async callbacks (e.g. Node TSFN) rarely panic; treat completion as Done.
        fut.await;
        let _ = status_run.compare_exchange(
            TaskStatus::Running.as_u8(),
            TaskStatus::Done.as_u8(),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    });

    guard.insert(
        id.clone(),
        TaskEntry {
            status,
            handle,
            created_at_ms,
            delay_ms,
        },
    );
    prune_terminal(&mut guard);
    id
}

/// Cancel a pending/running task. Returns whether the id was known.
pub fn cancel(id: &str) -> bool {
    let reg = registry();
    let Ok(mut guard) = reg.entries.lock() else {
        return false;
    };
    let Some(entry) = guard.get_mut(id) else {
        return false;
    };
    let current = TaskStatus::from_u8(entry.status.load(Ordering::SeqCst));
    if matches!(
        current,
        TaskStatus::Done | TaskStatus::Cancelled | TaskStatus::Failed
    ) {
        return true;
    }
    entry
        .status
        .store(TaskStatus::Cancelled.as_u8(), Ordering::SeqCst);
    entry.handle.abort();
    prune_terminal(&mut guard);
    true
}

/// Current status for a task id, if known.
pub fn status(id: &str) -> Option<TaskStatus> {
    let reg = registry();
    let guard = reg.entries.lock().ok()?;
    guard
        .get(id)
        .map(|e| TaskStatus::from_u8(e.status.load(Ordering::SeqCst)))
}

/// JSON snapshot of tracked tasks (for the cache monitor panel and bindings).
pub fn snapshot() -> Value {
    let reg = registry();
    let Ok(mut guard) = reg.entries.lock() else {
        return json!({
            "task_count": 0,
            "active_count": 0,
            "tasks": [],
        });
    };
    prune_terminal(&mut guard);

    let mut tasks: Vec<Value> = guard
        .iter()
        .map(|(id, e)| {
            let st = TaskStatus::from_u8(e.status.load(Ordering::SeqCst));
            json!({
                "id": id,
                "status": st.as_str(),
                "delay_ms": e.delay_ms,
                "created_at_ms": e.created_at_ms,
            })
        })
        .collect();
    // Newest first for the monitor table.
    tasks.sort_by(|a, b| {
        let am = a["created_at_ms"].as_u64().unwrap_or(0);
        let bm = b["created_at_ms"].as_u64().unwrap_or(0);
        bm.cmp(&am)
    });

    let active_count = tasks
        .iter()
        .filter(|t| {
            matches!(
                t["status"].as_str(),
                Some("pending") | Some("running")
            )
        })
        .count();

    json!({
        "task_count": tasks.len(),
        "active_count": active_count,
        "tasks": tasks,
    })
}

/// Drop all tracked tasks (tests). Running jobs are aborted.
pub fn reset_for_tests() {
    let reg = registry();
    if let Ok(mut guard) = reg.entries.lock() {
        for (_, entry) in guard.drain() {
            entry.handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn spawn_runs_to_done() {
        let _guard = test_lock();
        reset_for_tests();
        let flag = Arc::new(AtomicBool::new(false));
        let f = Arc::clone(&flag);
        let id = spawn_fn(move || {
            f.store(true, Ordering::SeqCst);
        });
        for _ in 0..100 {
            if status(&id) == Some(TaskStatus::Done) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(flag.load(Ordering::SeqCst), "job did not run");
        assert_eq!(status(&id), Some(TaskStatus::Done));
    }

    #[test]
    fn spawn_after_delays() {
        let _guard = test_lock();
        reset_for_tests();
        let flag = Arc::new(AtomicBool::new(false));
        let f = Arc::clone(&flag);
        let id = spawn_after_ms(80, move || {
            f.store(true, Ordering::SeqCst);
        });
        thread::sleep(Duration::from_millis(20));
        assert!(!flag.load(Ordering::SeqCst));
        assert!(matches!(
            status(&id),
            Some(TaskStatus::Pending) | Some(TaskStatus::Running)
        ));
        for _ in 0..100 {
            if flag.load(Ordering::SeqCst) && status(&id) == Some(TaskStatus::Done) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(flag.load(Ordering::SeqCst), "delayed job did not run");
        assert_eq!(status(&id), Some(TaskStatus::Done));
    }

    #[test]
    fn cancel_before_run() {
        let _guard = test_lock();
        reset_for_tests();
        let flag = Arc::new(AtomicBool::new(false));
        let f = Arc::clone(&flag);
        let id = spawn_after_ms(500, move || {
            f.store(true, Ordering::SeqCst);
        });
        assert!(cancel(&id));
        thread::sleep(Duration::from_millis(100));
        assert!(!flag.load(Ordering::SeqCst));
        assert_eq!(status(&id), Some(TaskStatus::Cancelled));
    }

    #[test]
    fn snapshot_lists_spawned_and_cancelled() {
        let _guard = test_lock();
        reset_for_tests();
        let id = spawn_after_ms(5_000, || {});
        let snap = snapshot();
        assert_eq!(snap["task_count"].as_u64(), Some(1));
        assert_eq!(snap["active_count"].as_u64(), Some(1));
        let tasks = snap["tasks"].as_array().expect("tasks array");
        assert_eq!(tasks[0]["id"].as_str(), Some(id.as_str()));
        assert_eq!(tasks[0]["status"].as_str(), Some("pending"));
        assert_eq!(tasks[0]["delay_ms"].as_u64(), Some(5_000));
        assert!(tasks[0]["created_at_ms"].as_u64().unwrap_or(0) > 0);

        assert!(cancel(&id));
        let snap2 = snapshot();
        assert_eq!(snap2["tasks"][0]["status"].as_str(), Some("cancelled"));
        assert_eq!(snap2["active_count"].as_u64(), Some(0));
    }
}
