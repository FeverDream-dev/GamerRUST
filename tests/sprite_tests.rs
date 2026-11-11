//! M3 harness — headless sprite-batcher pixel test: a full-screen sprite
//! must repaint every pixel; the camera matrix must put sprite pixels where
//! the math says (screen-center origin, +y down).

use gamerrust::camera::Camera2D;
use gamerrust::gfx::batch2d::{Sprite, SpriteBatch};
use gamerrust::gfx::{Graphics, Texture};
use glam::Vec2;

const W: u32 = 64;
const H: u32 = 64;

fn solid_texture(gfx: &Graphics) -> Texture {
    let mut data = vec![0u8; (W * H * 4) as usize];
    for px in data.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 0, 0, 255]); // pure red: survives sRGB round-trip
    }
    Texture::from_rgba(gfx, W, H, &data)
}

fn pixel(data: &[u8], x: u32, y: u32) -> [u8; 4] {
    let bpr = (W * 4).next_multiple_of(256);
    let i = (y * bpr + x * 4) as usize;
    [data[i], data[i + 1], data[i + 2], data[i + 3]]
}

#[test]
fn fullscreen_sprite_covers_target() {
    let Some(mut gfx) = Graphics::headless(W, H) else {
        eprintln!("skip: no GPU adapter");
        return;
    };
    let tex = solid_texture(&gfx);
    let mut batch = SpriteBatch::new(&gfx, tex);

    gfx.clear_color([0.0, 0.0, 1.0, 1.0]); // blue underlay
    gfx.begin_frame();
    batch.set_camera(&Camera2D::default(), W as f32, H as f32);
    batch.push(Sprite::new(Vec2::ZERO, Vec2::new(W as f32, H as f32)));
    batch.flush(&mut gfx);
    gfx.end_frame();

    let data = gfx.read_pixels().expect("read pixels");
    for y in (0..H).step_by(7) {
        for x in (0..W).step_by(7) {
            let p = pixel(&data, x, y);
            assert_eq!(p, [255, 0, 0, 255], "pixel ({x},{y}) not covered: {p:?}");
        }
    }
}

#[test]
fn sprite_lands_where_camera_math_says() {
    let Some(mut gfx) = Graphics::headless(W, H) else {
        eprintln!("skip: no GPU adapter");
        return;
    };
    let tex = solid_texture(&gfx);
    let mut batch = SpriteBatch::new(&gfx, tex);

    // Half-size sprite centered at origin covers the CENTER region:
    // screen-center origin, +y down → pixels [16,48)².
    gfx.clear_color([0.0, 0.0, 0.0, 1.0]); // black underlay
    gfx.begin_frame();
    batch.set_camera(&Camera2D::default(), W as f32, H as f32);
    batch.push(Sprite::new(Vec2::ZERO, Vec2::new(32.0, 32.0)));
    batch.flush(&mut gfx);
    gfx.end_frame();

    let data = gfx.read_pixels().expect("read pixels");
    // Center: sprite (red).
    assert_eq!(
        pixel(&data, 32, 32),
        [255, 0, 0, 255],
        "center inside sprite"
    );
    // Corner well outside the ±16 world region: clear (black).
    assert_eq!(pixel(&data, 8, 8), [0, 0, 0, 255], "TL corner outside");
    assert_eq!(pixel(&data, 56, 56), [0, 0, 0, 255], "BR corner outside");
}

#[test]
fn camera_offset_moves_sprite() {
    let Some(mut gfx) = Graphics::headless(W, H) else {
        eprintln!("skip: no GPU adapter");
        return;
    };
    let tex = solid_texture(&gfx);
    let mut batch = SpriteBatch::new(&gfx, tex);

    // Camera centered at world (16, 0): world x = screen_x - 16. The
    // ±16-wide sprite maps to screen x ∈ [0, 48); y unchanged: [16, 48).
    gfx.clear_color([0.0, 0.0, 0.0, 1.0]);
    gfx.begin_frame();
    let cam = Camera2D {
        position: Vec2::new(16.0, 0.0),
        ..Default::default()
    };
    batch.set_camera(&cam, W as f32, H as f32);
    batch.push(Sprite::new(Vec2::ZERO, Vec2::new(32.0, 32.0)));
    batch.flush(&mut gfx);
    gfx.end_frame();

    let data = gfx.read_pixels().expect("read pixels");
    // Screen (24, 32) → world (8, 0): inside the sprite.
    assert_eq!(
        pixel(&data, 24, 32),
        [255, 0, 0, 255],
        "inside shifted sprite"
    );
    // Screen (60, 32) → world (44, 0): outside.
    assert_eq!(
        pixel(&data, 60, 32),
        [0, 0, 0, 255],
        "right of shifted sprite"
    );
    // Screen (24, 8) → world (8, -24): outside vertically.
    assert_eq!(pixel(&data, 24, 8), [0, 0, 0, 255], "above shifted sprite");
}

// ---- M7: text pixel assert (same headless harness) ----

use gamerrust::gfx::text::{Text, TextContext};

#[test]
fn text_renders_visible_pixels() {
    let Some(mut gfx) = Graphics::headless(128, 64) else {
        eprintln!("skip: no GPU adapter");
        return;
    };
    let mut text = TextContext::new(&gfx);
    // Big white text on black: some pixels must be non-black after render.
    gfx.clear_color([0.0, 0.0, 0.0, 1.0]);
    gfx.begin_frame();
    text.draw(Text::new(Vec2::new(8.0, 8.0), "GAMERRUST").size(32.0));
    text.flush(&mut gfx);
    gfx.end_frame();

    let data = gfx.read_pixels().expect("read pixels");
    let bpr = (128u32 * 4).next_multiple_of(256) as usize;
    let mut lit = 0;
    for y in 0..64 {
        for x in 0..128 {
            let i = y * bpr + x * 4;
            if data[i] > 40 || data[i + 1] > 40 || data[i + 2] > 40 {
                lit += 1;
            }
        }
    }
    // 32px glyphs cover at least a few percent of a 128x64 target.
    assert!(
        lit > 200,
        "text rendered only {lit} lit pixels — glyphs missing?"
    );
}
