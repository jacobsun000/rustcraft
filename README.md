# Rustcraft

_A GPU-accelerated voxel sandbox written in Rust + WGPU._

![World Overview](docs/sc1.png)

## Table of Contents

- [Highlights](#highlights)
- [Sneak Peek](#sneak-peek)
- [Quick Start](#quick-start)
- [Controls & Interactions](#controls--interactions)
- [Configuration](#configuration)
- [Rendering & Performance](#rendering--performance)
- [Project Layout](#project-layout)
- [Assets & Block Authoring](#assets--block-authoring)
- [Development Workflow](#development-workflow)

## Highlights

- Hybrid renderer: switch between a classic chunked rasterizer and a compute-driven ray tracer via `render_method` in `config.json`.
- Streamed voxel world: procedural chunk generation, visibility culling, and background unloading keep memory predictable.
- Full interaction loop: dig, place, and pick blocks with a scrollable hotbar plus block-specific material properties (emissive lamps, transmissive glass, etc.).
- Physics-aware movement: toggleable walk/fly modes with gravity, collisions, and jump impulses.
- Built-in benchmarking path scripts and detailed timing overlay for profiling different GPUs or present modes.

## Quick Start

### Prerequisites

- Rust 1.80+ with the 2024 edition enabled (`rustup update stable`).
- A GPU with WGPU/DX12/Metal/Vulkan support.

### Build & Run

```bash
cargo fmt
cargo check
cargo run          # launches the interactive client
```

### Optional tooling

- `cargo run --bin benchmark` &mdash; runs the scripted performance sweep and prints frame time stats.
- `cargo run --bin atlasify assets/textures/blocks.png assets/textures/blocks.json 16` &mdash; regenerates atlas metadata when you update the block texture sheet.

## Controls & Interactions

- `WASD` move, `Space` jump/ascend, `Left Shift` descend, `F` toggles Walk ↔ Fly mode.
- Mouse look is active once the cursor is captured (click to capture, `Esc` to release).
- `Mouse Wheel` cycles the hotbar; number keys `1`–`9` jump directly to a slot.
- `Left Click` breaks blocks, `Right Click` places the currently selected block, `Middle Click` samples the looked-at block into the hotbar.
- Cursor capture automatically re-engages on click and releases on window unfocus.

## Configuration

Rustcraft reads `config.json` at startup (a default is generated if missing):

```jsonc
{
  "mouse_sensitivity": 0.05,
  "keymap": {
    "move_forward": "W",
    "move_backward": "S",
    "move_left": "A",
    "move_right": "D",
    "move_up": "Space",
    "move_down": "LShift"
  },
  "present_mode": "vsync",        // vsync | mailbox | immediate
  "max_fps": 240,                 // optional software frame limiter
  "render_method": "raytraced"    // rasterized | raytraced
}
```

Notes:

- Keys accept any `VirtualKeyCode` string (letters, digits, `Space`, `Ctrl`, etc.) and fall back to sensible defaults if parsing fails.
- `present_mode` maps to the platform’s swap-chain present modes; try `mailbox` for reduced latency, `immediate` for unlocked tearing.
- `max_fps` clamps CPU-side frame pacing; the ray tracer also collects GPU timestamps when the device supports `TIMESTAMP_QUERY`.

## Rendering & Performance

- **Raster Renderer** (`render_method = "rasterized"`): classic mesh-based pipeline that rebuilds chunk meshes when the world version increments.
- **Ray-Traced Renderer** (`render_method = "raytraced"`): compute pipeline (`raytrace_compute.wgsl`) that ingests packed voxel data, per-block material properties, and samples from the texture atlas in screen space.
- **Debug Overlay**: displays FPS, frame timings, chunk counts, renderer kind, and camera coordinates in the top-left corner.
- **Benchmark Script**: drives deterministic camera + movement paths to compare GPUs or renderer settings. Results include FPS percentiles, chunk throughput, and GPU timing averages.

## Project Layout

- `src/main.rs` & `src/app/`: window/event loop, renderer selection, and top-level state machine.
- `src/world.rs`: chunk streaming, procedural terrain, visibility masks, and block editing helpers.
- `src/render/`: raster mesh builder, compute ray tracer, shaders (`shader.wgsl`, `raytrace_*.wgsl`).
- `src/physics.rs`, `src/input.rs`, `src/camera.rs`: movement model, controller, and camera math.
- `src/texture.rs` + `assets/textures/`: atlas loader plus PNG/JSON pair used by both renderers.
- `src/bin/atlasify.rs`: CLI for generating atlas metadata from a tile sheet.
- `docs/ADDING_BLOCKS.md`: playbook for defining new blocks/materials.

## Assets & Block Authoring

1. Edit `assets/textures/blocks.png` (16×16 tiles per slot by default).
2. Re-run the atlas generator once layouts change.
3. Define new `BlockDefinition`s in `src/block.rs`, including material properties (specular, roughness, emission, transmission, etc.).
4. Update world generation (`generate_chunk`) if the block should appear procedurally.
5. Verify both renderers by running `cargo run` and toggling `render_method`.

Refer to `docs/ADDING_BLOCKS.md` for the full checklist.

## Development Workflow

- Format: `cargo fmt`.
- Lint: `cargo clippy --all-targets --all-features`.
- Test: `cargo test` (unit tests live next to their modules; add integration tests under `tests/` for larger scenarios).
- Gameplay smoke test: `cargo run` in both walk and fly modes, interact with blocks, and capture updated screenshots for PRs.
- Benchmark: `cargo run --bin benchmark` whenever renderer or physics code changes to track performance regressions.

When contributing, keep commit messages in imperative mood, document new public APIs with `///`, and include updated screenshots/GIFs where this README leaves placeholders.
