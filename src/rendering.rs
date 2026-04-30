//! Modern GPU-accelerated rendering engine
//! 
//! This module provides high-performance rendering using wgpu and egui
//! for smooth terminal display and UI elements.

use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use tracing::{debug, info, warn};

pub struct Renderer {
    config: Arc<crate::config::Config>,
    window: Arc<RwLock<Option<winit::window::Window>>>,
    gpu_context: Arc<RwLock<Option<GpuContext>>>,
    ui_state: Arc<RwLock<UiState>>,
}

#[derive(Debug)]
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
}

#[derive(Debug, Default)]
pub struct UiState {
    pub terminal_content: Vec<TerminalLine>,
    pub cursor_position: (usize, usize),
    pub scroll_offset: usize,
    pub input_buffer: String,
    pub suggestions: Vec<String>,
    pub show_ai_panel: bool,
    pub show_file_explorer: bool,
    pub show_debug_panel: bool,
}

#[derive(Debug, Clone)]
pub struct TerminalLine {
    pub content: String,
    pub attributes: LineAttributes,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct LineAttributes {
    pub foreground_color: Option<(u8, u8, u8)>,
    pub background_color: Option<(u8, u8, u8)>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Renderer {
    pub fn new(config: &Arc<crate::config::Config>) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            window: Arc::new(RwLock::new(None)),
            gpu_context: Arc::new(RwLock::new(None)),
            ui_state: Arc::new(RwLock::new(UiState::default())),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing renderer");
        
        // Create window
        self.create_window().await?;
        
        // Initialize GPU context
        self.initialize_gpu().await?;
        
        info!("Renderer initialized successfully");
        Ok(())
    }

    async fn create_window(&self) -> Result<()> {
        let event_loop = EventLoop::build().build()?;
        
        let window = WindowBuilder::new()
            .with_title("Weft Terminal")
            .with_inner_size(winit::dpi::PhysicalSize::new(1200, 800))
            .with_min_inner_size(winit::dpi::PhysicalSize::new(400, 300))
            .with_window_icon(Some(load_icon()))
            .build(&event_loop)?;
        
        *self.window.write() = Some(window);
        
        Ok(())
    }

    async fn initialize_gpu(&self) -> Result<()> {
        let window = self.window.read().as_ref().unwrap();
        
        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });

        // Create surface
        let surface = unsafe { instance.create_surface(window) }.unwrap();

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;

        // Request device
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_format],
        };

        surface.configure(&device, &surface_config);

        let gpu_context = GpuContext {
            instance,
            adapter,
            device,
            queue,
            surface,
            surface_config,
        };

        *self.gpu_context.write() = Some(gpu_context);
        
        Ok(())
    }

    pub async fn render(&self) -> Result<()> {
        let window = self.window.read();
        let window = window.as_ref().unwrap();
        
        let gpu_context = self.gpu_context.read();
        let gpu_context = gpu_context.as_ref().unwrap();
        
        let ui_state = self.ui_state.read();
        
        // Get the next surface texture
        let surface_texture = match gpu_context.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(e) => {
                warn!("Failed to get surface texture: {}", e);
                return Ok(());
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = gpu_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Render terminal content
            self.render_terminal_content(&mut render_pass, &ui_state);
        }

        // Submit commands
        let command_buffer = encoder.finish();
        gpu_context.queue.submit(Some(command_buffer));
        
        // Present
        surface_texture.present();

        Ok(())
    }

    fn render_terminal_content(
        &self,
        _render_pass: &mut wgpu::RenderPass<'_>,
        ui_state: &UiState,
    ) {
        debug!("Rendering {} terminal lines", ui_state.terminal_content.len());
        
        // TODO: Implement actual terminal rendering with wgpu
        // For now, we'll use a placeholder implementation
        
        // Render each line
        for (i, line) in ui_state.terminal_content.iter().enumerate() {
            if i >= ui_state.scroll_offset && i < ui_state.scroll_offset + 24 {
                // This line is visible
                debug!("Rendering line {}: {}", i, line.content);
            }
        }
        
        // Render cursor
        let (cursor_x, cursor_y) = ui_state.cursor_position;
        debug!("Rendering cursor at ({}, {})", cursor_x, cursor_y);
        
        // Render AI suggestions panel if visible
        if ui_state.show_ai_panel {
            debug!("Rendering AI suggestions panel");
            for suggestion in &ui_state.suggestions {
                debug!("  Suggestion: {}", suggestion);
            }
        }
    }

    pub fn add_terminal_line(&self, content: String, attributes: LineAttributes) {
        let mut ui_state = self.ui_state.write();
        ui_state.terminal_content.push(TerminalLine {
            content,
            attributes,
            timestamp: chrono::Utc::now(),
        });
        
        // Limit history size
        if ui_state.terminal_content.len() > 10000 {
            ui_state.terminal_content.drain(0..1000);
        }
    }

    pub fn set_cursor_position(&self, x: usize, y: usize) {
        let mut ui_state = self.ui_state.write();
        ui_state.cursor_position = (x, y);
    }

    pub fn set_input_buffer(&self, input: String) {
        let mut ui_state = self.ui_state.write();
        ui_state.input_buffer = input;
    }

    pub fn add_suggestion(&self, suggestion: String) {
        let mut ui_state = self.ui_state.write();
        ui_state.suggestions.push(suggestion);
        
        // Limit suggestions
        if ui_state.suggestions.len() > 10 {
            ui_state.suggestions.drain(0..5);
        }
    }

    pub fn clear_suggestions(&self) {
        let mut ui_state = self.ui_state.write();
        ui_state.suggestions.clear();
    }

    pub fn toggle_ai_panel(&self) {
        let mut ui_state = self.ui_state.write();
        ui_state.show_ai_panel = !ui_state.show_ai_panel;
    }

    pub fn toggle_file_explorer(&self) {
        let mut ui_state = self.ui_state.write();
        ui_state.show_file_explorer = !ui_state.show_file_explorer;
    }

    pub fn toggle_debug_panel(&self) {
        let mut ui_state = self.ui_state.write();
        ui_state.show_debug_panel = !ui_state.show_debug_panel;
    }

    pub fn scroll_up(&self, lines: usize) {
        let mut ui_state = self.ui_state.write();
        ui_state.scroll_offset = ui_state.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_down(&self, lines: usize) {
        let mut ui_state = self.ui_state.write();
        ui_state.scroll_offset = (ui_state.scroll_offset + lines)
            .min(ui_state.terminal_content.len().saturating_sub(24));
    }

    pub fn get_terminal_content(&self) -> Vec<TerminalLine> {
        self.ui_state.read().terminal_content.clone()
    }

    pub fn get_cursor_position(&self) -> (usize, usize) {
        self.ui_state.read().cursor_position
    }

    pub fn get_input_buffer(&self) -> String {
        self.ui_state.read().input_buffer.clone()
    }

    pub fn get_suggestions(&self) -> Vec<String> {
        self.ui_state.read().suggestions.clone()
    }
}

fn load_icon() -> winit::window::Icon {
    // Simple 16x16 icon (would normally load from file)
    let rgba = vec![
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    ];
    
    winit::window::Icon::from_rgba(rgba, 16, 16).unwrap()
}
