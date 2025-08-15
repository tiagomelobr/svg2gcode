use svg2gcode::{svg2program, ConversionConfig, ConversionOptions, HorizontalAlign, VerticalAlign, Machine, SupportedFunctionality};
use roxmltree::Document;

fn main() {
    let svg = "<svg viewBox='0 0 10 10'><path d='M0 0 L10 0 L10 10 L0 10 Z'/></svg>";
    let doc = Document::parse(svg).unwrap();
    let machine = Machine::new(SupportedFunctionality { circular_interpolation: false }, None, None, None, None, None);

    let configs = [
        ("center-top trim to 100x50", ConversionOptions { dimensions: [Some(svgtypes::Length { number: 100.0, unit: svgtypes::LengthUnit::Mm }), Some(svgtypes::Length { number: 50.0, unit: svgtypes::LengthUnit::Mm })], h_align: HorizontalAlign::Center, v_align: VerticalAlign::Top, trim: true }),
        ("right-bottom trim to 100x50", ConversionOptions { dimensions: [Some(svgtypes::Length { number: 100.0, unit: svgtypes::LengthUnit::Mm }), Some(svgtypes::Length { number: 50.0, unit: svgtypes::LengthUnit::Mm })], h_align: HorizontalAlign::Right, v_align: VerticalAlign::Bottom, trim: true }),
        ("left-top trim width 80", ConversionOptions { dimensions: [Some(svgtypes::Length { number: 80.0, unit: svgtypes::LengthUnit::Mm }), None], h_align: HorizontalAlign::Left, v_align: VerticalAlign::Top, trim: true }),
    ];

    for (label, opts) in configs { 
        let tokens = svg2program(&doc, &ConversionConfig::default(), opts.clone(), machine.clone());
        let mut out = String::new();
        g_code::emit::format_gcode_fmt(tokens.iter(), Default::default(), &mut out).unwrap();
        println!("===== {label} =====\n{out}");
    }
}
