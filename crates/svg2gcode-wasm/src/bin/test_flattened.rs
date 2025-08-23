use serde_json;
use svg2gcode_wasm::GCodeConversionOptions;

fn main() {
    let flattened_config = r#"{
  "tolerance": 0.002,
  "feedrate": 300.0,
  "dpi": 96.0,
  "origin_x": null,
  "origin_y": null,
  "min_arc_radius": null,
  "extra_attribute_name": null,
  "circular_interpolation": true,
  "tool_on_sequence": "M3 S1000",
  "tool_off_sequence": "M5",
  "begin_sequence": "",
  "end_sequence": "",
  "between_layers_sequence": "",
  "checksums": false,
  "line_numbers": false,
  "newline_before_comment": false,
  "override_width": null,
  "override_height": null,
  "h_align": null,
  "v_align": null,
  "trim": false
}"#;

    // Test 1: Basic flattened config
    println!("=== TEST 1: Basic Flattened Configuration ===");
    match serde_json::from_str::<GCodeConversionOptions>(flattened_config) {
        Ok(options) => {
            println!("✓ Flattened configuration parsed successfully!");
            println!("✓ Circular interpolation: {}", options.machine.circular_interpolation);
            println!("✓ Tool on sequence: {:?}", options.machine.tool_on_sequence);
            println!("✓ Tolerance: {}", options.conversion.tolerance);
        },
        Err(e) => {
            println!("✗ Error parsing flattened configuration: {}", e);
        }
    }

    // Test 2: Your actual settings export
    println!("\n=== TEST 2: Your Actual Settings Export ===");
    let your_settings = r#"{
  "dpi": 96,
  "trim": false,
  "useArcs": 0,
  "feedrate": 3000,
  "checksums": false,
  "tolerance": 0.02,
  "arcTolerance": 0.001,
  "line_numbers": false,
  "override_width": "",
  "override_height": "",
  "extra_attribute_name": "",
  "circularInterpolation": 1,
  "circular_interpolation": true,
  "newline_before_comment": true,
  "tool_on_sequence": "G4 P0.05\nG1 Z1\nG4 P0.05",
  "tool_off_sequence": "G4 P0.05\nG1 Z0\nG4 P0.2",
  "begin_sequence": "; Document Start\nG21\nG17\nG90\nF10000\nG0 Z0\nG4 P0.2\nG0 X0 Y0",
  "end_sequence": "; END\nG0 Z0\nG4 P0.2\nG0 X0 Y0 F10000",
  "between_layers_sequence": "M0",
  "origin_x": null,
  "origin_y": null,
  "min_arc_radius": null,
  "h_align": null,
  "v_align": null
}"#;

    match serde_json::from_str::<GCodeConversionOptions>(your_settings) {
        Ok(options) => {
            println!("✓ Your settings parsed successfully!");
            println!("✓ Circular interpolation: {}", options.machine.circular_interpolation);
            println!("✓ Tool on sequence: {:?}", options.machine.tool_on_sequence);
            println!("✓ Tool off sequence: {:?}", options.machine.tool_off_sequence);
            println!("✓ Begin sequence: {:?}", options.machine.begin_sequence);
            println!("✓ End sequence: {:?}", options.machine.end_sequence);
            println!("✓ Between layers: {:?}", options.machine.between_layers_sequence);
            println!("✓ Tolerance: {}", options.conversion.tolerance);
            println!("✓ Feedrate: {}", options.conversion.feedrate);
            println!("✓ DPI: {}", options.conversion.dpi);
            println!("✓ Checksums: {}", options.postprocess.checksums);
            println!("✓ Line numbers: {}", options.postprocess.line_numbers);
            println!("✓ Newline before comment: {}", options.postprocess.newline_before_comment);
        },
        Err(e) => {
            println!("✗ Error parsing your settings: {}", e);
            println!("Error details: {:?}", e);
        }
    }
}
