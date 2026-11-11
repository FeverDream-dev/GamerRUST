//! Text rendering over glyphon: system fonts by default, zero assets.
//!
//! [`TextContext`] owns the glyphon plumbing (font system, atlas, swash
//! cache). Queue strings with [`TextContext::draw`], then call
//! [`TextContext::flush`] between `begin_frame`/`end_frame`, after sprites.
//!
//! ponytail: no embedded font (crate size + licensing); `FontSystem::new()`
//! loads system fonts everywhere. No custom-font loading API until a game
//! needs it.

use glam::Vec2;
use glyphon::{
    Attrs, Buffer, Cache as GlyphonCache, Color, Family, FontSystem, Metrics, Shaping, SwashCache,
    TextArea, TextBounds, TextRenderer as GlyphonRenderer, Viewport,
};

/// A queued string with placement and style.
#[derive(Clone, Debug)]
pub struct Text {
    /// Screen-space top-left in pixels (viewport coordinates, +y down).
    pub position: Vec2,
    pub content: String,
    /// Font size in pixels.
    pub size: f32,
    /// sRGB color, each channel 0..=255.
    pub color: [u8; 4],
}

impl Text {
    pub fn new(position: Vec2, content: impl Into<String>) -> Self {
        Self {
            position,
            content: content.into(),
            size: 24.0,
            color: [255, 255, 255, 255],
        }
    }

    pub fn size(mut self, px: f32) -> Self {
        self.size = px;
        self
    }

    pub fn color(mut self, rgba: [u8; 4]) -> Self {
        self.color = rgba;
        self
    }
}

/// The text subsystem: create once (e.g. in `Game::init`), reuse per frame.
pub struct TextContext {
    font_system: FontSystem,
    swash: SwashCache,
    atlas: glyphon::TextAtlas,
    viewport: Viewport,
    renderer: GlyphonRenderer,
    /// Owned shaped buffers with their placement; consumed by flush.
    queued: Vec<(Buffer, Vec2, Color)>,
    screen: (u32, u32),
}

impl TextContext {
    pub fn new(gfx: &crate::gfx::Graphics) -> Self {
        let device = &gfx.device;
        let queue = &gfx.queue;
        let font_system = FontSystem::new();
        let cache = GlyphonCache::new(device);
        let mut atlas = glyphon::TextAtlas::new(device, queue, &cache, gfx.format());
        let mut viewport = Viewport::new(device, &cache);
        viewport.update(
            queue,
            glyphon::Resolution {
                width: 1,
                height: 1,
            },
        );
        let renderer = GlyphonRenderer::new(&mut atlas, device, Default::default(), None);
        Self {
            font_system,
            swash: SwashCache::new(),
            atlas,
            viewport,
            renderer,
            queued: Vec::new(),
            screen: (1, 1),
        }
    }

    /// Queue text for this frame (screen-space pixels, +y down).
    pub fn draw(&mut self, text: Text) {
        // Metrics: font size px, line height ~1.2× (cosmic default feel).
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new(text.size, text.size * 1.2),
        );
        buffer.set_text(
            &text.content,
            &Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
        let color = Color::rgba(text.color[0], text.color[1], text.color[2], text.color[3]);
        self.queued.push((buffer, text.position, color));
    }

    /// Upload glyphs and record the text pass into the in-flight frame.
    /// Call between `gfx.begin_frame()` and `gfx.end_frame()`, after sprites.
    pub fn flush(&mut self, gfx: &mut crate::gfx::Graphics) {
        let (w, h) = gfx.size();
        if self.screen != (w, h) {
            self.viewport.update(
                &gfx.queue,
                glyphon::Resolution {
                    width: w,
                    height: h,
                },
            );
            self.screen = (w, h);
        }
        if self.queued.is_empty() {
            return;
        }

        // Build text areas borrowing the owned buffers, then prepare.
        let queued = &self.queued;
        let areas: Vec<TextArea<'_>> = queued
            .iter()
            .map(|(b, pos, color)| TextArea {
                buffer: b,
                left: pos.x,
                top: pos.y,
                scale: 1.0,
                bounds: TextBounds::default(),
                default_color: *color,
                custom_glyphs: &[],
            })
            .collect();

        let result = self.renderer.prepare(
            &gfx.device,
            &gfx.queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash,
        );
        self.queued.clear();
        if let Err(e) = result {
            log::warn!("glyphon prepare failed: {e:?}");
            return;
        }

        let atlas = &self.atlas;
        let viewport = &self.viewport;
        let renderer = &self.renderer;
        gfx.with_frame(move |encoder, view| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gamerrust.text"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let _ = renderer.render(atlas, viewport, &mut pass);
        });
    }
}
