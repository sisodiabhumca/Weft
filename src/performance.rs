//! Performance optimization and monitoring utilities
//! 
//! This module provides performance-critical utilities and optimizations
//! for the Weft terminal to ensure smooth operation under heavy load.

use anyhow::Result;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub struct PerformanceManager {
    config: Arc<crate::config::Config>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    task_scheduler: Arc<RwLock<TaskScheduler>>,
    memory_pool: Arc<RwLock<MemoryPool>>,
    event_tx: mpsc::UnboundedSender<PerformanceEvent>,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<PerformanceEvent>>>>,
}

#[derive(Debug, Clone)]
pub enum PerformanceEvent {
    TaskScheduled { task_id: String, priority: TaskPriority, estimated_duration: Duration },
    TaskCompleted { task_id: String, actual_duration: Duration, success: bool },
    MemoryAllocated { size: usize, pool: String },
    MemoryDeallocated { size: usize, pool: String },
    FrameRendered { duration: Duration, frame_time: Duration },
    CommandProcessed { duration: Duration, complexity: CommandComplexity },
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub frame_times: VecDeque<Duration>,
    pub command_times: VecDeque<Duration>,
    pub memory_usage: VecDeque<MemorySnapshot>,
    pub cpu_usage: VecDeque<f64>,
    pub gpu_usage: VecDeque<f64>,
    pub network_latency: VecDeque<Duration>,
    pub task_queue_sizes: VecDeque<usize>,
    pub max_history_size: usize,
}

#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub timestamp: Instant,
    pub total_allocated: usize,
    pub total_freed: usize,
    pub current_usage: usize,
    pub peak_usage: usize,
    pub fragmentation_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct TaskScheduler {
    pub task_queue: VecDeque<ScheduledTask>,
    pub running_tasks: Vec<RunningTask>,
    pub completed_tasks: Vec<CompletedTask>,
    pub max_concurrent_tasks: usize,
    pub cpu_cores: usize,
}

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: String,
    pub priority: TaskPriority,
    pub task_type: TaskType,
    pub created_at: Instant,
    pub estimated_duration: Duration,
    pub dependencies: Vec<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone)]
pub struct RunningTask {
    pub id: String,
    pub started_at: Instant,
    pub estimated_duration: Duration,
    pub progress: f32,
    pub worker_id: usize,
}

#[derive(Debug, Clone)]
pub struct CompletedTask {
    pub id: String,
    pub started_at: Instant,
    pub completed_at: Instant,
    pub actual_duration: Duration,
    pub success: bool,
    pub worker_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical = 4,
    High = 3,
    Normal = 2,
    Low = 1,
    Background = 0,
}

#[derive(Debug, Clone)]
pub enum TaskType {
    RenderFrame,
    ProcessCommand,
    UpdateAI,
    HandleNetwork,
    ProcessPlugin,
    UpdateUI,
    GarbageCollect,
    BackgroundSync,
}

#[derive(Debug, Clone)]
pub enum CommandComplexity {
    Simple,     // Basic commands like ls, cd, echo
    Medium,     // Commands with pipes, redirects
    Complex,    // Commands with multiple processes, heavy I/O
    Heavy,      // Resource-intensive commands
}

