use svg2gcode::{svg2program, ConversionConfig, ConversionOptions, HorizontalAlign, VerticalAlign, Machine, SupportedFunctionality};
use roxmltree::Document;

fn extract_extents(gcode: &str) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for line in gcode.lines() {
        let mut x_opt = None;
        let mut y_opt = None;
        for part in line.split_whitespace() {
            if let Some(val) = part.strip_prefix('X') { if let Ok(v) = val.parse::<f64>() { x_opt = Some(v); } }
            if let Some(val) = part.strip_prefix('Y') { if let Ok(v) = val.parse::<f64>() { y_opt = Some(v); } }
        }
        if let Some(x) = x_opt { if x < min_x { min_x = x; } if x > max_x { max_x = x; } }
        if let Some(y) = y_opt { if y < min_y { min_y = y; } if y > max_y { max_y = y; } }
    }
    (min_x, max_x, min_y, max_y)
}

fn run(svg: &str, options: ConversionOptions) -> String {
    let doc = Document::parse(svg).unwrap();
    let machine = Machine::new(SupportedFunctionality { circular_interpolation: false }, None, None, None, None, None);
    let tokens = svg2program(&doc, &ConversionConfig::default(), options, machine);
    let mut out = String::new();
    g_code::emit::format_gcode_fmt(tokens.iter(), Default::default(), &mut out).unwrap();
    out
}

#[test]
fn trim_center_top_alignment() {
    let svg = r#"<svg viewBox=\"0 0 10 10\"><path d=\"M0 0 L10 0 L10 10 L0 10 Z\"/></svg>"#;
    let options = ConversionOptions {
        dimensions: [Some(svgtypes::Length { number: 100.0, unit: svgtypes::LengthUnit::Mm }), Some(svgtypes::Length { number: 50.0, unit: svgtypes::LengthUnit::Mm })],
        h_align: HorizontalAlign::Center,
        v_align: VerticalAlign::Top,
        trim: true,
    };
    let gcode = run(svg, options);
    let (min_x, max_x, min_y, max_y) = extract_extents(&gcode);
    assert!((min_x - 25.0).abs() < 0.05, "min_x={min_x}");
    assert!((max_x - 75.0).abs() < 0.05, "max_x={max_x}");
    assert!((min_y - 0.0).abs() < 0.05, "min_y={min_y}");
    assert!((max_y - 50.0).abs() < 0.05, "max_y={max_y}");
}

#[test]
fn trim_right_bottom_alignment() {
    let svg = r#"<svg viewBox=\"0 0 10 10\"><path d=\"M0 0 L10 0 L10 10 L0 10 Z\"/></svg>"#;
    let options = ConversionOptions {
        dimensions: [Some(svgtypes::Length { number: 100.0, unit: svgtypes::LengthUnit::Mm }), Some(svgtypes::Length { number: 50.0, unit: svgtypes::LengthUnit::Mm })],
        h_align: HorizontalAlign::Right,
        v_align: VerticalAlign::Bottom,
        trim: true,
    };
    let gcode = run(svg, options);
    let (min_x, max_x, min_y, max_y) = extract_extents(&gcode);
    assert!((min_x - 50.0).abs() < 0.05);
    assert!((max_x - 100.0).abs() < 0.05);
    assert!((min_y - 0.0).abs() < 0.05);
    assert!((max_y - 50.0).abs() < 0.05);
}

#[test]
fn trim_only_width() {
    let svg = r#"<svg viewBox=\"0 0 10 10\"><path d=\"M0 0 L10 0 L10 10 L0 10 Z\"/></svg>"#;
    let options = ConversionOptions {
        dimensions: [Some(svgtypes::Length { number: 80.0, unit: svgtypes::LengthUnit::Mm }), None],
        h_align: HorizontalAlign::Left,
        v_align: VerticalAlign::Top,
        trim: true,
    };
    let gcode = run(svg, options);
    let (min_x, max_x, min_y, max_y) = extract_extents(&gcode);
    assert!((min_x - 0.0).abs() < 0.05);
    assert!((max_x - 80.0).abs() < 0.05);
    assert!((min_y - 0.0).abs() < 0.05);
    assert!((max_y - 80.0).abs() < 0.05);
}
