# AI Coding Assistant Instructions for svg2gcode

These guidelines capture project-specific knowledge so an AI agent can be productive quickly.

## High-Level Architecture
- Workspace crates:
  - `lib` (crate: `svg2gcode`): Core SVG -> G-Code conversion (parsing, geometry, machine state, postprocessing).
  - `cli` (crate: `svg2gcode-cli`): Command-line interface wrapping the core crate.
  - `crates/svg2gcode-wasm`: WASM/JS bindings exposing `convert_svg` + JSON schema generator.
  - `web`: Yew (Rust/WASM) single-page app using the WASM crate (not the core crate directly) for browser usage.
  - `tools/alignment_check`: Ancillary tooling.
- Core flow: SVG text -> `roxmltree::Document` -> depth-first traversal (`converter::visit`) -> draw operations on a `Turtle` implementation (`GCodeTurtle`) -> token list -> formatted G-Code.
- Extensibility points:
  - Machine behavior via `Machine` sequences (tool on/off, begin/end, between layers) and capability flags (`SupportedFunctionality`).
  - Geometry flattening & arc handling in `arc.rs` and turtle methods.
  - Postprocessing (checksums, line numbers, comment formatting) through `g_code::emit::FormatOptions` (external crate).

## Key Modules (lib)
- `converter/`: Traverses SVG nodes, applies transforms, alignment, trimming; emits path primitives to the turtle.
- `turtle/`: Abstraction for drawing; `GCodeTurtle` manages tool state & sequencing; recent change defers `between_layers` until just before next `tool_on` (see `pending_between_layers`).
- `machine.rs`: Holds snippets (parsed mini G-Code templates) and simulates modal state to reduce redundant output.
- `postprocess.rs`: Adjusts final token stream (checksums, line numbering, comments) via external `g-code` crate formatting.
- `arc.rs`: Arc / curve approximation & splitting logic; watch tolerances & min arc radius interplay.

## Recent Behavioral Nuance
- Layer transitions: Order now enforced as: last cut -> tool_off -> rapid move -> (blank line + between_layers sequence) -> tool_on -> next cut. Implemented by deferring emission (field `pending_between_layers` in `GCodeTurtle`). When modifying layer logic, keep this invariant.

## WASM Layer
- `crates/svg2gcode-wasm/src/lib.rs` maps flattened option structs (`GCodeConversionOptions`) into core `Settings` then invokes `convert_svg`.
- Provides `param_schema_json()` using `schemars` for dynamic form generation.
- Any new config fields added in core must be mirrored here (and added to schema) to stay in parity.

## Web App
- Uses Yew + Yewdux for state; `web/src/state.rs` holds form + persisted settings.
- Converts multiple SVGs by looping and zipping results; when >1 file, builds a ZIP via `zip` crate.
- G-Code formatting options (checksums, line numbers, newline_before_comment) are sourced from settings and passed to `format_gcode_fmt` / `format_gcode_io`.

## CLI
- `cli/src/main.rs`: Parses flags (Clap), overlays onto `Settings`, upgrades versions (`Settings::try_upgrade`), parses snippets via `g_code::parse::snippet_parser`, constructs `Machine`.
- Keep argument alias consistency: long flags map to internal snake_case fields, plus alias lines like `#[arg(alias = "tool_on_sequence", long = "on")]`.
- When adding a new machine or conversion option, update: struct fields, snippet parsing array, diagnostic loop, export JSON consistency (tests rely on ordering in `lib/src/lib.rs` JSON samples).

## Versioning & Settings
- `Settings::try_upgrade` handles version migrations; current latest = `Version::V5`.
- Maintain backward compatibility by adding new optional fields with defaults; update deserialization tests under `lib/src/lib.rs` to cover new versions.

## Release Workflow (WASM)
- Only bump `crates/svg2gcode-wasm/Cargo.toml` for WASM releases; tag format strictly `vMAJOR.MINOR.PATCH` (e.g. `v0.1.10`).
- Commit message convention for release bumps: include summary + `bump wasm to vX.Y.Z`.
- Tag must match the bumped version exactly; never retag.

## Coding Conventions / Patterns
- Prefer calculating transforms early; alignment & trim transforms composed before traversal (`converter::mod.rs`).
- Maintain modal G-Code cleanliness: use `Machine::absolute()` / `relative()` helpers instead of raw commands.
- Snippets: Always parse via `snippet_parser` and defer errors collectively (see CLI snippet array pattern) to emit aggregated diagnostics.
- Insert layer logic ONLY inside `visit_exit` for `<g>` nodes; avoid side-effects in entry to maintain deterministic ordering.
- For arc generation, fallback thresholds: min radius derived from `tolerance * 0.05` if unset.

## Adding a New Config Field (Example Steps)
1. Add field to `ConversionConfig` / `MachineConfig` / `PostprocessConfig` with default & (serde) attributes.
2. Mirror in WASM structs + conversion `From` impls.
3. Update CLI flags (with alias if consistent) and apply overlay.
4. Extend tests / JSON version examples if serialized.
5. Update web form state + UI components.

## Testing / Validation
- Core golden tests use parsed expected G-Code fixtures under `lib/tests/*.gcode`.
- When changing emission logic, regenerate or adjust expected fixtures thoughtfully; keep numerical tolerance in tests (see `assert_close`).
- Prefer adding a focused example binary under `lib/src/bin` for manual inspection of new behaviors.

## External Dependencies of Note
- `g-code` crate: Token model & formatting; rely on its `FormatOptions`—avoid reimplementing formatting.
- `roxmltree`: Non-mutable DOM traversal; maintain node filtering helpers when extending.
- `lyon_geom`: Curve flattening; keep tolerance consistent across arc + Bezier conversions.

## Pitfalls / Gotchas
- Forgetting to parse a new snippet field in CLI or WASM path silently disables it (empty snippet default); always round-trip test.
- Adding between-layer logic without respecting deferred sequencing breaks tool motion safety.
- Changing field order in JSON may break backward deserialization tests.

## Do / Don’t Quick List
- Do reuse `Machine::new` parameter order; don’t reorder without updating all callers.
- Do use `ConversionOptions` for dimension overrides & alignment; don’t bypass with ad-hoc transforms.
- Do keep tag naming scheme for WASM releases; don’t push an untagged version bump.

---
If any section is unclear or missing context you need, request clarification with the specific file or workflow you’re targeting.
