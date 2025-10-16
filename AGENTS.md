# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Rust sources. Key modules include `main.rs` (game loop), `world.rs` (voxel data), `mesh.rs` (chunk meshing), `texture.rs` (PNG atlas loader), and `config.rs` (runtime settings).
- `assets/`: Static content such as `textures/blocks.png` and metadata JSON.
- `config.json`: Runtime configuration for input, present mode, and FPS limits. Keep defaults sensible when extending.
- `src/bin/`: Auxiliary tooling (e.g., `atlasify.rs` for generating atlas metadata).

## Build, Test, and Development Commands
- `cargo check` – Fast validation of the codebase without producing binaries; run before opening a PR.
- `cargo fmt` – Applies Rustfmt to match repository style.
- `cargo clippy --all-targets --all-features` – Lints for idiomatic issues; treat warnings as actionable.
- `cargo run` – Launches the game window with the current configuration.

## Coding Style & Naming Conventions
- Follow Rust 2024 edition defaults; four-space indentation, snake_case for modules/functions, PascalCase for types.
- Prefer expressive module boundaries (e.g., keep config parsing in `config.rs`).
- Use Rustfmt and Clippy prior to commit; avoid committing formatted diffs.
- Keep public APIs documented with concise `///` comments when exporting new modules.

## Testing Guidelines
- Unit tests belong adjacent to the code under test (`mod tests` inside the same file).
- Integration tests go under `tests/` if large scenarios are added.
- Name tests after behavior being validated (e.g., `chunk_generates_surface_faces`).
- Run `cargo test` locally; add targeted benches under `benches/` if performance-sensitive code is introduced.

## Commit & Pull Request Guidelines
- Commit messages follow imperative mood (`Add mouse look`, `Fix chunk mesher overlap`).
- Group related changes; avoid mixing formatting-only commits with feature work.
- PRs should include: summary of changes, testing evidence (`cargo check`, `cargo test`), and configuration notes (e.g., new `config.json` fields) plus screenshots/GIFs when UI-facing behavior changes.

## Configuration & Performance Tips
- Present mode (`vsync`, `mailbox`, `immediate`) and `max_fps` are user-configurable via `config.json`; ensure defaults remain stable after altering rendering code.
- When introducing new assets, keep PNGs under `assets/` and update metadata via `cargo run --bin atlasify`.