#[derive(Debug, Clone)]
pub struct MemoryPool {
    pub pools: Vec<MemoryPoolEntry>,
    pub allocation_strategy: AllocationStrategy,
    pub defragmentation_threshold: f64,
    pub max_pool_size: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryPoolEntry {
    pub name: String,
    pub block_size: usize,
    pub blocks: VecDeque<MemoryBlock>,
    pub allocated_blocks: usize,
    pub total_blocks: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub address: usize,
    pub size: usize,
    pub allocated: bool,
    pub last_used: Instant,
}

#[derive(Debug, Clone)]
pub enum AllocationStrategy {
    FirstFit,
    BestFit,
    WorstFit,
    BuddySystem,
}

#[derive(Debug, Clone)]
pub struct RenderOptimizer {
    pub frame_budget: Duration,
    pub target_fps: u32,
    pub adaptive_quality: bool,
    pub vsync_enabled: bool,
    pub frame_skip_threshold: Duration,
}

#[derive(Debug, Clone)]
pub struct CommandOptimizer {
    pub command_cache: Arc<RwLock<CommandCache>>,
    pub prediction_engine: Arc<RwLock<CommandPredictionEngine>>,
    pub parallel_execution: bool,
    pub max_parallel_commands: usize,
}

#[derive(Debug, Clone)]
pub struct CommandCache {
    pub entries: VecDeque<CachedCommand>,
    pub max_size: usize,
    pub hit_count: u64,
    pub miss_count: u64,
}

#[derive(Debug, Clone)]
pub struct CachedCommand {
    pub command: String,
    pub result: CommandResult,
    pub timestamp: Instant,
    pub access_count: u64,
    pub ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct CommandPredictionEngine {
    pub patterns: Vec<CommandPattern>,
    pub learning_enabled: bool,
    pub prediction_accuracy: f64,
}

#[derive(Debug, Clone)]
pub struct CommandPattern {
    pub pattern: String,
    pub frequency: f64,
    pub average_duration: Duration,
    pub success_rate: f64,
    pub context: Vec<String>,
}

impl PerformanceManager {
    pub fn new(config: &Arc<crate::config::Config>) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            config: config.clone(),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            task_scheduler: Arc::new(RwLock::new(TaskScheduler::default())),
            memory_pool: Arc::new(RwLock::new(MemoryPool::default())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing performance manager");
        
        // Initialize task scheduler
        self.initialize_task_scheduler().await?;
        
        // Initialize memory pool
        self.initialize_memory_pool().await?;
        
        // Start performance monitoring
        self.start_performance_monitoring().await?;
        
        info!("Performance manager initialized");
        Ok(())
    }

    async fn initialize_task_scheduler(&self) -> Result<()> {
        info!("Initializing task scheduler");
        
        let cpu_cores = num_cpus::get();
        let mut scheduler = self.task_scheduler.write();
        scheduler.cpu_cores = cpu_cores;
        scheduler.max_concurrent_tasks = cpu_cores * 2; // Allow oversubscription
        
        info!("Task scheduler initialized with {} cores, max {} concurrent tasks", 
              cpu_cores, scheduler.max_concurrent_tasks);
        
        Ok(())
    }

    async fn initialize_memory_pool(&self) -> Result<()> {
        info!("Initializing memory pool");
        
        let mut pool = self.memory_pool.write();
        
        // Create pools for different allocation sizes
        pool.pools = vec![
            MemoryPoolEntry {
                name: "small".to_string(),
                block_size: 64,
                blocks: VecDeque::new(),
                allocated_blocks: 0,
                total_blocks: 1000,
            },
            MemoryPoolEntry {
                name: "medium".to_string(),
                block_size: 1024,
                blocks: VecDeque::new(),
                allocated_blocks: 0,
                total_blocks: 500,
            },
            MemoryPoolEntry {
                name: "large".to_string(),
                block_size: 16384,
                blocks: VecDeque::new(),
                allocated_blocks: 0,
                total_blocks: 100,
            },
        ];
        
        pool.allocation_strategy = AllocationStrategy::BestFit;
        pool.defragmentation_threshold = 0.3;
        pool.max_pool_size = 10 * 1024 * 1024; // 10MB
        
        info!("Memory pool initialized with {} pools", pool.pools.len());
        Ok(())
    }

