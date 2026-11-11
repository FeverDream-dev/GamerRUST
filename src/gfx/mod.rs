//! wgpu graphics context: device/queue plus a window surface, or an
//! offscreen texture for headless tests ([`Graphics::headless`]).
//!
//! Frame lifecycle is owned here so a game can never forget to present:
//! [`Graphics::begin_frame`] acquires the target and bakes the clear color
//! into the encoder, [`Graphics::end_frame`] submits and presents (or stages
//! the headless readback copy). A throwaway clearing pass is recorded in
//! `begin_frame`; later milestones (M3 batcher, M7 text) record `Load`-passes
//! on the same view between the two calls.

use std::sync::Arc;

use winit::window::Window;

/// The graphics subsystem, reachable from [`crate::app::Context`] as
/// `ctx.gfx()`.
pub struct Graphics {
    /// Shared GPU device — public for custom pipelines in later milestones.
    pub device: wgpu::Device,
    /// Shared GPU queue — public for custom buffer/texture writes.
    pub queue: wgpu::Queue,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    offscreen: Option<Offscreen>,
    clear: wgpu::Color,
    frame: Option<Frame>,
}

/// Offscreen render target + readback buffer backing `headless` mode.
struct Offscreen {
    texture: wgpu::Texture,
    readback: wgpu::Buffer,
    /// Row stride for copies: wgpu requires 256-byte alignment.
    bytes_per_row: u32,
    width: u32,
    height: u32,
}

/// In-flight frame state between `begin_frame` and `end_frame`.
struct Frame {
    encoder: wgpu::CommandEncoder,
    /// Held for M3+ render passes (sprites/text/3D draw into this view).
    #[expect(dead_code)]
    view: wgpu::TextureView,
    surface_texture: Option<wgpu::SurfaceTexture>,
}

impl Graphics {
    /// Bind a graphics context to `window` (shared ownership: the engine
    /// keeps its own `Arc` so both can outlive each other's borrows).
    /// `vsync` selects the present mode (`Fifo` on, `AutoNoVsync` off).
    /// `None` when no adapter/surface is available (GPU-less CI) — the
    /// engine keeps running without drawing.
    pub fn new(window: Arc<Window>, vsync: bool) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let inner = window.inner_size();
        let surface = instance.create_surface(window).ok()?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("gamerrust.device"),
            ..Default::default()
        }))
        .ok()?;

        let Some(mut config) =
            surface.get_default_config(&adapter, inner.width.max(1), inner.height.max(1))
        else {
            log::warn!("surface has no default configuration");
            return None;
        };
        config.present_mode = if vsync {
            wgpu::PresentMode::Fifo
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        surface.configure(&device, &config);

        Some(Self {
            device,
            queue,
            surface: Some(surface),
            config: Some(config),
            offscreen: None,
            clear: wgpu::Color::BLACK,
            frame: None,
        })
    }

    /// Offscreen context for headless pixel tests: renders to a linear
    /// `Rgba8Unorm` texture, no window. Pair with [`Graphics::read_pixels`].
    /// `None` when no GPU adapter is available.
    pub fn headless(width: u32, height: u32) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("gamerrust.headless"),
            ..Default::default()
        }))
        .ok()?;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gamerrust.offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Linear (no sRGB) so cleared bytes == clear color × 255 exactly.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let bytes_per_row = (width * 4).next_multiple_of(256);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gamerrust.readback"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Some(Self {
            device,
            queue,
            surface: None,
            config: None,
            offscreen: Some(Offscreen {
                texture,
                readback,
                bytes_per_row,
                width,
                height,
            }),
            clear: wgpu::Color::BLACK,
            frame: None,
        })
    }

    /// Set the clear color (linear RGBA) used by subsequent frames.
    pub fn clear_color(&mut self, rgba: [f32; 4]) {
        self.clear = wgpu::Color {
            r: rgba[0] as f64,
            g: rgba[1] as f64,
            b: rgba[2] as f64,
            a: rgba[3] as f64,
        };
    }

    /// Acquire the frame target and record the clear. A failed surface
    /// acquire reconfigures and skips the frame (e.g. right after a resize).
    pub fn begin_frame(&mut self) {
        if self.frame.is_some() {
            self.end_frame(); // never two frames in flight
        }
        let (view, surface_texture) = match &self.surface {
            Some(surface) => match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t)
                | wgpu::CurrentSurfaceTexture::Suboptimal(t) => (
                    t.texture
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                    Some(t),
                ),
                status => {
                    // Timeout/Occluded/Outdated/Lost/Validation: reconfigure
                    // and skip this frame (recommended handling).
                    log::warn!("surface acquire failed ({status:?}); reconfiguring");
                    if let Some(config) = &self.config {
                        surface.configure(&self.device, config);
                    }
                    return;
                }
            },
            None => {
                let Some(o) = &self.offscreen else {
                    return;
                };
                (
                    o.texture
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                    None,
                )
            }
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gamerrust.frame"),
            });
        {
            // Throwaway pass: baking the clear in is enough; it is dropped
            // here so later Load-passes can target the same view.
            let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gamerrust.clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        self.frame = Some(Frame {
            encoder,
            view,
            surface_texture,
        });
    }

    /// Submit the frame: present when windowed; stage the texture→buffer
    /// copy for [`Graphics::read_pixels`] when headless.
    pub fn end_frame(&mut self) {
        let Some(mut frame) = self.frame.take() else {
            return;
        };
        if let Some(o) = &self.offscreen {
            frame.encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &o.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &o.readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(o.bytes_per_row),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: o.width,
                    height: o.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.queue.submit([frame.encoder.finish()]);
        if let Some(t) = frame.surface_texture {
            self.queue.present(t);
        }
    }

    /// Read the headless target back to CPU (RGBA8, rows padded to
    /// `bytes_per_row`). `None` when windowed or the copy failed. Blocks
    /// until the GPU lands the copy.
    pub fn read_pixels(&mut self) -> Option<Vec<u8>> {
        let o = self.offscreen.as_ref()?;
        let (tx, rx) = std::sync::mpsc::channel();
        let slice = o.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        rx.recv().ok()?.ok()?;
        let data = slice.get_mapped_range().ok()?.to_vec();
        o.readback.unmap();
        Some(data)
    }

    /// Reconfigure after a window resize (no-op headless; zero sizes from a
    /// minimized window are ignored).
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if let (Some(surface), Some(config)) = (&self.surface, &mut self.config) {
            config.width = width;
            config.height = height;
            surface.configure(&self.device, config);
        }
    }

    /// The format custom pipelines must target (headless: `Rgba8Unorm`).
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config
            .as_ref()
            .map(|c| c.format)
            .unwrap_or(wgpu::TextureFormat::Rgba8Unorm)
    }

    /// Current target size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.config
            .as_ref()
            .map(|c| (c.width, c.height))
            .or_else(|| self.offscreen.as_ref().map(|o| (o.width, o.height)))
            .unwrap_or((1, 1))
    }
}
