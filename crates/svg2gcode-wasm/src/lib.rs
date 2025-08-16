use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json;
use svg2gcode::{
    svg2program, ConversionConfig as CoreConversionConfig, Machine,
    MachineConfig as CoreMachineConfig, PostprocessConfig as CorePostprocessConfig, Settings,
    SupportedFunctionality as CoreSupportedFunctionality, ConversionOptions, HorizontalAlign, VerticalAlign,
};
use wasm_bindgen::prelude::*;

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ConversionConfig {
    /// Curve interpolation tolerance in millimeters. Default: 0.002
    pub tolerance: f64,
    /// Machine feed rate in millimeters/minute. Default: 300.0
    pub feedrate: f64,
    /// Dots per inch for pixels, picas, points, etc. Default: 96.0
    pub dpi: f64,
    /// The X coordinate of the origin in millimeters. Default: 0.0
    pub origin_x: Option<f64>,
    /// The Y coordinate of the origin in millimeters. Default: 0.0
    pub origin_y: Option<f64>,
    /// Minimum arc radius (in mm) below which arcs are converted to line segments. If omitted, a
    /// conservative default derived from tolerance (tolerance * 0.05) is used.
    #[serde(default)]
    pub min_arc_radius: Option<f64>,
    /// An extra attribute to include in comments, for debugging. Default: None
    pub extra_attribute_name: Option<String>,
}

impl From<ConversionConfig> for CoreConversionConfig {
    fn from(config: ConversionConfig) -> Self {
        Self {
            tolerance: config.tolerance,
            feedrate: config.feedrate,
            dpi: config.dpi,
            origin: [config.origin_x, config.origin_y],
            min_arc_radius: config.min_arc_radius,
            extra_attribute_name: config.extra_attribute_name,
        }
    }
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct MachineConfig {
    /// Whether to use G2/G3 commands for circular interpolation. Default: false
    pub circular_interpolation: bool,
    /// G-Code sequence to turn the tool on. Default: None
    pub tool_on_sequence: Option<String>,
    /// G-Code sequence to turn the tool off. Default: None
    pub tool_off_sequence: Option<String>,
    /// G-Code sequence to run at the beginning of the program. Default: None
    pub begin_sequence: Option<String>,
    /// G-Code sequence to run at the end of the program. Default: None
    pub end_sequence: Option<String>,
    /// G-Code sequence to run between sibling SVG groups/layers. Default: None
    pub between_layers_sequence: Option<String>,
}

impl From<MachineConfig> for CoreMachineConfig {
    fn from(config: MachineConfig) -> Self {
        Self {
            supported_functionality: CoreSupportedFunctionality {
                circular_interpolation: config.circular_interpolation,
            },
            tool_on_sequence: config.tool_on_sequence,
            tool_off_sequence: config.tool_off_sequence,
            begin_sequence: config.begin_sequence,
            end_sequence: config.end_sequence,
            between_layers_sequence: config.between_layers_sequence,
        }
    }
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct PostprocessConfig {
    /// Whether to include checksums in the G-Code output. Default: false
    pub checksums: bool,
    /// Whether to include line numbers in the G-Code output. Default: false
    pub line_numbers: bool,
    /// Whether to include a newline before comments in the G-Code output. Default: false
    pub newline_before_comment: bool,
}

impl From<PostprocessConfig> for CorePostprocessConfig {
    fn from(config: PostprocessConfig) -> Self {
        Self {
            checksums: config.checksums,
            line_numbers: config.line_numbers,
            newline_before_comment: config.newline_before_comment,
        }
    }
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GCodeConversionOptions {
    #[serde(flatten)]
    pub conversion: ConversionConfig,
    #[serde(flatten)]
    pub machine: MachineConfig,
    #[serde(flatten)]
    pub postprocess: PostprocessConfig,
    /// Optional width override (e.g. "210mm"). If provided with height and trim=true acts like paper fit.
    pub override_width: Option<String>,
    /// Optional height override (e.g. "297mm").
    pub override_height: Option<String>,
    /// Horizontal alignment when an override dimension or trim is applied. left|center|right
    #[serde(default)]
    pub h_align: Option<String>,
    /// Vertical alignment when an override dimension or trim is applied. top|center|bottom
    #[serde(default)]
    pub v_align: Option<String>,
    /// If true, scales tight drawing bbox into the override dimensions (paper style fit).
    #[serde(default)]
    pub trim: bool,
}

impl GCodeConversionOptions {
    pub fn param_schema_json() -> String {
        let schema = schemars::schema_for!(GCodeConversionOptions);
        serde_json::to_string_pretty(&schema).unwrap()
    }
}

#[wasm_bindgen]
pub fn param_schema_json() -> String {
    GCodeConversionOptions::param_schema_json()
}

#[wasm_bindgen]
pub fn convert_svg(svg_str: &str, options: &JsValue) -> Result<String, String> {
    let options: GCodeConversionOptions =
        serde_wasm_bindgen::from_value(options.clone()).map_err(|e| e.to_string())?;

    let settings = Settings {
        conversion: options.conversion.into(),
        machine: options.machine.into(),
        postprocess: options.postprocess.into(),
        ..Default::default()
    };

    let doc = roxmltree::Document::parse(svg_str).map_err(|e| e.to_string())?;
    let machine = Machine::new(
        settings.machine.supported_functionality.clone(),
        settings.machine.tool_on_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
        settings.machine.tool_off_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
        settings.machine.begin_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
        settings.machine.end_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
    settings.machine.between_layers_sequence.as_deref().map(g_code::parse::snippet_parser).transpose().unwrap(),
    );

    // Build ConversionOptions from overrides
    let mut dimensions: [Option<svgtypes::Length>; 2] = [None, None];
    for (i, src) in [options.override_width.as_ref(), options.override_height.as_ref()].into_iter().enumerate() {
        if let Some(s) = src {
            if !s.is_empty() {
                if let Some(first) = svgtypes::LengthListParser::from(s.as_str()).next() { dimensions[i] = Some(first.map_err(|e| e.to_string())?); }
            }
        }
    }
    let h_align = match options.h_align.as_deref() { Some("center") => HorizontalAlign::Center, Some("right") => HorizontalAlign::Right, _ => HorizontalAlign::Left };
    let v_align = match options.v_align.as_deref() { Some("center") => VerticalAlign::Center, Some("bottom") => VerticalAlign::Bottom, _ => VerticalAlign::Top };
    let conv_options = ConversionOptions { dimensions, h_align, v_align, trim: options.trim };

    let gcode_tokens = svg2program(&doc, &settings.conversion, conv_options, machine);

    let mut gcode_out = String::new();
    g_code::emit::format_gcode_fmt(
        gcode_tokens.iter(),
        g_code::emit::FormatOptions {
            checksums: settings.postprocess.checksums,
            line_numbers: settings.postprocess.line_numbers,
            newline_before_comment: settings.postprocess.newline_before_comment,
            ..Default::default()
        },
        &mut gcode_out,
    )
    .map_err(|e| e.to_string())?;

    Ok(gcode_out)
}