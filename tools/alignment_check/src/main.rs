use roxmltree::Document;
use svg2gcode::{svg2program, ConversionConfig, ConversionOptions, HorizontalAlign, VerticalAlign, Machine, SupportedFunctionality};

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

fn run_case(label:&str, opts: ConversionOptions, expect: impl Fn(f64,f64,f64,f64)->Result<(),String>) {
    let svg = "<svg viewBox='0 0 10 10'><path d='M0 0 L10 0 L10 10 L0 10 Z'/></svg>";
    let doc = Document::parse(svg).unwrap();
    let machine = Machine::new(SupportedFunctionality { circular_interpolation: false }, None,None,None,None,None);
    let tokens = svg2program(&doc, &ConversionConfig::default(), opts.clone(), machine);
    let mut out=String::new();
    g_code::emit::format_gcode_fmt(tokens.iter(), Default::default(), &mut out).unwrap();
    let (min_x,max_x,min_y,max_y)=extents(&out);
    match expect(min_x,max_x,min_y,max_y) {
        Ok(()) => println!("PASS {label}: [{min_x:.2},{max_x:.2}] x [{min_y:.2},{max_y:.2}]"),
        Err(msg) => {
            println!("FAIL {label}: {msg} -> [{min_x:.2},{max_x:.2}] x [{min_y:.2},{max_y:.2}]\nGCode:\n{out}");
        }
    }
}

fn approx(a:f64,b:f64,eps:f64)->bool { (a-b).abs() < eps }

fn main() {
    use svgtypes::{Length, LengthUnit};
    let mm = LengthUnit::Mm;

    // Scenarios derived from previous integration tests
    run_case(
        "trim center-top 100x50",
        ConversionOptions { dimensions:[Some(Length{number:100.0,unit:mm}), Some(Length{number:50.0,unit:mm})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Top, trim:true },
        |min_x,max_x,min_y,max_y| {
            if !approx(min_x,25.0,0.3) { return Err(format!("min_x expected ~25 got {min_x}")); }
            if !approx(max_x,75.0,0.3) { return Err(format!("max_x expected ~75 got {max_x}")); }
            if !approx(min_y,0.0,0.1) { return Err(format!("min_y expected 0 got {min_y}")); }
            if !approx(max_y,50.0,0.2) { return Err(format!("max_y expected 50 got {max_y}")); }
            Ok(())
        }
    );

    run_case(
        "trim right-bottom 100x50",
        ConversionOptions { dimensions:[Some(Length{number:100.0,unit:mm}), Some(Length{number:50.0,unit:mm})], h_align:HorizontalAlign::Right, v_align:VerticalAlign::Bottom, trim:true },
        |min_x,max_x,min_y,max_y| {
            if !approx(min_x,50.0,0.3) { return Err(format!("min_x expected ~50 got {min_x}")); }
            if !approx(max_x,100.0,0.3) { return Err(format!("max_x expected ~100 got {max_x}")); }
            if !approx(min_y,0.0,0.1) { return Err(format!("min_y expected 0 got {min_y}")); }
            if !approx(max_y,50.0,0.2) { return Err(format!("max_y expected 50 got {max_y}")); }
            Ok(())
        }
    );

    run_case(
        "trim left-top width=80",
        ConversionOptions { dimensions:[Some(Length{number:80.0,unit:mm}), None], h_align:HorizontalAlign::Left, v_align:VerticalAlign::Top, trim:true },
        |min_x,max_x,min_y,max_y| {
            if !approx(min_x,0.0,0.1) { return Err(format!("min_x expected 0 got {min_x}")); }
            if !approx(max_x,80.0,0.2) { return Err(format!("max_x expected 80 got {max_x}")); }
            if !approx(min_y,0.0,0.1) { return Err(format!("min_y expected 0 got {min_y}")); }
            if !approx(max_y,80.0,0.2) { return Err(format!("max_y expected 80 got {max_y}")); }
            Ok(())
        }
    );

    run_case(
        "dimensions no-trim center-center 100x50",
        ConversionOptions { dimensions:[Some(Length{number:100.0,unit:mm}), Some(Length{number:50.0,unit:mm})], h_align:HorizontalAlign::Center, v_align:VerticalAlign::Center, trim:false },
        |min_x,max_x,min_y,max_y| {
            let width = max_x - min_x; let height = max_y - min_y;
            if !(width <= 100.5 && width > 40.0) { return Err(format!("unexpected width {width}")); }
            if !(height <= 50.5 && height > 20.0) { return Err(format!("unexpected height {height}")); }
            Ok(())
        }
    );

    println!("Done. Any FAIL lines above indicate alignment math needs adjustment.");
    println!("Running extended verification suite...\n");
    // run extended cases
    alignment_check::verify::verify();
}
