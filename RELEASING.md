## Release Guide (WASM / svg2gcode-wasm)

This guide codifies the exact process to publish a new WebAssembly (and optional npm) release without skipping critical parity or build checks. Follow it verbatim. If any step fails, stop and fix before proceeding.

### Scope
Applies to version bumps of `crates/svg2gcode-wasm` (npm & Git tag). Core crate (`lib`) has its own SemVer. Only bump WASM when public JS API / schema / behavior changes or you need to ship new core features to JS.

### TL;DR Checklist
1. Implement changes (core + wasm + web UI parity).
2. Ensure all new config fields are present in ALL layers:
   - `lib`: `ConversionConfig` / other structs
   - `crates/svg2gcode-wasm/src/lib.rs`: struct + `From` impl + schema doc comments
   - `web/src/state.rs`: form/state mapping (even if not yet exposed in UI inputs)
   - Update examples / README if user-facing
3. Update docs (`README.md` release section if needed).
4. Bump version in BOTH:
   - `crates/svg2gcode-wasm/Cargo.toml`
   - `crates/svg2gcode-wasm/package.json`
5. Build & test locally:
   - `cargo test` (workspace)
   - `cargo build -p svg2gcode-wasm --target wasm32-unknown-unknown --release`
   - `wasm-pack build --target bundler --out-dir pkg --out-name svg2gcode_wasm crates/svg2gcode-wasm` (ensures packaging still works)
6. Regenerate / inspect parameter schema (optional sanity):
   - Run a tiny snippet to call `param_schema_json()` (Node/browser or `wasm-pack test --node` if enabled) and diff against expectations.
7. Verify no unstaged changes:
   - `git status` must be clean except intentional edits.
8. Stage ONLY intended files (avoid accidental workspace noise):
   - `git add crates/svg2gcode-wasm/Cargo.toml crates/svg2gcode-wasm/package.json Cargo.lock README.md RELEASING.md <changed_sources>`
9. Run `git diff --cached --name-only` and confirm it includes the wasm `lib.rs` if you changed any fields.
10. Commit with message including version: `feat(wasm): <summary>; bump wasm to vX.Y.Z` (or `fix(wasm): ...`).
11. Tag AFTER the commit: `git tag vX.Y.Z`.
12. Push commit & tag: `git push origin main && git push origin vX.Y.Z`.
13. (Optional) Publish to npm: `cd crates/svg2gcode-wasm && npm publish`.

### Common Pitfalls & How to Avoid Them
| Pitfall | Prevention |
|---------|------------|
| Tag created before all files committed | Always run build & `git status` right before tagging; create tag only after commit hash is final. |
| Forgot to add new config field at one layer (e.g., `min_arc_radius`) | Use the parity checklist (Step 2). Search for the field name in repo: it should appear in core, wasm struct, web state, docs/example if user facing. |
| Package versions out of sync (`Cargo.toml` vs `package.json`) | Grep both before commit: `git grep "version = \"X.Y.Z\""` and `grep '"version": "X.Y.Z"'`. |
| Pushed tag referencing wrong commit | Compare: `git show vX.Y.Z` and ensure the diff contains your field changes. If wrong: do NOT move tag; bump patch version and redo correctly. |
| Schema missing new field | Run `param_schema_json()` and ensure new key present. |
| Accidentally included unrelated workspace changes | Stage files explicitly; review `git diff --cached`. |

### Detailed Steps (Narrative)
1. Implement feature / field in core crate (e.g., add `min_arc_radius` with serde default). Provide defaults to preserve backward compatibility.
2. Mirror field in WASM `ConversionConfig` with docs and `#[serde(default)]` if optional. Update `From<ConversionConfig>` mapping.
3. Mirror field in `web/src/state.rs` (state struct, `TryInto<Settings>`, `From<&Settings>`). Optionally add UI input later—state presence avoids deserialization gaps.
4. Update README examples showing new field if it’s relevant to users.
5. Bump versions (Cargo + npm) *after* code changes, so diff is clean.
6. Run all builds/tests. Fix any compile errors now.
7. Ensure no pending modifications: `git status` must show nothing unstaged. If files still modified—stage & re-run builds.
8. Commit & tag as described.
9. Push and monitor CI. If CI fails due to a missed file, DO NOT reuse the tag—create a new patch version (document the reason in commit message).

### Quick Verification Script (PowerShell)
```powershell
# Ensure versions sync and field parity for a given field (example: min_arc_radius)
$ver = (Select-String -Path crates/svg2gcode-wasm/Cargo.toml -Pattern '^version').Line.Split('=')[1].Trim() -replace '"',''
Write-Host "Cargo wasm version: $ver"
$pkgVer = (Get-Content crates/svg2gcode-wasm/package.json | ConvertFrom-Json).version
Write-Host "npm package version: $pkgVer"
if ($ver -ne $pkgVer) { Write-Error 'Version mismatch'; exit 1 }
$field = 'min_arc_radius'
$hits = git grep $field
if (-not $hits) { Write-Error "Field '$field' not found anywhere"; exit 1 }
$expectedFiles = @('lib/src/converter/mod.rs','crates/svg2gcode-wasm/src/lib.rs','web/src/state.rs')
$missing = $expectedFiles | Where-Object { -not (git grep --name-only $field | Select-String $_) }
if ($missing) { Write-Error "Parity missing in: $missing"; exit 1 }
Write-Host 'Parity + version checks passed.'
```

### When to Bump Which Part of SemVer
- `PATCH`: Bug fix, field addition with defaults (non-breaking), internal doc updates.
- `MINOR`: New user-facing config fields, new functionality visible in JS API.
- `MAJOR`: Breaking JS API change (renamed/removed fields or behavior contract changes).

### Rollback Strategy
If a bad tag is published:
- NEVER force-move the tag.
- Implement fix, bump patch version, follow the process correctly.
- Optionally mark the bad version as deprecated on npm (`npm deprecate`).

---
Adhering to this process should prevent recurrence of missing-field and premature tag issues.
