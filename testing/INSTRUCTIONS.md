# Quick Instructions for AI Agents

## Objective
Test svg2gcode polygon arc detection feature and validate G-code generation quality.

## Quick Start (30 seconds)
```bash
cd testing/
./scripts/run_tests.ps1    # Windows
./scripts/run_tests.sh     # Linux/macOS
```

## What Gets Tested
1. **Polygon Arc Detection**: Converts polygon segments to G2/G3 arc commands
2. **G-code Quality**: Measures arc usage, file size reduction, command counts
3. **Schema Validation**: Ensures WASM interface exposes new configuration fields

## Key Files
- `scripts/run_tests.ps1`: Main test runner (PowerShell)
- `scripts/analyze_output.ps1`: G-code analysis and statistics
- `settings/polygon_arc_enabled.json`: Test configuration with arc detection ON
- `svg-samples/`: Test SVG files (polygonal shapes ideal for arc detection)

## Success Criteria
- Arc usage >20% for polygonal SVGs
- G-code files smaller with arc detection enabled
- Zero parsing/generation errors
- WASM schema includes: `detect_polygon_arcs`, `min_polygon_arc_points`, `polygon_arc_tolerance`

## Troubleshooting
- **No arcs detected**: Ensure `circular_interpolation: true` in settings
- **Schema validation fails**: Rebuild WASM with `wasm-pack build --target web --out-dir pkg ../crates/svg2gcode-wasm`
- **Scripts fail**: Run from workspace root: `cd C:\path\to\svg2gcode`

## Manual Testing
```bash
# Test single SVG with arc detection
cargo run --bin svg2gcode -- testing/svg-samples/polygonal/circles.svg --settings testing/settings/polygon_arc_enabled.json --out output.gcode

# Analyze result
Select-String "G[23]" output.gcode | Measure-Object | % Count  # PowerShell
grep -c "G[23]" output.gcode                                   # Bash
```

## Release Validation
Before WASM version bump:
1. All tests pass ✓
2. Schema includes new fields ✓  
3. No regression in existing features ✓
4. Arc detection works as expected ✓
