//! M2 harness — headless wgpu render test: clear to a known color, read
//! pixels back, assert. This is the backbone for all later render tests
//! (sprite placement M3, text pixels M7, lit/unlit M8).
//!
//! Gracefully skips when no GPU adapter is available (GPU-less CI):
//! `eprintln!("skip")` + early return.

use gamerrust::gfx::Graphics;

const W: u32 = 32;
const H: u32 = 32;

fn read_center_pixel(gfx: &mut Graphics) -> [u8; 4] {
    let data = gfx.read_pixels().expect("read headless pixels");
    let bytes_per_row = (W * 4).next_multiple_of(256);
    let i = ((H / 2) * bytes_per_row + (W / 2) * 4) as usize;
    [data[i], data[i + 1], data[i + 2], data[i + 3]]
}

#[test]
fn headless_clear_matches_expected_color() {
    let Some(mut gfx) = Graphics::headless(W, H) else {
        eprintln!("skip: no GPU adapter for headless render test");
        return;
    };

    gfx.clear_color([0.2, 0.4, 0.6, 1.0]);
    gfx.begin_frame();
    // A pass that writes nothing else — the clear must land unmodified.
    gfx.end_frame();

    let px = read_center_pixel(&mut gfx);
    eprintln!("center pixel: {px:?}");
    // Rgba8Unorm linear: f32 → u8 is round(c × 255).
    assert_eq!(px, [51, 102, 153, 255]);
}

#[test]
fn headless_clear_is_uniform_across_target() {
    let Some(mut gfx) = Graphics::headless(W, H) else {
        eprintln!("skip: no GPU adapter for headless render test");
        return;
    };

    gfx.clear_color([1.0, 0.0, 0.0, 1.0]);
    gfx.begin_frame();
    gfx.end_frame();

    let data = gfx.read_pixels().expect("read headless pixels");
    let bytes_per_row = (W * 4).next_multiple_of(256);
    for y in 0..H {
        for x in 0..W {
            let i = (y * bytes_per_row + x * 4) as usize;
            assert_eq!(
                [data[i], data[i + 1], data[i + 2], data[i + 3]],
                [255, 0, 0, 255],
                "uniform red expected at ({x},{y})"
            );
        }
    }
}
