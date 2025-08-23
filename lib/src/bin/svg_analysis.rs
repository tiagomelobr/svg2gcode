use std::fs;
use roxmltree::Document;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SVG Content Analysis ===\n");
    
    let svg_files = ["svg-1.svg", "svg-2.svg"];
    
    for svg_file in &svg_files {
        println!("🔍 Analyzing file: {}", svg_file);
        
        let svg_path = format!("../{}", svg_file);
        let svg_content = match fs::read_to_string(&svg_path) {
            Ok(content) => content,
            Err(e) => {
                println!("❌ Error reading {}: {}", svg_file, e);
                continue;
            }
        };
        
        println!("📄 File size: {} characters", svg_content.len());
        
        // Parse SVG
        let doc = match Document::parse(&svg_content) {
            Ok(doc) => doc,
            Err(e) => {
                println!("❌ Error parsing SVG: {}", e);
                continue;
            }
        };
        
        // Detailed analysis
        analyze_svg_content(&svg_content);
        analyze_svg_structure(&doc);
        
        // Sample path data analysis
        sample_path_data(&svg_content);
        
        println!("\n{}\n", "=".repeat(60));
    }
    
    Ok(())
}

fn analyze_svg_content(svg_content: &str) {
    println!("\n--- Content Analysis ---");
    
    // Count various SVG elements
    let circles = svg_content.matches("<circle").count();
    let ellipses = svg_content.matches("<ellipse").count();
    let rects = svg_content.matches("<rect").count();
    let paths = svg_content.matches("<path").count();
    let polygons = svg_content.matches("<polygon").count();
    let polylines = svg_content.matches("<polyline").count();
    let lines = svg_content.matches("<line").count();
    
    println!("🔹 Circles: {}", circles);
    println!("🔹 Ellipses: {}", ellipses);
    println!("🔹 Rectangles: {}", rects);
    println!("🔹 Paths: {}", paths);
    println!("🔹 Polygons: {}", polygons);
    println!("🔹 Polylines: {}", polylines);
    println!("🔹 Lines: {}", lines);
    
    // Count path commands
    let move_commands = svg_content.matches(" M ").count() + svg_content.matches(" m ").count();
    let line_commands = svg_content.matches(" L ").count() + svg_content.matches(" l ").count();
    let curve_commands = svg_content.matches(" C ").count() + svg_content.matches(" c ").count() +
                        svg_content.matches(" S ").count() + svg_content.matches(" s ").count() +
                        svg_content.matches(" Q ").count() + svg_content.matches(" q ").count() +
                        svg_content.matches(" T ").count() + svg_content.matches(" t ").count();
    let arc_commands = svg_content.matches(" A ").count() + svg_content.matches(" a ").count();
    let close_commands = svg_content.matches(" Z").count() + svg_content.matches(" z").count();
    
    println!("\n--- Path Commands ---");
    println!("🔸 Move (M/m): {}", move_commands);
    println!("🔸 Line (L/l): {}", line_commands);
    println!("🔸 Curves (C/c/S/s/Q/q/T/t): {}", curve_commands);
    println!("🔸 Arcs (A/a): {}", arc_commands);
    println!("🔸 Close (Z/z): {}", close_commands);
}

fn analyze_svg_structure(doc: &Document) {
    println!("\n--- Document Structure ---");
    
    let mut path_count = 0;
    let mut circle_count = 0;
    let mut rect_count = 0;
    let mut other_count = 0;
    
    for node in doc.descendants() {
        match node.tag_name().name() {
            "path" => path_count += 1,
            "circle" => circle_count += 1,
            "rect" => rect_count += 1,
            "ellipse" | "polygon" | "polyline" | "line" => other_count += 1,
            _ => {}
        }
    }
    
    println!("🔷 Total path elements: {}", path_count);
    println!("🔷 Total circle elements: {}", circle_count);
    println!("🔷 Total rect elements: {}", rect_count);
    println!("🔷 Other shape elements: {}", other_count);
    
    // Check for transforms
    let mut transform_count = 0;
    for node in doc.descendants() {
        if node.attribute("transform").is_some() {
            transform_count += 1;
        }
    }
    println!("🔷 Elements with transforms: {}", transform_count);
}

fn sample_path_data(svg_content: &str) {
    println!("\n--- Sample Path Data ---");
    
    // Find first few path elements and show their data
    let mut path_samples = Vec::new();
    let mut in_path = false;
    let mut current_path = String::new();
    
    for line in svg_content.lines().take(1000) { // First 1000 lines only
        if line.contains("<path") {
            in_path = true;
            current_path.clear();
        }
        
        if in_path {
            current_path.push_str(line);
            current_path.push('\n');
            
            if line.contains("/>") || line.contains("</path>") {
                in_path = false;
                path_samples.push(current_path.clone());
                
                if path_samples.len() >= 3 {
                    break;
                }
            }
        }
    }
    
    for (i, path) in path_samples.iter().enumerate() {
        println!("📋 Path {} sample:", i + 1);
        
        // Extract just the d attribute if present
        if let Some(d_start) = path.find("d=\"") {
            let d_start = d_start + 3;
            if let Some(d_end) = path[d_start..].find("\"") {
                let d_attr = &path[d_start..d_start + d_end];
                let truncated = if d_attr.len() > 200 {
                    format!("{}...", &d_attr[..200])
                } else {
                    d_attr.to_string()
                };
                println!("   d=\"{}\"", truncated);
            }
        } else {
            // Show truncated path element
            let truncated = if path.len() > 300 {
                format!("{}...", &path[..300])
            } else {
                path.clone()
            };
            println!("   {}", truncated.trim());
        }
        println!();
    }
}
