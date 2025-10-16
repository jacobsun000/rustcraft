# Adding New Blocks

1. Define block metadata in `src/block.rs`.
   - Assign a unique `BlockId` (keep within `u8::MAX`).
   - Append a `BlockDefinition` entry with `solid`, `luminance`, `reflectivity`, and `face_tiles` values. Each face index (NegX…PosZ) maps to a tile in the atlas.
   - Export a helper constant if the block will be referenced frequently (e.g. `pub const BLOCK_MY_BLOCK: BlockId = …`).

2. Update world logic if the block should appear in terrain.
   - Use `BlockKind::from_id` or add a new `BlockKind` variant when the block is part of procedural generation.
   - Modify `generate_chunk` (or other generation systems) to place the new block IDs.
   - Keep the lamp-style placement logic handy as a template for hand-crafted insertions.

3. Expand the texture atlas.
   - Add new tiles to `assets/textures/blocks.png`; keep tile size consistent (currently 16×16).
   - Regenerate atlas metadata if layout changes: `cargo run --bin atlasify`.
   - Reference the new tile coordinates in the `face_tiles` array; tile `(x, y)` corresponds to the grid index within the PNG.

4. Adjust renderer-facing assets if needed.
   - For rasterization, no extra work is required beyond atlas updates.
   - The ray tracer automatically consumes `BlockDefinition` fields; tweak `luminance` or `reflectivity` to change emission and mirror response.

5. Verify end-to-end.
   - Run `cargo fmt` and `cargo check`.
   - Launch via `cargo run` to ensure the block renders in both raster and ray-traced modes.
   - Capture screenshots or notes for inclusion in PRs, and document config/world impacts in `AGENTS.md` if behavior changes materially.
