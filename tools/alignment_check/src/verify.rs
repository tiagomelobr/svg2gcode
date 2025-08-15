use roxmltree::Document;
use svg2gcode::{svg2program, ConversionConfig, ConversionOptions, HorizontalAlign as H, VerticalAlign as V, Machine, SupportedFunctionality};
use svgtypes::{Length, LengthUnit as LU};
use std::fs;

fn extents(gcode: &str) -> (f64,f64,f64,f64) {
    let (mut min_x, mut max_x, mut min_y, mut max_y)=(f64::INFINITY,f64::NEG_INFINITY,f64::INFINITY,f64::NEG_INFINITY);
    for line in gcode.lines(){
        let mut x=None; let mut y=None;
        for p in line.split_whitespace(){
            if let Some(v)=p.strip_prefix('X'){ if let Ok(f)=v.parse(){ x=Some(f);} }
            if let Some(v)=p.strip_prefix('Y'){ if let Ok(f)=v.parse(){ y=Some(f);} }
        }
        if let Some(xx)=x { min_x=min_x.min(xx); max_x=max_x.max(xx);} 
        if let Some(yy)=y { min_y=min_y.min(yy); max_y=max_y.max(yy);} 
    }
    (min_x,max_x,min_y,max_y)
}

// Own the SVG string to avoid temporary borrow lifetime issues.
struct Case { name: &'static str, svg: String, opts: ConversionOptions, check: Box<dyn Fn(f64,f64,f64,f64)->Result<(),String>> }

fn run(case:&Case){
    let doc=Document::parse(&case.svg).unwrap();
    let machine=Machine::new(SupportedFunctionality{circular_interpolation:false},None,None,None,None,None);
    let tokens=svg2program(&doc,&ConversionConfig::default(),case.opts.clone(),machine);
    let mut out=String::new();
    g_code::emit::format_gcode_fmt(tokens.iter(),Default::default(),&mut out).unwrap();
    let (min_x,max_x,min_y,max_y)=extents(&out);
    match (case.check)(min_x,max_x,min_y,max_y){
        Ok(())=>println!("PASS {} [{min_x:.2},{max_x:.2}] x [{min_y:.2},{max_y:.2}]",case.name),
        Err(e)=>println!("FAIL {} {} [{min_x:.2},{max_x:.2}] x [{min_y:.2},{max_y:.2}]\n{}",case.name,e,out)
    }
}

pub fn verify(){
    let mm=LU::Mm;
    // Preload all file contents as owned Strings. Use manifest dir for stable path when run from workspace root.
    let base = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let file = |p:&str| fs::read_to_string(format!("{base}/tests/data/{p}")) .unwrap();

    let cases:Vec<Case>=vec![
        Case{name:"square trim center-top", svg:file("square10.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:100.0,unit:mm}),Some(Length{number:50.0,unit:mm})],h_align:H::Center,v_align:V::Top,trim:true}, check:Box::new(|min_x,max_x,_,max_y|{ if (min_x-25.0).abs()<0.2 && (max_x-75.0).abs()<0.2 && (max_y-50.0).abs()<0.2 {Ok(())} else {Err("expected centered 25..75 & height 50".into())}})},
        Case{name:"square trim right-bottom", svg:file("square10.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:100.0,unit:mm}),Some(Length{number:50.0,unit:mm})],h_align:H::Right,v_align:V::Bottom,trim:true}, check:Box::new(|min_x,max_x,_,max_y|{ if (min_x-50.0).abs()<0.2 && (max_x-100.0).abs()<0.2 && (max_y-50.0).abs()<0.2 {Ok(())} else {Err("expected 50..100 bottom".into())}})},
        Case{name:"rect20x10 fit 120x60", svg:file("rect20x10.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:120.0,unit:mm}),Some(Length{number:60.0,unit:mm})],h_align:H::Center,v_align:V::Top,trim:true}, check:Box::new(|min_x,max_x,_,max_y|{ if min_x.abs()<0.3 && (max_x-120.0).abs()<0.3 && (max_y-60.0).abs()<0.3 {Ok(())} else {Err("expected full fill".into())}})},
        Case{name:"rect10x20 scaled center", svg:file("rect10x20.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:120.0,unit:mm}),Some(Length{number:60.0,unit:mm})],h_align:H::Center,v_align:V::Top,trim:true}, check:Box::new(|min_x,max_x,_,max_y|{ if (min_x-45.0).abs()<0.5 && (max_x-75.0).abs()<0.5 && (max_y-60.0).abs()<0.3 {Ok(())} else {Err("expected centered narrow".into())}})},
        Case{name:"triangle left-bottom", svg:file("triangle.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:80.0,unit:mm}),Some(Length{number:80.0,unit:mm})],h_align:H::Left,v_align:V::Bottom,trim:true}, check:Box::new(|min_x,_,min_y,_|{ if min_x.abs()<0.2 && min_y.abs()<0.2 {Ok(())} else {Err("expected origin".into())}})},
        Case{name:"circle no-trim center", svg:file("circle.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:100.0,unit:mm}),Some(Length{number:100.0,unit:mm})],h_align:H::Center,v_align:V::Center,trim:false}, check:Box::new(|min_x,max_x,min_y,max_y|{ let cx=(min_x+max_x)/2.0; let cy=(min_y+max_y)/2.0; if cx.abs()<1.0 && cy.abs()<1.0 {Ok(())} else {Err("expected centered".into())}})},
        Case{name:"ellipse trim center", svg:file("ellipse.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:200.0,unit:mm}),Some(Length{number:100.0,unit:mm})],h_align:H::Center,v_align:V::Center,trim:true}, check:Box::new(|min_x,max_x,min_y,max_y|{ if min_x.abs()<0.3 && (max_x-200.0).abs()<0.3 && min_y.abs()<0.3 && (max_y-100.0).abs()<0.3 {Ok(())} else {Err("expected full ellipse fit".into())}})},
        Case{name:"nested group trim left-top", svg:file("nested_group.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:60.0,unit:mm}),Some(Length{number:60.0,unit:mm})],h_align:H::Left,v_align:V::Top,trim:true}, check:Box::new(|min_x,max_x,_min_y,max_y|{ // allow some slack due to transform order nuances
            if (max_x-60.0).abs()<0.5 && (max_y-60.0).abs()<0.5 && min_x>=-0.5 && min_x<5.0 {Ok(())} else {Err("expected near top-left".into())}})},
        Case{name:"offset shape right-bottom", svg:file("offset_shape.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:100.0,unit:mm}),Some(Length{number:100.0,unit:mm})],h_align:H::Right,v_align:V::Bottom,trim:true}, check:Box::new(|min_x,max_x,_min_y,max_y|{ if (max_x-100.0).abs()<0.3 && (max_y-100.0).abs()<0.3 {Ok(())} else {Err(format!("expected anchored bottom-right got min_x={min_x}"))}})},
        Case{name:"complex center trim", svg:file("complex_paths.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:150.0,unit:mm}),Some(Length{number:150.0,unit:mm})],h_align:H::Center,v_align:V::Center,trim:true}, check:Box::new(|min_x,max_x,min_y,max_y|{ if min_x>-2.0 && (max_x-150.0).abs()<2.0 && min_y>-2.0 && (max_y-150.0).abs()<2.0 {Ok(())} else {Err("expected centered fill".into())}})},
        Case{name:"viewbox no-trim right-bottom", svg:file("viewbox_no_trim.svg"), opts:ConversionOptions{dimensions:[Some(Length{number:200.0,unit:mm}),Some(Length{number:80.0,unit:mm})],h_align:H::Right,v_align:V::Bottom,trim:false}, check:Box::new(|min_x,max_x,min_y,max_y|{ if min_x.abs()<0.5 && (max_x-200.0).abs()<0.5 && min_y.abs()<0.5 && (max_y-80.0).abs()<0.5 {Ok(())} else {Err("expected full viewport coverage".into())}})},
    ];

    for c in &cases { run(c); }
}
