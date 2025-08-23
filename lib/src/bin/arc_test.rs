use std::fs;
use svg2gcode::{svg2program, ConversionConfig, Settings, Machine, SupportedFunctionality, ConversionOptions};
use roxmltree::Document;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SVG Arc Conversion Testing ===\n");
    
    // Test files
    let svg_files = ["svg-1.svg", "svg-2.svg"];
    
    // Different tolerance settings to test
    let tolerance_settings = [
        0.001,  // Very precise
        0.002,  // Default
        0.005,  // Medium
        0.01,   // Relaxed
        0.02,   // Your current setting
        0.05,   // Very relaxed
        0.1,    // Coarse
    ];
    
    // Different min_arc_radius settings
    let min_arc_radius_settings = [
        None,           // Auto (tolerance * 0.05)
        Some(0.001),    // Very small arcs
        Some(0.01),     // Small arcs
        Some(0.1),      // Medium arcs
        Some(0.5),      // Large arcs only
    ];
    
    for svg_file in &svg_files {
        println!("🔍 Testing file: {}", svg_file);
        
        // Read SVG content
        let svg_path = format!("../{}", svg_file);
        let svg_content = match fs::read_to_string(&svg_path) {
            Ok(content) => content,
            Err(e) => {
                println!("❌ Error reading {}: {}", svg_file, e);
                continue;
            }
        };
        
        println!("📄 SVG file size: {} characters", svg_content.len());
        
        // Parse SVG
        let doc = match Document::parse(&svg_content) {
            Ok(doc) => doc,
            Err(e) => {
                println!("❌ Error parsing SVG {}: {}", svg_file, e);
                continue;
            }
        };
        
        // Count potential arc indicators in SVG
        let arc_indicators = count_arc_indicators(&svg_content);
        println!("🎯 Arc indicators found: {:?}", arc_indicators);
        
        println!("\n--- Testing Tolerance Settings ---");
        
        // Test different tolerance settings with auto min_arc_radius
        for &tolerance in &tolerance_settings {
            test_conversion(
                &doc,
                tolerance,
                None, // auto min_arc_radius
                &format!("Tolerance: {:.3}", tolerance)
            );
        }
        
        println!("\n--- Testing Min Arc Radius Settings (tolerance=0.002) ---");
        
        // Test different min_arc_radius settings with default tolerance
        for &min_arc_radius in &min_arc_radius_settings {
            let description = match min_arc_radius {
                None => "Min arc radius: Auto".to_string(),
                Some(val) => format!("Min arc radius: {:.3}", val),
            };
            
            test_conversion(
                &doc,
                0.002, // default tolerance
                min_arc_radius,
                &description
            );
        }
        
        println!("\n--- Best Combination Test ---");
        
        // Test what might be the best combination for arc detection
        test_conversion(
            &doc,
            0.001,      // High precision
            Some(0.001), // Allow very small arcs
            "Best combo: High precision + Small arcs"
        );
        
        println!("\n{}\n", "=".repeat(60));
    }
    
    Ok(())
}

fn count_arc_indicators(svg_content: &str) -> ArcIndicators {
    ArcIndicators {
        circles: svg_content.matches("<circle").count(),
        ellipses: svg_content.matches("<ellipse").count(),
        arc_commands: svg_content.matches(" A ").count() + svg_content.matches(" a ").count(),
        path_elements: svg_content.matches("<path").count(),
        curves: svg_content.matches(" C ").count() + svg_content.matches(" c ").count() +
                svg_content.matches(" Q ").count() + svg_content.matches(" q ").count() +
                svg_content.matches(" S ").count() + svg_content.matches(" s ").count() +
                svg_content.matches(" T ").count() + svg_content.matches(" t ").count(),
    }
}

#[derive(Debug)]
struct ArcIndicators {
    circles: usize,
    ellipses: usize,
    arc_commands: usize,
    path_elements: usize,
    curves: usize,
}

fn test_conversion(doc: &Document, tolerance: f64, min_arc_radius: Option<f64>, description: &str) {
    let conversion_config = ConversionConfig {
        tolerance,
        feedrate: 3000.0,
        dpi: 96.0,
        origin: [None, None],
        min_arc_radius,
        extra_attribute_name: None,
        detect_polygon_arcs: false,
        min_polygon_arc_points: 5,
        polygon_arc_tolerance: None,
    };
    
    let machine = Machine::new(
        SupportedFunctionality {
            circular_interpolation: true,
        },
        Some(g_code::parse::snippet_parser("G4 P0.05\nG1 Z1\nG4 P0.05").unwrap()),
        Some(g_code::parse::snippet_parser("G4 P0.05\nG1 Z0\nG4 P0.2").unwrap()),
        Some(g_code::parse::snippet_parser("; Document Start\nG21\nG17\nG90\nF10000\nG0 Z0\nG4 P0.2\nG0 X0 Y0").unwrap()),
        Some(g_code::parse::snippet_parser("; END\nG0 Z0\nG4 P0.2\nG0 X0 Y0 F10000").unwrap()),
        Some(g_code::parse::snippet_parser("M0").unwrap()),
    );
    
    let conversion_options = ConversionOptions::default();
    
    let gcode_tokens = svg2program(doc, &conversion_config, conversion_options, machine);
    
    // Convert tokens to string for analysis
    let mut gcode_output = String::new();
    g_code::emit::format_gcode_fmt(
        gcode_tokens.iter(),
        g_code::emit::FormatOptions {
            checksums: false,
            line_numbers: false,
            newline_before_comment: true,
            ..Default::default()
        },
        &mut gcode_output,
    ).unwrap();
    
    let stats = analyze_gcode(&gcode_output);
    println!("✅ {}: G1={}, G2={}, G3={}, Total lines={}", 
        description, stats.g1_count, stats.g2_count, stats.g3_count, stats.total_lines);
}

#[derive(Debug)]
struct GCodeStats {
    g1_count: usize,
    g2_count: usize,
    g3_count: usize,
    total_lines: usize,
}

fn analyze_gcode(gcode: &str) -> GCodeStats {
    let lines: Vec<&str> = gcode.lines().collect();
    let total_lines = lines.len();
    
    let mut g1_count = 0;
    let mut g2_count = 0;
    let mut g3_count = 0;
    
    for line in &lines {
        let line = line.trim();
        if line.starts_with("G1 ") || line.starts_with("G01 ") {
            g1_count += 1;
        } else if line.starts_with("G2 ") || line.starts_with("G02 ") {
            g2_count += 1;
        } else if line.starts_with("G3 ") || line.starts_with("G03 ") {
            g3_count += 1;
        }
    }
    
    GCodeStats {
        g1_count,
        g2_count,
        g3_count,
        total_lines,
    }
}
