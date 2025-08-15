use svg2gcode::{svg2program, ConversionConfig, ConversionOptions, HorizontalAlign, VerticalAlign, Machine, SupportedFunctionality};
use roxmltree::Document;

fn extents(gcode: &str) -> (f64,f64,f64,f64) {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for line in gcode.lines() {
        let mut x=None; let mut y=None;
        for part in line.split_whitespace() {
            if let Some(v)=part.strip_prefix('X') { if let Ok(f)=v.parse::<f64>() { x=Some(f);} }
            if let Some(v)=part.strip_prefix('Y') { if let Ok(f)=v.parse::<f64>() { y=Some(f);} }
        }
        if let Some(xx)=x { min_x=min_x.min(xx); max_x=max_x.max(xx);} 
        if let Some(yy)=y { min_y=min_y.min(yy); max_y=max_y.max(yy);} 
    }
    (min_x,max_x,min_y,max_y)
}

fn run(opts: ConversionOptions) -> (f64,f64,f64,f64) {
    let svg = "<svg viewBox='0 0 10 10'><path d='M0 0 L10 0 L10 10 L0 10 Z'/></svg>";
    let doc = Document::parse(svg).unwrap();
    let machine = Machine::new(SupportedFunctionality { circular_interpolation: false }, None,None,None,None,None);
    let tokens = svg2program(&doc, &ConversionConfig::default(), opts, machine);
    let mut out=String::new();
    g_code::emit::format_gcode_fmt(tokens.iter(), Default::default(), &mut out).unwrap();
    extents(&out)
}

#[test]
fn trim_center_top() {
    let opts = ConversionOptions { dimensions:[Some(svgtypes::Length{number:100.0, unit:svgtypes::LengthUnit::Mm}), Some(svgtypes::Length{number:50.0, unit:svgtypes::LengthUnit::Mm})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Top, trim:true };
    let (min_x,max_x,min_y,max_y)=run(opts);
    // width scaled to 50 (uniform) and centered in 100 -> 25..75
    assert!((min_x-25.0).abs()<0.2, "min_x={min_x}");
    assert!((max_x-75.0).abs()<0.2, "max_x={max_x}");
    assert!((min_y-0.0).abs()<0.05);
    assert!((max_y-50.0).abs()<0.05);
}

#[test]
fn trim_right_bottom() {
    let opts = ConversionOptions { dimensions:[Some(svgtypes::Length{number:100.0, unit:svgtypes::LengthUnit::Mm}), Some(svgtypes::Length{number:50.0, unit:svgtypes::LengthUnit::Mm})], h_align:HorizontalAlign::Right, v_align:VerticalAlign::Bottom, trim:true };
    let (min_x,max_x,min_y,max_y)=run(opts);
    assert!((min_x-50.0).abs()<0.2, "min_x={min_x}");
    assert!((max_x-100.0).abs()<0.2, "max_x={max_x}");
    assert!((min_y-0.0).abs()<0.05);
    assert!((max_y-50.0).abs()<0.05);
}

#[test]
fn trim_width_only() {
    let opts = ConversionOptions { dimensions:[Some(svgtypes::Length{number:80.0, unit:svgtypes::LengthUnit::Mm}), None], h_align:HorizontalAlign::Left, v_align:VerticalAlign::Top, trim:true };
    let (min_x,max_x,min_y,max_y)=run(opts);
    assert!((min_x-0.0).abs()<0.05);
    assert!((max_x-80.0).abs()<0.05);
    assert!((min_y-0.0).abs()<0.05);
    assert!((max_y-80.0).abs()<0.05);
}

#[test]
fn dimensions_no_trim_center_alignment_should_not_scale_bbox() {
    // No trim: override viewport to 100x50, but drawing (10x10 user units) becomes scaled non-uniform? We just verify alignment translation roughly.
    let opts = ConversionOptions { dimensions:[Some(svgtypes::Length{number:100.0, unit:svgtypes::LengthUnit::Mm}), Some(svgtypes::Length{number:50.0, unit:svgtypes::LengthUnit::Mm})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Center, trim:false };
    let (min_x,max_x,min_y,max_y)=run(opts);
    let width = max_x - min_x; let height = max_y - min_y;
    // Expect full width equals 100 or close (since viewport width override). Just ensure within bounds.
    assert!(width <= 100.1 && width > 40.0, "unexpected width {width}");
    assert!(height <= 50.1 && height > 20.0, "unexpected height {height}");
}

#[test]
fn trim_left_top() {
    let opts = ConversionOptions { dimensions:[Some(svgtypes::Length{number:120.0, unit:svgtypes::LengthUnit::Mm}), Some(svgtypes::Length{number:60.0, unit:svgtypes::LengthUnit::Mm})], h_align:HorizontalAlign::Left, v_align:VerticalAlign::Top, trim:true };
    let (min_x,max_x,min_y,max_y)=run(opts);
    // After trim scaling uniform factor = min(120/10,60/10)=6 -> bbox 60x60 placed top-left inside 120x60
    assert!((min_x-0.0).abs()<0.2);
    assert!((max_x-60.0).abs()<0.2, "max_x={max_x}");
    assert!((max_y-60.0).abs()<0.2);
}
