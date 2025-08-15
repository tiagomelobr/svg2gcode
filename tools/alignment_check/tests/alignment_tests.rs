use std::fs;
use roxmltree::Document;
use svg2gcode::{svg2program, ConversionConfig, ConversionOptions, HorizontalAlign, VerticalAlign, Machine, SupportedFunctionality};
use svgtypes::{Length, LengthUnit};

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

fn run(svg_path:&str, opts: ConversionOptions) -> (f64,f64,f64,f64) {
    let svg = fs::read_to_string(svg_path).expect("svg file");
    let doc = Document::parse(&svg).unwrap();
    let machine = Machine::new(SupportedFunctionality { circular_interpolation: false }, None,None,None,None,None);
    let tokens = svg2program(&doc, &ConversionConfig::default(), opts, machine);
    let mut out=String::new();
    g_code::emit::format_gcode_fmt(tokens.iter(), Default::default(), &mut out).unwrap();
    extents(&out)
}

const MM: LengthUnit = LengthUnit::Mm;

#[test]
fn square_center_top_trim() {
    let (min_x,max_x,_,max_y)=run("tests/data/square10.svg", ConversionOptions { dimensions:[Some(Length{number:100.0,unit:MM}), Some(Length{number:50.0,unit:MM})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Top, trim:true });
    assert!((min_x-25.0).abs()<0.2); assert!((max_x-75.0).abs()<0.2); assert!((max_y-50.0).abs()<0.2);
}

#[test]
fn square_right_bottom_trim() {
    let (min_x,max_x,_,max_y)=run("tests/data/square10.svg", ConversionOptions { dimensions:[Some(Length{number:100.0,unit:MM}), Some(Length{number:50.0,unit:MM})], h_align:HorizontalAlign::Right, v_align:VerticalAlign::Bottom, trim:true });
    assert!((min_x-50.0).abs()<0.2); assert!((max_x-100.0).abs()<0.2); assert!((max_y-50.0).abs()<0.2);
}

#[test]
fn rect_wider_center_trim() {
    let (min_x,max_x,_,max_y)=run("tests/data/rect20x10.svg", ConversionOptions { dimensions:[Some(Length{number:120.0,unit:MM}), Some(Length{number:60.0,unit:MM})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Top, trim:true });
    // aspect ratio 2:1 fitting into 120x60 -> scale = min(120/20,60/10)=6 so bbox 120x60 (exact fit)
    assert!((min_x-0.0).abs()<0.2); assert!((max_x-120.0).abs()<0.2); assert!((max_y-60.0).abs()<0.2);
}

#[test]
fn rect_taller_center_trim() {
    let (min_x,max_x,_,max_y)=run("tests/data/rect10x20.svg", ConversionOptions { dimensions:[Some(Length{number:120.0,unit:MM}), Some(Length{number:60.0,unit:MM})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Top, trim:true });
    // aspect ratio .5 fitting -> scale=min(120/10,60/20)=3 so bbox 30x60 centered horizontally
    assert!((min_x-45.0).abs()<0.3, "min_x={min_x}"); assert!((max_x-75.0).abs()<0.3, "max_x={max_x}"); assert!((max_y-60.0).abs()<0.2);
}

#[test]
fn triangle_left_bottom_trim() {
    let (min_x,_,min_y,_)=run("tests/data/triangle.svg", ConversionOptions { dimensions:[Some(Length{number:80.0,unit:MM}), Some(Length{number:80.0,unit:MM})], h_align:HorizontalAlign::Left, v_align:VerticalAlign::Bottom, trim:true });
    assert!(min_x.abs()<0.2); assert!(min_y.abs()<0.2);
}

#[test]
fn circle_center_no_trim() {
    let (min_x,max_x,min_y,max_y)=run("tests/data/circle.svg", ConversionOptions { dimensions:[Some(Length{number:100.0,unit:MM}), Some(Length{number:100.0,unit:MM})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Center, trim:false });
    // ViewBox square inside square override; just ensure roughly centered
    let cx=(min_x+max_x)/2.0; let cy=(min_y+max_y)/2.0; assert!(cx.abs()<1.0); assert!(cy.abs()<1.0);
}

#[test]
fn ellipse_center_trim() {
    let (min_x,max_x,min_y,max_y)=run("tests/data/ellipse.svg", ConversionOptions { dimensions:[Some(Length{number:200.0,unit:MM}), Some(Length{number:100.0,unit:MM})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Center, trim:true });
    assert!((min_x-0.0).abs()<0.2); assert!((max_x-200.0).abs()<0.2); assert!((min_y-0.0).abs()<0.2); assert!((max_y-100.0).abs()<0.2);
}

#[test]
fn nested_group_left_top_trim() {
    let (min_x,max_x,min_y,max_y)=run("tests/data/nested_group.svg", ConversionOptions { dimensions:[Some(Length{number:60.0,unit:MM}), Some(Length{number:60.0,unit:MM})], h_align:HorizontalAlign::Left, v_align:VerticalAlign::Top, trim:true });
    println!("nested_group bbox [{min_x},{max_x}] x [{min_y},{max_y}]");
    // inner rect 5x5 scaled by translate(5,5) then scale(2) => size 10 placed at (5,5) -> bbox 5..15 so after trim fit into 60 -> scale= min(60/10,60/10)=6 -> final 60 with left/top
    assert!((min_x-0.0).abs()<0.3); assert!((max_x-60.0).abs()<0.3); assert!((max_y-60.0).abs()<0.3); assert!((min_y-0.0).abs()<0.3);
}

#[test]
fn offset_shape_right_bottom_trim() {
    let (min_x,max_x,min_y,max_y)=run("tests/data/offset_shape.svg", ConversionOptions { dimensions:[Some(Length{number:100.0,unit:MM}), Some(Length{number:100.0,unit:MM})], h_align:HorizontalAlign::Right, v_align:VerticalAlign::Bottom, trim:true });
    println!("offset_shape bbox [{min_x},{max_x}] x [{min_y},{max_y}]");
    // shape from 10..30 -> width 20 -> scale = 100/30? actually bbox 10..30 inside 0..40 viewBox so trim uses tight 20x20 -> scale= min(100/20,100/20)=5 -> width 100 anchored right => min_x=0? right alignment after trim uses container 100 so min_x should be 0? wait right alignment sets dx= container - width - bbox.min.x ; bbox.min.x is 0 after scaling; so min_x=0; right==left when full width. Accept min_x near 0 and max_x 100
    assert!(min_x.abs()<0.2); assert!((max_x-100.0).abs()<0.2); assert!(min_y.abs()<0.2); assert!((max_y-100.0).abs()<0.2);
}

#[test]
fn complex_paths_center_trim() {
    let (min_x,max_x,min_y,max_y)=run("tests/data/complex_paths.svg", ConversionOptions { dimensions:[Some(Length{number:150.0,unit:MM}), Some(Length{number:150.0,unit:MM})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Center, trim:true });
    // tall shape (bars at y 0..5 and 45..50 plus circle) overall bbox 0..50 both axes -> scale = 150/50=3 -> width/height 150; center -> min ~0
    assert!(min_x>-1.0 && min_y>-1.0); assert!((max_x-150.0).abs()<1.0); assert!((max_y-150.0).abs()<1.0);
}

#[test]
fn viewbox_no_trim_dimensions_alignment() {
    let (min_x,max_x,min_y,max_y)=run("tests/data/viewbox_no_trim.svg", ConversionOptions { dimensions:[Some(Length{number:200.0,unit:MM}), Some(Length{number:80.0,unit:MM})], h_align:HorizontalAlign::Right, v_align:VerticalAlign::Bottom, trim:false });
    println!("viewbox_no_trim bbox [{min_x},{max_x}] x [{min_y},{max_y}]");
    // No trim: content should fill width 200 and height 80 exactly (viewBox scaled)
    assert!((min_x-0.0).abs()<0.2); assert!((max_x-200.0).abs()<0.2); assert!((min_y-0.0).abs()<0.2); assert!((max_y-80.0).abs()<0.2);
}
