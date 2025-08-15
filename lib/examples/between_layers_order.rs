use svg2gcode::{svg2program, ConversionConfig, ConversionOptions, Machine, MachineConfig, PostprocessConfig, Settings, SupportedFunctionality};

fn main() {
    // SVG with two group layers each containing a simple path
    let svg = r#"<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10' viewBox='0 0 10 10'>
<g id='layer1'><path d='M1 1 L9 1'/></g>
<g id='layer2'><path d='M1 9 L9 9'/></g>
</svg>"#;

    let doc = roxmltree::Document::parse(svg).unwrap();

    let mut settings = Settings::default();
    settings.conversion = ConversionConfig { tolerance: 0.002, feedrate: 300.0, dpi: 96.0, origin: [None,None], extra_attribute_name: None };
    settings.machine = MachineConfig {
        supported_functionality: SupportedFunctionality { circular_interpolation: false },
        tool_on_sequence: Some("M3".into()),
        tool_off_sequence: Some("M5".into()),
        begin_sequence: None,
        end_sequence: None,
        between_layers_sequence: Some("(BL)".into()),
    };
    settings.postprocess = PostprocessConfig { checksums: false, line_numbers: false, newline_before_comment: false };

    let machine = Machine::new(
        settings.machine.supported_functionality.clone(),
        settings.machine.tool_on_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
        settings.machine.tool_off_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
        settings.machine.begin_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
        settings.machine.end_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
        settings.machine.between_layers_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
    );

    let tokens = svg2program(&doc, &settings.conversion, ConversionOptions::default(), machine);

    let mut out = String::new();
    g_code::emit::format_gcode_fmt(tokens.iter(), g_code::emit::FormatOptions::default(), &mut out).unwrap();
    println!("{}", out);
}
