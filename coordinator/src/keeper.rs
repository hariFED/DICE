//! DICE Keeper — automated transaction execution service.
//!
//! Runs as an independent `tokio::spawn` task alongside VRF.
//! Evaluates trigger conditions on a configurable interval and
//! submits Solana transactions when tasks are due.

use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::metrics::Metrics;
use crate::solana_rpc::SolanaRpc;
use crate::solana_tx::OnChainCtx;

// ---------------------------------------------------------------------------
// Configuration constants
// ---------------------------------------------------------------------------

/// Maximum execution history entries kept in memory.
const MAX_HISTORY: usize = 200;

// ---------------------------------------------------------------------------
// Trigger types
// ---------------------------------------------------------------------------

/// When should a keeper task execute?
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeeperTrigger {
    /// Execute every `every_secs` seconds.
    Interval { every_secs: u64 },
    /// Execute once (one-shot). Automatically disabled after firing.
    Once,
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A registered keeper task — represents one recurring (or one-shot) job.
#[derive(Debug, Clone)]
pub struct KeeperTask {
    pub id: Uuid,
    pub name: String,
    pub trigger: KeeperTrigger,
    pub target_program: Pubkey,
    pub instruction_data: Vec<u8>,
    pub accounts: Vec<AccountMeta>,
    pub enabled: bool,
    pub created_at: Instant,
    /// Next time this task should fire.
    pub next_fire: Instant,
    // Stats
    pub total_executions: u64,
    pub total_failures: u64,
    pub last_execution: Option<Instant>,
    pub last_tx_signature: Option<String>,
    pub last_error: Option<String>,
}

/// Serializable view of a task for API responses.
#[derive(Serialize)]
pub struct KeeperTaskInfo {
    pub id: String,
    pub name: String,
    pub trigger: KeeperTrigger,
    pub target_program: String,
    pub enabled: bool,
    pub total_executions: u64,
    pub total_failures: u64,
    pub last_tx_signature: Option<String>,
    pub last_error: Option<String>,
    pub next_fire_in_secs: Option<u64>,
    pub success_rate: f64,
}

/// Record of a single keeper execution.
#[derive(Debug, Clone, Serialize)]
pub struct KeeperExecution {
    pub task_id: String,
    pub task_name: String,
    pub tx_signature: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub latency_ms: u64,
    pub timestamp: u64,
}

/// Aggregate keeper stats for the dashboard.
#[derive(Serialize)]
pub struct KeeperStats {
    pub enabled: bool,
    pub total_tasks: usize,
    pub active_tasks: usize,
    pub total_executions: u64,
    pub total_failures: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub recent_executions: usize,
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

pub struct KeeperManager {
    pub tasks: Vec<KeeperTask>,
    pub history: VecDeque<KeeperExecution>,
    pub running: u32,
    pub max_concurrent: u32,
}

pub type KeeperState = Arc<Mutex<KeeperManager>>;

pub fn new_keeper_state(max_concurrent: u32) -> KeeperState {
    Arc::new(Mutex::new(KeeperManager {
        tasks: Vec::new(),
        history: VecDeque::with_capacity(MAX_HISTORY),
        running: 0,
        max_concurrent,
    }))
}

// ---------------------------------------------------------------------------
// Task management
// ---------------------------------------------------------------------------

/// Register a new keeper task. Returns the assigned UUID.
pub fn register_task(
    manager: &mut KeeperManager,
    name: String,
    trigger: KeeperTrigger,
    target_program: Pubkey,
    instruction_data: Vec<u8>,
    accounts: Vec<AccountMeta>,
) -> Uuid {
    let id = Uuid::new_v4();
    let now = Instant::now();
    let next_fire = match &trigger {
        KeeperTrigger::Interval { every_secs } => now + Duration::from_secs(*every_secs),
        KeeperTrigger::Once => now, // fire immediately
    };

    let task = KeeperTask {
        id,
        name: name.clone(),
        trigger,
        target_program,
        instruction_data,
        accounts,
        enabled: true,
        created_at: now,
        next_fire,
        total_executions: 0,
        total_failures: 0,
        last_execution: None,
        last_tx_signature: None,
        last_error: None,
    };

    info!(task_id = %id, name = %name, "keeper task registered");
    manager.tasks.push(task);
    id
}

/// List all tasks as serializable info objects.
pub fn list_tasks(manager: &KeeperManager) -> Vec<KeeperTaskInfo> {
    let now = Instant::now();
    manager
        .tasks
        .iter()
        .map(|t| {
            let next_fire_in_secs = if t.enabled && t.next_fire > now {
                Some(t.next_fire.duration_since(now).as_secs())
            } else if t.enabled {
                Some(0)
            } else {
                None
            };
            let success_rate = if t.total_executions > 0 {
                (t.total_executions - t.total_failures) as f64 / t.total_executions as f64
            } else {
                1.0
            };
            KeeperTaskInfo {
                id: t.id.to_string(),
                name: t.name.clone(),
                trigger: t.trigger.clone(),
                target_program: t.target_program.to_string(),
                enabled: t.enabled,
                total_executions: t.total_executions,
                total_failures: t.total_failures,
                last_tx_signature: t.last_tx_signature.clone(),
                last_error: t.last_error.clone(),
                next_fire_in_secs,
                success_rate,
            }
        })
        .collect()
}

/// Compute aggregate stats.
pub fn compute_stats(manager: &KeeperManager) -> KeeperStats {
    let total_executions: u64 = manager.tasks.iter().map(|t| t.total_executions).sum();
    let total_failures: u64 = manager.tasks.iter().map(|t| t.total_failures).sum();
    let success_rate = if total_executions > 0 {
        (total_executions - total_failures) as f64 / total_executions as f64
    } else {
        1.0
    };
    let avg_latency_ms = if manager.history.is_empty() {
        0.0
    } else {
        manager.history.iter().map(|e| e.latency_ms).sum::<u64>() as f64
            / manager.history.len() as f64
    };

    KeeperStats {
        enabled: true,
        total_tasks: manager.tasks.len(),
        active_tasks: manager.tasks.iter().filter(|t| t.enabled).count(),
        total_executions,
        total_failures,
        success_rate,
        avg_latency_ms,
        recent_executions: manager.history.len(),
    }
}

/// Toggle a task's enabled state. Returns Ok(new_state) or Err if not found.
pub fn toggle_task(manager: &mut KeeperManager, task_id: Uuid) -> Result<bool> {
    let task = manager
        .tasks
        .iter_mut()
        .find(|t| t.id == task_id)
        .ok_or_else(|| anyhow!("task not found: {}", task_id))?;
    task.enabled = !task.enabled;
    if task.enabled {
        // Reset next_fire so it doesn't fire immediately for stale timers.
        let now = Instant::now();
        task.next_fire = match &task.trigger {
            KeeperTrigger::Interval { every_secs } => now + Duration::from_secs(*every_secs),
            KeeperTrigger::Once => now,
        };
    }
    info!(task_id = %task_id, enabled = task.enabled, "keeper task toggled");
    Ok(task.enabled)
}

/// Remove a task by ID. Returns Ok(()) or Err if not found.
pub fn remove_task(manager: &mut KeeperManager, task_id: Uuid) -> Result<()> {
    let pos = manager
        .tasks
        .iter()
        .position(|t| t.id == task_id)
        .ok_or_else(|| anyhow!("task not found: {}", task_id))?;
    let removed = manager.tasks.remove(pos);
    info!(task_id = %task_id, name = %removed.name, "keeper task removed");
    Ok(())
}

/// Get execution history (most recent first).
pub fn get_history(manager: &KeeperManager, limit: usize) -> Vec<&KeeperExecution> {
    manager.history.iter().rev().take(limit).collect()
}

// ---------------------------------------------------------------------------
// Trigger evaluation
// ---------------------------------------------------------------------------

/// Returns indices of tasks that are due to fire right now.
fn evaluate_triggers(tasks: &[KeeperTask]) -> Vec<usize> {
    let now = Instant::now();
    tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.enabled && now >= t.next_fire)
        .map(|(i, _)| i)
        .collect()
}

/// After a task fires, advance its next_fire (or disable if one-shot).
fn advance_trigger(task: &mut KeeperTask) {
    match &task.trigger {
        KeeperTrigger::Interval { every_secs } => {
            task.next_fire = Instant::now() + Duration::from_secs(*every_secs);
        }
        KeeperTrigger::Once => {
            task.enabled = false;
        }
    }
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Execute a single keeper task: build instruction, sign, submit.
async fn execute_task(
    task: &KeeperTask,
    rpc: &SolanaRpc,
    keypair: &solana_sdk::signature::Keypair,
) -> KeeperExecution {
    let start = Instant::now();
    let unix_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let ix = Instruction {
        program_id: task.target_program,
        accounts: task.accounts.clone(),
        data: task.instruction_data.clone(),
    };

    match rpc.sign_and_send(keypair, vec![ix]).await {
        Ok(sig) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            info!(
                task = %task.name,
                signature = %sig,
                latency_ms,
                "keeper task executed"
            );
            KeeperExecution {
                task_id: task.id.to_string(),
                task_name: task.name.clone(),
                tx_signature: Some(sig.to_string()),
                success: true,
                error: None,
                latency_ms,
                timestamp: unix_now,
            }
        }
        Err(e) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            warn!(
                task = %task.name,
                error = %e,
                latency_ms,
                "keeper task failed"
            );
            KeeperExecution {
                task_id: task.id.to_string(),
                task_name: task.name.clone(),
                tx_signature: None,
                success: false,
                error: Some(e.to_string()),
                latency_ms,
                timestamp: unix_now,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Main keeper loop
// ---------------------------------------------------------------------------

/// The keeper evaluation loop. Spawned as a `tokio::spawn` task in main.rs.
///
/// Every `interval_secs`, evaluates all triggers and executes due tasks.
/// Completely independent of the VRF state machine.
pub async fn keeper_loop(
    state: KeeperState,
    on_chain: OnChainCtx,
    metrics: Metrics,
    interval_secs: u64,
) {
    info!(
        interval_secs,
        "keeper loop started"
    );

    let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;

        let mut mgr = state.lock().await;

        // Find tasks due to fire.
        let due_indices = evaluate_triggers(&mgr.tasks);
        if due_indices.is_empty() {
            drop(mgr);
            continue;
        }

        // Check concurrency limit.
        let available = mgr.max_concurrent.saturating_sub(mgr.running);
        if available == 0 {
            debug!("keeper at max concurrency, skipping tick");
            drop(mgr);
            continue;
        }

        // Collect task data for execution (can't hold lock during async RPC calls).
        let tasks_to_run: Vec<(usize, KeeperTask)> = due_indices
            .into_iter()
            .take(available as usize)
            .map(|i| (i, mgr.tasks[i].clone()))
            .collect();

        // Advance triggers immediately so they don't re-fire next tick.
        for &(idx, _) in &tasks_to_run {
            advance_trigger(&mut mgr.tasks[idx]);
        }
        mgr.running += tasks_to_run.len() as u32;
        drop(mgr);

        // Execute tasks concurrently (outside the lock).
        let rpc = on_chain.rpc.clone();
        let keypair = on_chain.keypair.clone();
        let state2 = state.clone();
        let metrics2 = metrics.clone();

        for (_idx, task) in tasks_to_run {
            let rpc = rpc.clone();
            let keypair = keypair.clone();
            let state3 = state2.clone();
            let metrics3 = metrics2.clone();

            tokio::spawn(async move {
                let result = execute_task(&task, &rpc, &keypair).await;

                // Record result back into shared state.
                let mut mgr = state3.lock().await;
                mgr.running = mgr.running.saturating_sub(1);

                // Update task stats.
                if let Some(t) = mgr.tasks.iter_mut().find(|t| t.id == task.id) {
                    t.total_executions += 1;
                    t.last_execution = Some(Instant::now());
                    if result.success {
                        t.last_tx_signature = result.tx_signature.clone();
                        t.last_error = None;
                        metrics3.keeper_executions_total.inc();
                    } else {
                        t.total_failures += 1;
                        t.last_error = result.error.clone();
                        metrics3.keeper_failures_total.inc();
                    }
                }

                // Push to history ring buffer.
                if mgr.history.len() >= MAX_HISTORY {
                    mgr.history.pop_front();
                }
                mgr.history.push_back(result);
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_trigger_fires_when_due() {
        let now = Instant::now();
        let tasks = vec![KeeperTask {
            id: Uuid::new_v4(),
            name: "test".into(),
            trigger: KeeperTrigger::Interval { every_secs: 10 },
            target_program: Pubkey::default(),
            instruction_data: vec![],
            accounts: vec![],
            enabled: true,
            created_at: now,
            next_fire: now - Duration::from_secs(1), // past due
            total_executions: 0,
            total_failures: 0,
            last_execution: None,
            last_tx_signature: None,
            last_error: None,
        }];

        let due = evaluate_triggers(&tasks);
        assert_eq!(due, vec![0]);
    }

    #[test]
    fn test_interval_trigger_does_not_fire_early() {
        let now = Instant::now();
        let tasks = vec![KeeperTask {
            id: Uuid::new_v4(),
            name: "test".into(),
            trigger: KeeperTrigger::Interval { every_secs: 10 },
            target_program: Pubkey::default(),
            instruction_data: vec![],
            accounts: vec![],
            enabled: true,
            created_at: now,
            next_fire: now + Duration::from_secs(5), // future
            total_executions: 0,
            total_failures: 0,
            last_execution: None,
            last_tx_signature: None,
            last_error: None,
        }];

        let due = evaluate_triggers(&tasks);
        assert!(due.is_empty());
    }

    #[test]
    fn test_disabled_task_does_not_fire() {
        let now = Instant::now();
        let tasks = vec![KeeperTask {
            id: Uuid::new_v4(),
            name: "test".into(),
            trigger: KeeperTrigger::Interval { every_secs: 10 },
            target_program: Pubkey::default(),
            instruction_data: vec![],
            accounts: vec![],
            enabled: false, // disabled
            created_at: now,
            next_fire: now - Duration::from_secs(1),
            total_executions: 0,
            total_failures: 0,
            last_execution: None,
            last_tx_signature: None,
            last_error: None,
        }];

        let due = evaluate_triggers(&tasks);
        assert!(due.is_empty());
    }

    #[test]
    fn test_once_trigger_disables_after_advance() {
        let mut task = KeeperTask {
            id: Uuid::new_v4(),
            name: "one-shot".into(),
            trigger: KeeperTrigger::Once,
            target_program: Pubkey::default(),
            instruction_data: vec![],
            accounts: vec![],
            enabled: true,
            created_at: Instant::now(),
            next_fire: Instant::now(),
            total_executions: 0,
            total_failures: 0,
            last_execution: None,
            last_tx_signature: None,
            last_error: None,
        };

        advance_trigger(&mut task);
        assert!(!task.enabled, "once trigger should disable after firing");
    }

    #[test]
    fn test_interval_advance_sets_future_fire() {
        let mut task = KeeperTask {
            id: Uuid::new_v4(),
            name: "interval".into(),
            trigger: KeeperTrigger::Interval { every_secs: 30 },
            target_program: Pubkey::default(),
            instruction_data: vec![],
            accounts: vec![],
            enabled: true,
            created_at: Instant::now(),
            next_fire: Instant::now(),
            total_executions: 0,
            total_failures: 0,
            last_execution: None,
            last_tx_signature: None,
            last_error: None,
        };

        let before = Instant::now();
        advance_trigger(&mut task);
        assert!(task.next_fire > before);
        assert!(task.enabled);
    }

    #[test]
    fn test_register_and_list_tasks() {
        let mut mgr = KeeperManager {
            tasks: Vec::new(),
            history: VecDeque::new(),
            running: 0,
            max_concurrent: 5,
        };

        let id = register_task(
            &mut mgr,
            "test-task".into(),
            KeeperTrigger::Interval { every_secs: 10 },
            Pubkey::default(),
            vec![1, 2, 3],
            vec![],
        );

        let list = list_tasks(&mgr);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id.to_string());
        assert_eq!(list[0].name, "test-task");
        assert!(list[0].enabled);
    }

    #[test]
    fn test_toggle_task() {
        let mut mgr = KeeperManager {
            tasks: Vec::new(),
            history: VecDeque::new(),
            running: 0,
            max_concurrent: 5,
        };

        let id = register_task(
            &mut mgr,
            "toggle-me".into(),
            KeeperTrigger::Interval { every_secs: 10 },
            Pubkey::default(),
            vec![],
            vec![],
        );

        let state = toggle_task(&mut mgr, id).unwrap();
        assert!(!state); // was true, now false

        let state = toggle_task(&mut mgr, id).unwrap();
        assert!(state); // back to true
    }

    #[test]
    fn test_remove_task() {
        let mut mgr = KeeperManager {
            tasks: Vec::new(),
            history: VecDeque::new(),
            running: 0,
            max_concurrent: 5,
        };

        let id = register_task(
            &mut mgr,
            "remove-me".into(),
            KeeperTrigger::Once,
            Pubkey::default(),
            vec![],
            vec![],
        );

        assert_eq!(mgr.tasks.len(), 1);
        remove_task(&mut mgr, id).unwrap();
        assert!(mgr.tasks.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_task_errors() {
        let mut mgr = KeeperManager {
            tasks: Vec::new(),
            history: VecDeque::new(),
            running: 0,
            max_concurrent: 5,
        };

        assert!(remove_task(&mut mgr, Uuid::new_v4()).is_err());
    }

    #[test]
    fn test_history_ring_buffer() {
        let mut mgr = KeeperManager {
            tasks: Vec::new(),
            history: VecDeque::with_capacity(MAX_HISTORY),
            running: 0,
            max_concurrent: 5,
        };

        // Fill past capacity.
        for i in 0..(MAX_HISTORY + 10) {
            if mgr.history.len() >= MAX_HISTORY {
                mgr.history.pop_front();
            }
            mgr.history.push_back(KeeperExecution {
                task_id: Uuid::new_v4().to_string(),
                task_name: format!("task-{}", i),
                tx_signature: None,
                success: true,
                error: None,
                latency_ms: 100,
                timestamp: i as u64,
            });
        }

        assert_eq!(mgr.history.len(), MAX_HISTORY);
        // Oldest should be from index 10 (first 10 were evicted).
        assert_eq!(mgr.history.front().unwrap().task_name, "task-10");
    }

    #[test]
    fn test_compute_stats() {
        let mut mgr = KeeperManager {
            tasks: Vec::new(),
            history: VecDeque::new(),
            running: 0,
            max_concurrent: 5,
        };

        register_task(
            &mut mgr,
            "t1".into(),
            KeeperTrigger::Interval { every_secs: 10 },
            Pubkey::default(),
            vec![],
            vec![],
        );

        mgr.tasks[0].total_executions = 10;
        mgr.tasks[0].total_failures = 2;

        let stats = compute_stats(&mgr);
        assert_eq!(stats.total_tasks, 1);
        assert_eq!(stats.total_executions, 10);
        assert_eq!(stats.total_failures, 2);
        assert!((stats.success_rate - 0.8).abs() < 0.001);
    }
}
