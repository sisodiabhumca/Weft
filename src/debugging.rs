//! Advanced debugging and profiling tools
//! 
//! This module provides comprehensive debugging capabilities including
//! performance monitoring, memory profiling, and command tracing.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub struct DebuggingEngine {
    config: Arc<crate::config::Config>,
    profiler: Arc<RwLock<Profiler>>,
    tracer: Arc<RwLock<CommandTracer>>,
    performance_monitor: Arc<RwLock<PerformanceMonitor>>,
    event_tx: mpsc::UnboundedSender<DebugEvent>,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<DebugEvent>>>>,
}

#[derive(Debug, Clone)]
pub enum DebugEvent {
    CommandStarted { session_id: String, command: String, timestamp: chrono::DateTime<chrono::Utc> },
    CommandCompleted { session_id: String, command: String, duration: Duration, exit_code: i32 },
    MemorySnapshot { timestamp: chrono::DateTime<chrono::Utc>, usage: MemoryUsage },
    PerformanceMetric { metric_type: MetricType, value: f64, timestamp: chrono::DateTime<chrono::Utc> },
    NetworkActivity { direction: NetworkDirection, bytes: usize, endpoint: String },
    ErrorOccurred { error: String, context: String, timestamp: chrono::DateTime<chrono::Utc> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profiler {
    pub enabled: bool,
    pub sampling_rate: f64,
    pub max_samples: usize,
    pub samples: Vec<ProfileSample>,
    pub function_stats: HashMap<String, FunctionStats>,
    pub memory_samples: Vec<MemorySample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSample {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stack_trace: Vec<String>,
    pub memory_usage: MemoryUsage,
    pub cpu_usage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionStats {
    pub call_count: u64,
    pub total_time: Duration,
    pub average_time: Duration,
    pub max_time: Duration,
    pub min_time: Duration,
    pub self_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySample {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub heap_size: usize,
    pub stack_size: usize,
    pub allocations: u64,
    pub deallocations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub total: usize,
    pub used: usize,
    pub free: usize,
    pub heap: usize,
    pub stack: usize,
}

#[derive(Debug, Clone)]
pub struct CommandTracer {
    pub enabled: bool,
    pub trace_level: TraceLevel,
    pub max_entries: usize,
    pub entries: Vec<TraceEntry>,
    pub filters: Vec<TraceFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceLevel {
    Off,
    Basic,
    Detailed,
    Verbose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: TraceLevel,
    pub session_id: String,
    pub command: String,
    pub input: String,
    pub output: String,
    pub exit_code: i32,
    pub duration: Duration,
    pub environment: HashMap<String, String>,
    pub working_directory: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceFilter {
    pub field: String,
    pub pattern: String,
    pub include: bool,
}

#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    pub enabled: bool,
    pub metrics: HashMap<String, MetricValue>,
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub histograms: HashMap<String, Histogram>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub value: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub buckets: Vec<f64>,
    pub counts: Vec<u64>,
    pub sum: f64,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    CpuUsage,
    MemoryUsage,
    DiskUsage,
    NetworkLatency,
    CommandLatency,
    RenderFps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub session_id: String,
    pub performance_summary: PerformanceSummary,
    pub memory_summary: MemorySummary,
    pub command_summary: CommandSummary,
    pub error_summary: ErrorSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub total_commands: u64,
    pub average_command_time: Duration,
    pub cpu_peak: f64,
    pub memory_peak: usize,
    pub render_fps_average: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub initial_usage: usize,
    pub peak_usage: usize,
    pub final_usage: usize,
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub leaks_detected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSummary {
    pub total_commands: u64,
    pub successful_commands: u64,
    pub failed_commands: u64,
    pub most_common_commands: Vec<(String, u64)>,
    pub slowest_commands: Vec<(String, Duration)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSummary {
    pub total_errors: u64,
    pub error_types: HashMap<String, u64>,
    pub recent_errors: Vec<String>,
}

impl DebuggingEngine {
    pub fn new(config: &Arc<crate::config::Config>) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            config: config.clone(),
            profiler: Arc::new(RwLock::new(Profiler::default())),
            tracer: Arc::new(RwLock::new(CommandTracer::default())),
            performance_monitor: Arc::new(RwLock::new(PerformanceMonitor::default())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing debugging engine");
        
        if !self.config.debugging.enabled {
            info!("Debugging features are disabled");
            return Ok(());
        }

        // Start performance monitoring
        if self.config.debugging.performance_monitoring {
            self.start_performance_monitoring().await?;
        }

        // Start memory profiling
        if self.config.debugging.memory_profiling {
            self.start_memory_profiling().await?;
        }

        // Start command tracing
        if self.config.debugging.command_tracing {
            self.start_command_tracing().await?;
        }

        info!("Debugging engine initialized");
        Ok(())
    }

    async fn start_performance_monitoring(&self) -> Result<()> {
        info!("Starting performance monitoring");
        
        let monitor = self.performance_monitor.clone();
        let event_tx = self.event_tx.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                // Collect CPU usage
                if let Ok(cpu_usage) = Self::get_cpu_usage() {
                    if let Err(e) = event_tx.send(DebugEvent::PerformanceMetric {
                        metric_type: MetricType::CpuUsage,
                        value: cpu_usage,
                        timestamp: chrono::Utc::now(),
                    }) {
                        warn!("Failed to send CPU metric: {}", e);
                    }
                }
                
                // Collect memory usage
                if let Ok(memory_usage) = Self::get_memory_usage() {
                    if let Err(e) = event_tx.send(DebugEvent::MemorySnapshot {
                        timestamp: chrono::Utc::now(),
                        usage: memory_usage,
                    }) {
                        warn!("Failed to send memory snapshot: {}", e);
                    }
                }
                
                // Update monitor
                monitor.write().update_metrics();
            }
        });
        
        Ok(())
    }

    async fn start_memory_profiling(&self) -> Result<()> {
        info!("Starting memory profiling");
        
        let profiler = self.profiler.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                if profiler.read().enabled {
                    let sample = Self::collect_memory_sample();
                    profiler.write().add_memory_sample(sample);
                }
            }
        });
        
        Ok(())
    }

    async fn start_command_tracing(&self) -> Result<()> {
        info!("Starting command tracing");
        
        // Command tracing is event-driven, so no background task needed
        Ok(())
    }

    pub async fn process_events(&self) -> Result<()> {
        let mut event_rx = self.event_rx.write().take().unwrap();
        
        while let Some(event) = event_rx.recv().await {
            debug!("Processing debug event: {:?}", event);
            
            match event {
                DebugEvent::CommandStarted { session_id, command, timestamp } => {
                    self.handle_command_started(session_id, command, timestamp).await?;
                }
                DebugEvent::CommandCompleted { session_id, command, duration, exit_code } => {
                    self.handle_command_completed(session_id, command, duration, exit_code).await?;
                }
                DebugEvent::MemorySnapshot { timestamp, usage } => {
                    self.handle_memory_snapshot(timestamp, usage).await?;
                }
                DebugEvent::PerformanceMetric { metric_type, value, timestamp } => {
                    self.handle_performance_metric(metric_type, value, timestamp).await?;
                }
                DebugEvent::NetworkActivity { direction, bytes, endpoint } => {
                    self.handle_network_activity(direction, bytes, endpoint).await?;
                }
                DebugEvent::ErrorOccurred { error, context, timestamp } => {
                    self.handle_error_occurred(error, context, timestamp).await?;
                }
            }
        }

        // Put the receiver back
        *self.event_rx.write() = Some(event_rx);
        Ok(())
    }

    async fn handle_command_started(&self, session_id: String, command: String, timestamp: chrono::DateTime<chrono::Utc>) -> Result<()> {
        debug!("Command started in session {}: {}", session_id, command);
        
        // Add to tracer
        let mut tracer = self.tracer.write();
        if tracer.enabled {
            tracer.add_command_started(session_id, command, timestamp);
        }
        
        Ok(())
    }

    async fn handle_command_completed(&self, session_id: String, command: String, duration: Duration, exit_code: i32) -> Result<()> {
        debug!("Command completed in session {}: '{}' ({}ms, exit: {})", session_id, command, duration.as_millis(), exit_code);
        
        // Update tracer
        let mut tracer = self.tracer.write();
        if tracer.enabled {
            tracer.add_command_completed(session_id, command, duration, exit_code);
        }
        
        // Update performance monitor
        let mut monitor = self.performance_monitor.write();
        monitor.record_command_duration(&command, duration);
        
        Ok(())
    }

    async fn handle_memory_snapshot(&self, timestamp: chrono::DateTime<chrono::Utc>, usage: MemoryUsage) -> Result<()> {
        debug!("Memory snapshot: {} used / {} total", usage.used, usage.total);
        
        // Update profiler
        let mut profiler = self.profiler.write();
        if profiler.enabled {
            profiler.add_memory_sample(MemorySample {
                timestamp,
                heap_size: usage.heap,
                stack_size: usage.stack,
                allocations: 0, // Would need tracking
                deallocations: 0, // Would need tracking
            });
        }
        
        Ok(())
    }

    async fn handle_performance_metric(&self, metric_type: MetricType, value: f64, timestamp: chrono::DateTime<chrono::Utc>) -> Result<()> {
        debug!("Performance metric: {:?} = {}", metric_type, value);
        
        // Update performance monitor
        let mut monitor = self.performance_monitor.write();
        monitor.add_metric(metric_type, value, timestamp);
        
        Ok(())
    }

    async fn handle_network_activity(&self, direction: NetworkDirection, bytes: usize, endpoint: String) -> Result<()> {
        debug!("Network activity: {:?} {} bytes to/from {}", direction, bytes, endpoint);
        
        // Update performance monitor
        let mut monitor = self.performance_monitor.write();
        monitor.record_network_activity(direction, bytes, &endpoint);
        
        Ok(())
    }

    async fn handle_error_occurred(&self, error: String, context: String, timestamp: chrono::DateTime<chrono::Utc>) -> Result<()> {
        warn!("Error occurred in {}: {}", context, error);
        
        // Update tracer
        let mut tracer = self.tracer.write();
        if tracer.enabled {
            tracer.add_error(error, context, timestamp);
        }
        
        Ok(())
    }

    pub fn trace_command_start(&self, session_id: &str, command: &str) -> Result<()> {
        if let Err(e) = self.event_tx.send(DebugEvent::CommandStarted {
            session_id: session_id.to_string(),
            command: command.to_string(),
            timestamp: chrono::Utc::now(),
        }) {
            warn!("Failed to send command started event: {}", e);
        }
        Ok(())
    }

    pub fn trace_command_complete(&self, session_id: &str, command: &str, duration: Duration, exit_code: i32) -> Result<()> {
        if let Err(e) = self.event_tx.send(DebugEvent::CommandCompleted {
            session_id: session_id.to_string(),
            command: command.to_string(),
            duration,
            exit_code,
        }) {
            warn!("Failed to send command completed event: {}", e);
        }
        Ok(())
    }

    pub fn trace_error(&self, error: String, context: String) -> Result<()> {
        if let Err(e) = self.event_tx.send(DebugEvent::ErrorOccurred {
            error,
            context,
            timestamp: chrono::Utc::now(),
        }) {
            warn!("Failed to send error occurred event: {}", e);
        }
        Ok(())
    }

    pub fn generate_report(&self, session_id: &str) -> Result<DebugReport> {
        let tracer = self.tracer.read();
        let profiler = self.profiler.read();
        let monitor = self.performance_monitor.read();
        
        let performance_summary = PerformanceSummary {
            total_commands: tracer.get_command_count(),
            average_command_time: tracer.get_average_command_time(),
            cpu_peak: monitor.get_peak_cpu(),
            memory_peak: profiler.get_peak_memory(),
            render_fps_average: monitor.get_average_fps(),
        };
        
        let memory_summary = MemorySummary {
            initial_usage: profiler.get_initial_memory(),
            peak_usage: profiler.get_peak_memory(),
            final_usage: profiler.get_current_memory(),
            total_allocations: 0, // Would need tracking
            total_deallocations: 0, // Would need tracking
            leaks_detected: 0, // Would need analysis
        };
        
        let command_summary = CommandSummary {
            total_commands: tracer.get_command_count(),
            successful_commands: tracer.get_successful_command_count(),
            failed_commands: tracer.get_failed_command_count(),
            most_common_commands: tracer.get_most_common_commands(),
            slowest_commands: tracer.get_slowest_commands(),
        };
        
        let error_summary = ErrorSummary {
            total_errors: tracer.get_error_count(),
            error_types: tracer.get_error_types(),
            recent_errors: tracer.get_recent_errors(),
        };
        
        Ok(DebugReport {
            timestamp: chrono::Utc::now(),
            session_id: session_id.to_string(),
            performance_summary,
            memory_summary,
            command_summary,
            error_summary,
        })
    }

    fn get_cpu_usage() -> Result<f64> {
        // Simplified CPU usage calculation
        // In a real implementation, this would use system APIs
        Ok(0.0)
    }

    fn get_memory_usage() -> Result<MemoryUsage> {
        // Simplified memory usage calculation
        // In a real implementation, this would use system APIs
        Ok(MemoryUsage {
            total: 8 * 1024 * 1024 * 1024, // 8GB
            used: 512 * 1024 * 1024, // 512MB
            free: 7 * 1024 * 1024 * 1024,
            heap: 256 * 1024 * 1024, // 256MB
            stack: 8 * 1024 * 1024, // 8MB
        })
    }

    fn collect_memory_sample() -> MemorySample {
        // Simplified memory sample collection
        MemorySample {
            timestamp: chrono::Utc::now(),
            heap_size: 256 * 1024 * 1024,
            stack_size: 8 * 1024 * 1024,
            allocations: 1000,
            deallocations: 950,
        }
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self {
            enabled: false,
            sampling_rate: 1.0,
            max_samples: 10000,
            samples: Vec::new(),
            function_stats: HashMap::new(),
            memory_samples: Vec::new(),
        }
    }
}

impl Default for CommandTracer {
    fn default() -> Self {
        Self {
            enabled: false,
            trace_level: TraceLevel::Basic,
            max_entries: 10000,
            entries: Vec::new(),
            filters: Vec::new(),
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self {
            enabled: false,
            metrics: HashMap::new(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }
}

impl Profiler {
    pub fn add_memory_sample(&mut self, sample: MemorySample) {
        self.memory_samples.push(sample);
        if self.memory_samples.len() > self.max_samples {
            self.memory_samples.drain(0..1000);
        }
    }

    pub fn get_peak_memory(&self) -> usize {
        self.memory_samples
            .iter()
            .map(|s| s.heap_size + s.stack_size)
            .max()
            .unwrap_or(0)
    }

    pub fn get_current_memory(&self) -> usize {
        self.memory_samples
            .last()
            .map(|s| s.heap_size + s.stack_size)
            .unwrap_or(0)
    }

    pub fn get_initial_memory(&self) -> usize {
        self.memory_samples
            .first()
            .map(|s| s.heap_size + s.stack_size)
            .unwrap_or(0)
    }
}

impl CommandTracer {
    pub fn add_command_started(&mut self, session_id: String, command: String, timestamp: chrono::DateTime<chrono::Utc>) {
        let entry = TraceEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp,
            level: self.trace_level.clone(),
            session_id,
            command,
            input: String::new(),
            output: String::new(),
            exit_code: -1,
            duration: Duration::ZERO,
            environment: std::env::vars().collect(),
            working_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                .to_string_lossy()
                .to_string(),
            metadata: HashMap::new(),
        };

        self.entries.push(entry);
        
        if self.entries.len() > self.max_entries {
            self.entries.drain(0..1000);
        }
    }

    pub fn add_command_completed(&mut self, session_id: String, command: String, duration: Duration, exit_code: i32) {
        if let Some(entry) = self.entries.iter_mut().rev().find(|e| e.session_id == session_id && e.command == command && e.exit_code == -1) {
            entry.duration = duration;
            entry.exit_code = exit_code;
        }
    }

    pub fn add_error(&mut self, error: String, context: String, timestamp: chrono::DateTime<chrono::Utc>) {
        let metadata = HashMap::from([
            ("error".to_string(), serde_json::Value::String(error)),
            ("context".to_string(), serde_json::Value::String(context)),
        ]);

        if let Some(entry) = self.entries.last_mut() {
            entry.metadata.extend(metadata);
        }
    }

    pub fn get_command_count(&self) -> u64 {
        self.entries.len() as u64
    }

    pub fn get_successful_command_count(&self) -> u64 {
        self.entries.iter().filter(|e| e.exit_code == 0).count() as u64
    }

    pub fn get_failed_command_count(&self) -> u64 {
        self.entries.iter().filter(|e| e.exit_code != 0 && e.exit_code != -1).count() as u64
    }

    pub fn get_average_command_time(&self) -> Duration {
        let completed_commands: Vec<_> = self.entries.iter()
            .filter(|e| e.exit_code != -1)
            .collect();
        
        if completed_commands.is_empty() {
            return Duration::ZERO;
        }

        let total: Duration = completed_commands.iter().map(|e| e.duration).sum();
        total / completed_commands.len() as u32
    }

    pub fn get_most_common_commands(&self) -> Vec<(String, u64)> {
        let mut counts: HashMap<String, u64> = HashMap::new();
        
        for entry in &self.entries {
            *counts.entry(entry.command.clone()).or_insert(0) += 1;
        }
        
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(10);
        sorted
    }

    pub fn get_slowest_commands(&self) -> Vec<(String, Duration)> {
        let mut completed: Vec<_> = self.entries.iter()
            .filter(|e| e.exit_code != -1)
            .map(|e| (e.command.clone(), e.duration))
            .collect();
        
        completed.sort_by(|a, b| b.1.cmp(&a.1));
        completed.truncate(10);
        completed
    }

    pub fn get_error_count(&self) -> u64 {
        self.entries.iter()
            .filter(|e| e.metadata.contains_key("error"))
            .count() as u64
    }

    pub fn get_error_types(&self) -> HashMap<String, u64> {
        let mut types: HashMap<String, u64> = HashMap::new();
        
        for entry in &self.entries {
            if let Some(error) = entry.metadata.get("error") {
                if let Some(error_str) = error.as_str() {
                    *types.entry(error_str.to_string()).or_insert(0) += 1;
                }
            }
        }
        
        types
    }

    pub fn get_recent_errors(&self) -> Vec<String> {
        self.entries.iter()
            .rev()
            .filter_map(|e| e.metadata.get("error").and_then(|err| err.as_str().map(|s| s.to_string())))
            .take(10)
            .collect()
    }
}

impl PerformanceMonitor {
    pub fn update_metrics(&mut self) {
        // Update internal metrics
        self.metrics.insert("cpu".to_string(), MetricValue {
            value: 0.0,
            timestamp: chrono::Utc::now(),
            tags: HashMap::new(),
        });
        
        self.metrics.insert("memory".to_string(), MetricValue {
            value: 0.0,
            timestamp: chrono::Utc::now(),
            tags: HashMap::new(),
        });
    }

    pub fn add_metric(&mut self, metric_type: MetricType, value: f64, timestamp: chrono::DateTime<chrono::Utc>) {
        let name = match metric_type {
            MetricType::CpuUsage => "cpu_usage",
            MetricType::MemoryUsage => "memory_usage",
            MetricType::DiskUsage => "disk_usage",
            MetricType::NetworkLatency => "network_latency",
            MetricType::CommandLatency => "command_latency",
            MetricType::RenderFps => "render_fps",
        };

        self.metrics.insert(name.to_string(), MetricValue {
            value,
            timestamp,
            tags: HashMap::new(),
        });
    }

    pub fn record_command_duration(&mut self, command: &str, duration: Duration) {
        let histogram = self.histograms.entry("command_duration".to_string())
            .or_insert_with(|| Histogram {
                buckets: vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0],
                counts: vec![0; 6],
                sum: 0.0,
                count: 0,
            });

        let duration_ms = duration.as_secs_f64() * 1000.0;
        histogram.sum += duration_ms;
        histogram.count += 1;

        for (i, bucket) in histogram.buckets.iter().enumerate() {
            if duration_ms <= *bucket {
                histogram.counts[i] += 1;
            }
        }
    }

    pub fn record_network_activity(&mut self, direction: NetworkDirection, bytes: usize, endpoint: &str) {
        let counter_name = match direction {
            NetworkDirection::Upload => "network_upload_bytes",
            NetworkDirection::Download => "network_download_bytes",
        };

        *self.counters.entry(counter_name.to_string()).or_insert(0) += bytes as u64;
    }

    pub fn get_peak_cpu(&self) -> f64 {
        self.metrics.get("cpu_usage")
            .map(|m| m.value)
            .unwrap_or(0.0)
    }

    pub fn get_average_fps(&self) -> f64 {
        self.metrics.get("render_fps")
            .map(|m| m.value)
            .unwrap_or(60.0)
    }
}
