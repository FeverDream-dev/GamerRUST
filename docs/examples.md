---
layout: default
title: Examples
---

# Examples

All examples are self-contained and generate their runtime assets where practical. Run them from the repository root with `cargo run --example <name>`.

| Example | Purpose | Controls |
| --- | --- | --- |
| `window` | Engine loop and animated clear color | <kbd>Esc</kbd> exits |
| `bunnymark_2d` | Instanced 2D sprite batching stress test | Hold <kbd>Space</kbd> to add 1,000 sprites; <kbd>Esc</kbd> exits |
| `input_demo` | Unified keyboard, mouse, and gamepad input | <kbd>WASD</kbd> or left stick moves; hold left mouse to drag; <kbd>Esc</kbd> exits |
| `audio_demo` | Generated sound effects and looped music | <kbd>1</kbd>/<kbd>2</kbd>/<kbd>3</kbd> play sounds; <kbd>M</kbd> starts or pauses music; <kbd>N</kbd> stops it; <kbd>Esc</kbd> exits |
| `text_demo` | Text positioning, color, size, and FPS display | <kbd>Esc</kbd> exits |
| `cube_3d` | Lit, depth-tested 3D mesh rendering | <kbd>A</kbd>/<kbd>D</kbd> orbit; <kbd>W</kbd>/<kbd>S</kbd> zoom; <kbd>Esc</kbd> exits |

## Window and engine loop

```sh
cargo run --example window
```

This is the smallest end-to-end engine example. It exercises window creation, the fixed update loop, draw callbacks, timing data, and graceful exit.

## 2D sprite batching

```sh
cargo run --example bunnymark_2d
```

The bunnymark starts with 1,000 generated checkerboard sprites. Hold <kbd>Space</kbd> to add sprites in batches of 1,000, up to 100,000. The title displays the current sprite count and FPS.

## Input

```sh
cargo run --example input_demo
```

The demo combines keyboard and connected gamepad movement. Holding the left mouse button moves the tracked position to the cursor. The position is reflected in both the window title and clear color.

## Audio

```sh
cargo run --example audio_demo
```

No audio files are required. Sound effects and the looped arpeggio are generated in memory. The audio device opens lazily when playback starts.

## Text

```sh
cargo run --example text_demo
```

This demo uses system fonts to draw an FPS counter, a sentence, colored text, and a moving banner.

## 3D cube

```sh
cargo run --example cube_3d
```

The 3D example uploads a generated cube mesh and renders it with a perspective camera, a model transform, directional lighting, and depth testing.

## Automated smoke exits

Some examples support `GAMERRUST_EXIT_AFTER` for automated smoke checks:

```sh
GAMERRUST_EXIT_AFTER=120 cargo run --example window
GAMERRUST_EXIT_AFTER=3 cargo run --example audio_demo
```

For `window`, the value is a frame count. For `audio_demo`, it is a number of seconds.
