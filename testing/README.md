# SVG2GCode Testing Framework

This folder contains comprehensive testing scripts and resources for svg2gcode, specifically designed for AI agents and developers to validate polygon arc detection and G-code generation features.

## Folder Structure

```
testing/
├── README.md                    # This file - main testing documentation
├── INSTRUCTIONS.md              # Quick start guide for AI agents
├── svg-samples/                 # Test SVG files organized by type
│   ├── polygonal/              # SVGs with polygonal shapes (ideal for arc detection)
│   ├── curved/                 # SVGs with native curves
│   └── mixed/                  # SVGs with both curves and polygons
├── settings/                   # Configuration files for different test scenarios
│   ├── polygon_arc_enabled.json    # Settings with polygon arc detection ON
│   ├── polygon_arc_disabled.json   # Settings with polygon arc detection OFF
│   ├── high_precision.json         # High precision settings for detailed testing
│   └── production.json              # Production-ready settings
├── scripts/                    # Testing and analysis scripts
│   ├── run_tests.ps1          # Main PowerShell test runner
│   ├── run_tests.sh           # Main Bash test runner
│   ├── analyze_output.ps1     # G-code analysis script (PowerShell)
│   ├── analyze_output.sh      # G-code analysis script (Bash)
│   ├── validate_schema.js     # WASM schema validation
│   └── benchmark.ps1          # Performance benchmarking
└── output/                     # Generated G-code files (auto-created)
    ├── with_arcs/             # Output with polygon arc detection enabled
    └── without_arcs/          # Output with polygon arc detection disabled
```

## Key Features Tested

### 1. Polygon Arc Detection
- **Purpose**: Converts polygon segments into G2/G3 arc commands when they approximate circles
- **Parameters**:
  - `detect_polygon_arcs`: Enable/disable the feature
  - `min_polygon_arc_points`: Minimum points to consider for arc detection (default: 5)
  - `polygon_arc_tolerance`: Maximum deviation tolerance (default: uses main tolerance)

### 2. G-code Quality Analysis
- **Arc Usage Percentage**: Ratio of G2/G3 commands to total movement commands
- **Command Statistics**: Count of G0, G1, G2, G3 commands
- **File Size Optimization**: Comparison of file sizes with/without arc detection
- **Tool Sequence Validation**: Proper tool on/off sequences

### 3. Cross-Platform Compatibility
- PowerShell scripts for Windows environments
- Bash scripts for Linux/macOS environments
- Node.js scripts for WASM interface testing

## Test Categories

### Functional Tests
1. **Basic Arc Detection**: Verify arcs are detected in simple polygonal shapes
2. **Parameter Validation**: Test different configuration combinations
3. **Edge Cases**: Handle degenerate polygons, very small arcs, etc.
4. **Integration**: CLI, WASM, and web interface consistency

### Performance Tests
1. **Processing Speed**: Compare execution times with/without arc detection
2. **Memory Usage**: Monitor resource consumption
3. **Output Size**: Measure G-code file size optimization

### Regression Tests
1. **Backward Compatibility**: Ensure existing functionality remains unchanged
2. **Schema Validation**: Verify WASM interface includes all new fields
3. **Settings Migration**: Test configuration file upgrades

## Usage for AI Agents

### Quick Testing Workflow
1. Navigate to the `testing/` folder
2. Run `scripts/run_tests.ps1` (Windows) or `scripts/run_tests.sh` (Linux/macOS)
3. Review results in `output/` folder
4. Analyze statistics using `scripts/analyze_output.ps1`

### Validation Checklist
- [ ] Arc detection enabled/disabled toggle works
- [ ] Generated G-code contains proper G2/G3 commands
- [ ] Tool sequences are correct (M3/M5 or custom)
- [ ] File size is reduced when arcs are detected
- [ ] No regression in existing curve handling
- [ ] WASM schema includes new polygon arc fields

### Key Metrics to Monitor
- **Arc Usage %**: Should be >20% for polygonal SVGs with circular features
- **Command Reduction**: G-code files should be smaller with arc detection
- **Processing Time**: Should remain under 2x of original processing time
- **Error Rate**: Zero parsing or generation errors

## Configuration Examples

### Enable Polygon Arc Detection
```json
{
  "conversion": {
    "detect_polygon_arcs": true,
    "min_polygon_arc_points": 5,
    "polygon_arc_tolerance": 0.001
  },
  "machine": {
    "circular_interpolation": true
  }
}
```

### Disable for Comparison
```json
{
  "conversion": {
    "detect_polygon_arcs": false
  },
  "machine": {
    "circular_interpolation": true
  }
}
```

## Troubleshooting

### Common Issues
1. **No arcs detected**: Check `circular_interpolation` is enabled and polygon has enough points
2. **High processing time**: Reduce `polygon_arc_tolerance` or increase `min_polygon_arc_points`
3. **Schema missing fields**: Rebuild WASM package with `wasm-pack build`

### Debug Commands
```bash
# Build and test core library
cargo test -p svg2gcode

# Test CLI with verbose output
cargo run --bin svg2gcode -- --help

# Validate WASM schema
node testing/scripts/validate_schema.js
```

## Development Notes

This testing framework was developed to validate the polygon arc detection feature implementation. It covers the complete testing pipeline from SVG input to G-code analysis, ensuring robust validation of new features while maintaining backward compatibility.

For release validation, ensure all tests pass before bumping WASM version numbers.