    async fn start_performance_monitoring(&self) -> Result<()> {
        info!("Starting performance monitoring");
        
        let metrics = self.metrics.clone();
        let event_tx = self.event_tx.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                // Collect performance metrics
                let frame_time = Self::measure_frame_time();
                let memory_usage = Self::measure_memory_usage();
                let cpu_usage = Self::measure_cpu_usage();
                
                // Update metrics
                {
                    let mut metrics = metrics.write();
                    metrics.add_frame_time(frame_time);
                    metrics.add_memory_usage(memory_usage);
                    metrics.add_cpu_usage(cpu_usage);
                    
                    // Trim history if needed
                    metrics.trim_history();
                }
                
                // Send events if thresholds are exceeded
                if frame_time > Duration::from_millis(16) { // > 60 FPS
                    if let Err(e) = event_tx.send(PerformanceEvent::FrameRendered {
                        duration: frame_time,
                        frame_time,
                    }) {
                        warn!("Failed to send frame render event: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }

    pub async fn process_events(&self) -> Result<()> {
        let mut event_rx = self.event_rx.write().take().unwrap();
        
        while let Some(event) = event_rx.recv().await {
            debug!("Processing performance event: {:?}", event);
            
            match event {
                PerformanceEvent::TaskScheduled { task_id, priority, estimated_duration } => {
                    self.handle_task_scheduled(task_id, priority, estimated_duration).await?;
                }
                PerformanceEvent::TaskCompleted { task_id, actual_duration, success } => {
                    self.handle_task_completed(task_id, actual_duration, success).await?;
                }
                PerformanceEvent::MemoryAllocated { size, pool } => {
                    self.handle_memory_allocated(size, pool).await?;
                }
                PerformanceEvent::MemoryDeallocated { size, pool } => {
                    self.handle_memory_deallocated(size, pool).await?;
                }
                PerformanceEvent::FrameRendered { duration, frame_time } => {
                    self.handle_frame_rendered(duration, frame_time).await?;
                }
                PerformanceEvent::CommandProcessed { duration, complexity } => {
                    self.handle_command_processed(duration, complexity).await?;
                }
            }
        }

        // Put the receiver back
        *self.event_rx.write() = Some(event_rx);
        Ok(())
    }

    async fn handle_task_scheduled(&self, task_id: String, priority: TaskPriority, estimated_duration: Duration) -> Result<()> {
        debug!("Task scheduled: {} (priority: {:?}, estimated: {:?})", task_id, priority, estimated_duration);
        
        let mut scheduler = self.task_scheduler.write();
        scheduler.schedule_task(task_id, priority, TaskType::ProcessCommand, estimated_duration);
        
        Ok(())
    }

    async fn handle_task_completed(&self, task_id: String, actual_duration: Duration, success: bool) -> Result<()> {
        debug!("Task completed: {} (duration: {:?}, success: {})", task_id, actual_duration, success);
        
        let mut scheduler = self.task_scheduler.write();
        scheduler.complete_task(task_id, actual_duration, success);
        
        Ok(())
    }

    async fn handle_memory_allocated(&self, size: usize, pool: String) -> Result<()> {
        debug!("Memory allocated: {} bytes from pool '{}'", size, pool);
        
        let mut memory_pool = self.memory_pool.write();
        memory_pool.allocate(size, &pool);
        
        Ok(())
    }

    async fn handle_memory_deallocated(&self, size: usize, pool: String) -> Result<()> {
        debug!("Memory deallocated: {} bytes to pool '{}'", size, pool);
        
        let mut memory_pool = self.memory_pool.write();
        memory_pool.deallocate(size, &pool);
        
        Ok(())
    }

    async fn handle_frame_rendered(&self, duration: Duration, frame_time: Duration) -> Result<()> {
        debug!("Frame rendered: duration={:?}, frame_time={:?}", duration, frame_time);
        
        // Update frame time metrics
        let mut metrics = self.metrics.write();
        metrics.add_frame_time(frame_time);
        
        // Check if we need to optimize rendering
        if frame_time > Duration::from_millis(16) {
            warn!("Frame time exceeded budget: {:?}", frame_time);
            // TODO: Trigger render optimization
        }
        
        Ok(())
    }

    async fn handle_command_processed(&self, duration: Duration, complexity: CommandComplexity) -> Result<()> {
        debug!("Command processed: duration={:?}, complexity={:?}", duration, complexity);
        
        // Update command time metrics
        let mut metrics = self.metrics.write();
        metrics.add_command_time(duration);
        
        Ok(())
    }

    pub fn schedule_task(&self, task_id: String, priority: TaskPriority, task_type: TaskType, estimated_duration: Duration) -> Result<()> {
        if let Err(e) = self.event_tx.send(PerformanceEvent::TaskScheduled {
            task_id: task_id.clone(),
            priority,
            estimated_duration,
        }) {
            warn!("Failed to send task scheduled event: {}", e);
        }
        Ok(())
    }

    pub fn allocate_memory(&self, size: usize, pool: &str) -> Result<usize> {
        let mut memory_pool = self.memory_pool.write();
        let address = memory_pool.allocate(size, pool)?;
        
        if let Err(e) = self.event_tx.send(PerformanceEvent::MemoryAllocated {
            size,
            pool: pool.to_string(),
        }) {
            warn!("Failed to send memory allocated event: {}", e);
        }
        
        Ok(address)
    }

    pub fn deallocate_memory(&self, address: usize, size: usize, pool: &str) -> Result<()> {
        let mut memory_pool = self.memory_pool.write();
        memory_pool.deallocate_at_address(address, size, pool)?;
        
        if let Err(e) = self.event_tx.send(PerformanceEvent::MemoryDeallocated {
            size,
            pool: pool.to_string(),
        }) {
            warn!("Failed to send memory deallocated event: {}", e);
        }
        
        Ok(())
    }

    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().clone()
    }

    pub fn get_task_scheduler_status(&self) -> TaskScheduler {
        self.task_scheduler.read().clone()
    }

    pub fn get_memory_pool_status(&self) -> MemoryPool {
        self.memory_pool.read().clone()
    }

    fn measure_frame_time() -> Duration {
        // In a real implementation, this would measure actual frame rendering time
        Duration::from_millis(10)
    }

    fn measure_memory_usage() -> MemorySnapshot {
        // In a real implementation, this would measure actual memory usage
        MemorySnapshot {
            timestamp: Instant::now(),
            total_allocated: 512 * 1024 * 1024, // 512MB
            total_freed: 256 * 1024 * 1024,    // 256MB
            current_usage: 256 * 1024 * 1024,  // 256MB
            peak_usage: 300 * 1024 * 1024,     // 300MB
            fragmentation_ratio: 0.15,
        }
    }

    fn measure_cpu_usage() -> f64 {
        // In a real implementation, this would measure actual CPU usage
        0.25 // 25%
    }

    pub fn optimize_rendering(&self) -> Result<()> {
        info!("Optimizing rendering performance");
        
        // TODO: Implement render optimization strategies
        // - Reduce rendering quality if frame times are high
        // - Enable frame skipping
        // - Adjust vsync settings
        
        Ok(())
    }

    pub fn optimize_memory(&self) -> Result<()> {
        info!("Optimizing memory usage");
        
        let mut memory_pool = self.memory_pool.write();
        
        // Check if defragmentation is needed
        if memory_pool.get_fragmentation_ratio() > memory_pool.defragmentation_threshold {
            memory_pool.defragment()?;
        }
        
        // Clean up unused memory blocks
        memory_pool.cleanup()?;
        
        Ok(())
    }

    pub fn optimize_task_scheduling(&self) -> Result<()> {
        info!("Optimizing task scheduling");
        
        let mut scheduler = self.task_scheduler.write();
        
        // Rebalance task priorities
        scheduler.rebalance_priorities();
        
        // Cancel low-priority tasks if queue is full
        scheduler.cancel_low_priority_tasks();
        
        Ok(())
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(1000),
            command_times: VecDeque::with_capacity(1000),
            memory_usage: VecDeque::with_capacity(1000),
            cpu_usage: VecDeque::with_capacity(1000),
            gpu_usage: VecDeque::with_capacity(1000),
            network_latency: VecDeque::with_capacity(1000),
            task_queue_sizes: VecDeque::with_capacity(1000),
            max_history_size: 1000,
        }
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self {
            task_queue: VecDeque::new(),
            running_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            max_concurrent_tasks: 4,
            cpu_cores: 4,
        }
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self {
            pools: Vec::new(),
            allocation_strategy: AllocationStrategy::BestFit,
            defragmentation_threshold: 0.3,
            max_pool_size: 10 * 1024 * 1024,
        }
    }
}

impl PerformanceMetrics {
    pub fn add_frame_time(&mut self, frame_time: Duration) {
        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > self.max_history_size {
            self.frame_times.pop_front();
        }
    }

    pub fn add_command_time(&mut self, command_time: Duration) {
        self.command_times.push_back(command_time);
        if self.command_times.len() > self.max_history_size {
            self.command_times.pop_front();
        }
    }

    pub fn add_memory_usage(&mut self, usage: MemorySnapshot) {
        self.memory_usage.push_back(usage);
        if self.memory_usage.len() > self.max_history_size {
            self.memory_usage.pop_front();
        }
    }

    pub fn add_cpu_usage(&mut self, cpu_usage: f64) {
        self.cpu_usage.push_back(cpu_usage);
        if self.cpu_usage.len() > self.max_history_size {
            self.cpu_usage.pop_front();
        }
    }

    pub fn trim_history(&mut self) {
        if self.frame_times.len() > self.max_history_size {
            let excess = self.frame_times.len() - self.max_history_size;
            for _ in 0..excess {
                self.frame_times.pop_front();
            }
        }
        
        if self.command_times.len() > self.max_history_size {
            let excess = self.command_times.len() - self.max_history_size;
            for _ in 0..excess {
                self.command_times.pop_front();
            }
        }
        
        if self.memory_usage.len() > self.max_history_size {
            let excess = self.memory_usage.len() - self.max_history_size;
            for _ in 0..excess {
                self.memory_usage.pop_front();
            }
        }
        
        if self.cpu_usage.len() > self.max_history_size {
            let excess = self.cpu_usage.len() - self.max_history_size;
            for _ in 0..excess {
                self.cpu_usage.pop_front();
            }
        }
    }

    pub fn get_average_frame_time(&self) -> Duration {
        if self.frame_times.is_empty() {
            return Duration::ZERO;
        }
        
        let total: Duration = self.frame_times.iter().sum();
        total / self.frame_times.len() as u32
    }

    pub fn get_average_command_time(&self) -> Duration {
        if self.command_times.is_empty() {
            return Duration::ZERO;
        }
        
        let total: Duration = self.command_times.iter().sum();
        total / self.command_times.len() as u32
    }

    pub fn get_current_memory_usage(&self) -> Option<usize> {
        self.memory_usage.back().map(|usage| usage.current_usage)
    }

    pub fn get_current_cpu_usage(&self) -> Option<f64> {
        self.cpu_usage.back().copied()
    }
}

impl TaskScheduler {
    pub fn schedule_task(&mut self, task_id: String, priority: TaskPriority, task_type: TaskType, estimated_duration: Duration) {
        let task = ScheduledTask {
            id: task_id,
            priority,
            task_type,
            created_at: Instant::now(),
            estimated_duration,
            dependencies: Vec::new(),
            retry_count: 0,
        };
        
        // Insert task in priority order
        let insert_pos = self.task_queue.iter()
            .position(|t| t.priority < task.priority)
            .unwrap_or(self.task_queue.len());
        
        self.task_queue.insert(insert_pos, task);
    }

    pub fn complete_task(&mut self, task_id: String, actual_duration: Duration, success: bool) {
        if let Some(pos) = self.running_tasks.iter().position(|t| t.id == task_id) {
            let running_task = self.running_tasks.remove(pos);
            
            let completed_task = CompletedTask {
                id: task_id,
                started_at: Instant::now() - actual_duration,
                completed_at: Instant::now(),
                actual_duration,
                success,
                worker_id: running_task.worker_id,
            };
            
            self.completed_tasks.push(completed_task);
            
            // Trim completed tasks history
            if self.completed_tasks.len() > 1000 {
                self.completed_tasks.drain(0..500);
            }
        }
    }

    pub fn rebalance_priorities(&mut self) {
        // Sort task queue by priority
        self.task_queue.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn cancel_low_priority_tasks(&mut self) {
        if self.task_queue.len() > self.max_concurrent_tasks * 2 {
            let tasks_to_remove = self.task_queue.len() - self.max_concurrent_tasks * 2;
            
            // Remove lowest priority tasks
            self.task_queue.truncate(self.task_queue.len() - tasks_to_remove);
        }
    }

    pub fn get_next_task(&mut self) -> Option<ScheduledTask> {
        if self.running_tasks.len() < self.max_concurrent_tasks {
            self.task_queue.pop_front()
        } else {
            None
        }
    }
}

impl MemoryPool {
    pub fn allocate(&mut self, size: usize, pool_name: &str) -> Result<usize> {
        let pool = self.pools.iter_mut()
            .find(|p| p.name == pool_name)
            .ok_or_else(|| anyhow::anyhow!("Pool '{}' not found", pool_name))?;
        
        if pool.allocated_blocks >= pool.total_blocks {
            return Err(anyhow::anyhow!("Pool '{}' is full", pool_name));
        }
        
        // Find a free block
        if let Some(block) = pool.blocks.iter_mut().find(|b| !b.allocated && b.size >= size) {
            block.allocated = true;
            block.last_used = Instant::now();
            pool.allocated_blocks += 1;
            return Ok(block.address);
        }
        
        // Create new block if possible
        if pool.blocks.len() < pool.total_blocks {
            let address = pool.blocks.len() * pool.block_size;
            let block = MemoryBlock {
                address,
                size: pool.block_size,
                allocated: true,
                last_used: Instant::now(),
            };
            
            pool.blocks.push_back(block);
            pool.allocated_blocks += 1;
            return Ok(address);
        }
        
        Err(anyhow::anyhow!("No available blocks in pool '{}'", pool_name))
    }

    pub fn deallocate(&mut self, size: usize, pool_name: &str) {
        let pool = if let Some(p) = self.pools.iter_mut().find(|p| p.name == pool_name) {
            p
        } else {
            return;
        };
        
        // Find and deallocate block
        if let Some(block) = pool.blocks.iter_mut().find(|b| b.allocated && b.size == size) {
            block.allocated = false;
            pool.allocated_blocks -= 1;
        }
    }

    pub fn deallocate_at_address(&mut self, address: usize, size: usize, pool_name: &str) -> Result<()> {
        let pool = self.pools.iter_mut()
            .find(|p| p.name == pool_name)
            .ok_or_else(|| anyhow::anyhow!("Pool '{}' not found", pool_name))?;
        
        if let Some(block) = pool.blocks.iter_mut().find(|b| b.address == address) {
            if block.allocated && block.size == size {
                block.allocated = false;
                pool.allocated_blocks -= 1;
                return Ok(());
            }
        }
        
        Err(anyhow::anyhow!("Block not found or mismatch"))
    }

    pub fn get_fragmentation_ratio(&self) -> f64 {
        let total_blocks: usize = self.pools.iter().map(|p| p.total_blocks).sum();
        let allocated_blocks: usize = self.pools.iter().map(|p| p.allocated_blocks).sum();
        
        if total_blocks == 0 {
            0.0
        } else {
            1.0 - (allocated_blocks as f64 / total_blocks as f64)
        }
    }

    pub fn defragment(&mut self) -> Result<()> {
        info!("Starting memory pool defragmentation");
        
        for pool in &mut self.pools {
            // Move allocated blocks to the front
            let mut allocated_blocks: Vec<_> = pool.blocks.drain(..)
                .filter(|b| b.allocated)
                .collect();
            
            let mut free_blocks: Vec<_> = pool.blocks.drain(..)
                .filter(|b| !b.allocated)
                .collect();
            
            // Rebuild pool with allocated blocks first
            pool.blocks.extend(allocated_blocks);
            pool.blocks.extend(free_blocks);
        }
        
        info!("Memory pool defragmentation completed");
        Ok(())
    }

    pub fn cleanup(&mut self) -> Result<()> {
        info!("Cleaning up memory pool");
        
        for pool in &mut self.pools {
            // Remove very old free blocks
            let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes
            
            pool.blocks.retain(|block| {
                block.allocated || block.last_used > cutoff
            });
            
            // Update total blocks count
            pool.total_blocks = pool.blocks.len();
            pool.allocated_blocks = pool.blocks.iter().filter(|b| b.allocated).count();
        }
        
        info!("Memory pool cleanup completed");
        Ok(())
    }
}
