use std::fmt::Debug;

use g_code::emit::Token;
use lyon_geom::euclid::default::Transform2D;
use roxmltree::{Document, Node};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use svgtypes::Length;
use uom::si::f64::Length as UomLength;
use uom::si::length::{inch, millimeter, centimeter, pica_computer};

use self::units::CSS_DEFAULT_DPI;
use crate::{turtle::*, Machine};

#[cfg(feature = "serde")]
mod length_serde;
mod path;
mod transform;
mod units;
mod visit;

/// High-level output configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConversionConfig {
    /// Curve interpolation tolerance in millimeters
    pub tolerance: f64,
    /// Feedrate in millimeters / minute
    pub feedrate: f64,
    /// Dots per inch for pixels, picas, points, etc.
    pub dpi: f64,
    /// Set the origin point in millimeters for this conversion
    #[cfg_attr(feature = "serde", serde(default = "zero_origin"))]
    pub origin: [Option<f64>; 2],
    /// Set extra attribute to add when printing node name
    pub extra_attribute_name: Option<String>,
}

const fn zero_origin() -> [Option<f64>; 2] {
    [Some(0.); 2]
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            tolerance: 0.002,
            feedrate: 300.0,
            dpi: 96.0,
            origin: zero_origin(),
	    extra_attribute_name : None,
        }
    }
}

/// Options are specific to this conversion.
///
/// This is separate from [ConversionConfig] to support bulk processing in the web interface.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConversionOptions {
    /// Width and height override
    ///
    /// Useful when an SVG does not have a set width and height or you want to override it.
    #[cfg_attr(feature = "serde", serde(with = "length_serde"))]
    pub dimensions: [Option<Length>; 2],
    /// Horizontal alignment within the (possibly overridden) viewport or target dimensions
    /// Only applied when an explicit width or height override is provided, or when `trim` is true.
    #[cfg_attr(feature = "serde", serde(default))]
    pub h_align: HorizontalAlign,
    /// Vertical alignment within the (possibly overridden) viewport or target dimensions
    /// Only applied when an explicit width or height override is provided, or when `trim` is true.
    #[cfg_attr(feature = "serde", serde(default))]
    pub v_align: VerticalAlign,
    /// If true, scales the drawing's tight bounding box to the provided `dimensions` (paper style fit)
    /// instead of treating `dimensions` purely as viewport size. Aspect ratio preserved.
    /// - With both dimensions: uniform scale to fit inside
    /// - With one dimension: scale so that dimension matches
    /// - With neither dimension: no effect
    #[cfg_attr(feature = "serde", serde(default))]
    pub trim: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum HorizontalAlign { Left, Center, Right }

impl Default for HorizontalAlign { fn default() -> Self { Self::Left } }

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum VerticalAlign { Bottom, Center, Top }

impl Default for VerticalAlign { fn default() -> Self { Self::Top } }

/// Maps SVG [`Node`]s and their attributes into operations on a [`Terrarium`]
#[derive(Debug)]
struct ConversionVisitor<'a, T: Turtle> {
    terrarium: Terrarium<T>,
    name_stack: Vec<String>,
    /// Used to convert percentage values
    viewport_dim_stack: Vec<[f64; 2]>,
    _config: &'a ConversionConfig,
    options: ConversionOptions,
}

impl<'a, T: Turtle> ConversionVisitor<'a, T> {
    fn comment(&mut self, node: &Node) {
        let mut comment = String::new();
        self.name_stack.iter().for_each(|name| {
            comment += name;
            comment += " > ";
        });
        comment += &node_name(node,&self._config.extra_attribute_name);

        self.terrarium.turtle.comment(comment);
    }

    fn begin(&mut self) {
        // Part 1 of converting from SVG to GCode coordinates
        self.terrarium.push_transform(Transform2D::scale(1., -1.));
        self.terrarium.turtle.begin();
    }

    fn end(&mut self) {
        self.terrarium.turtle.end();
        self.terrarium.pop_transform();
    }
}

/// Top-level function for converting an SVG [`Document`] into g-code
pub fn svg2program<'a, 'input: 'a>(
    doc: &'a Document,
    config: &ConversionConfig,
    options: ConversionOptions,
    machine: Machine<'input>,
) -> Vec<Token<'input>> {
    let bounding_box_and_viewport_generator = || {
        let mut visitor = ConversionVisitor {
            terrarium: Terrarium::new(DpiConvertingTurtle {
                inner: PreprocessTurtle::default(),
                dpi: config.dpi,
            }),
            _config: config,
            options: options.clone(),
            name_stack: vec![],
            viewport_dim_stack: vec![],
        };

        visitor.begin();
        visit::depth_first_visit(doc, &mut visitor);
        visitor.end();

        (
            visitor.terrarium.turtle.inner.bounding_box,
            // Last pushed viewport dims (user units) if any
            visitor.viewport_dim_stack.last().copied().unwrap_or([1.0, 1.0]),
        )
    };

    // Convert from millimeters to user units
    let origin = config
        .origin
        .map(|dim| dim.map(|d| UomLength::new::<millimeter>(d).get::<inch>() * CSS_DEFAULT_DPI));

    // Precompute bounding box (mm) & viewport size (user units) when needed for alignment/trim/origin
    let (pre_bbox_mm, viewport_user_units) = bounding_box_and_viewport_generator();

    // Convert viewport size to mm (DPI based) for alignment math
    let viewport_mm = viewport_user_units.map(|v| {
        // user units -> inches -> mm (mirrors DpiConvertingTurtle logic for coordinates)
        UomLength::new::<inch>(v / config.dpi).get::<millimeter>()
    });

    let origin_transform = match origin {
        [None, Some(origin_y)] => {
            Transform2D::translation(0., origin_y - pre_bbox_mm.min.y)
        }
        [Some(origin_x), None] => {
            Transform2D::translation(origin_x - pre_bbox_mm.min.x, 0.)
        }
        [Some(origin_x), Some(origin_y)] => {
            Transform2D::translation(
                origin_x - pre_bbox_mm.min.x,
                origin_y - pre_bbox_mm.min.y,
            )
        }
        [None, None] => Transform2D::identity(),
    };

    // Alignment & optional trim scaling
    let mut post_transform = Transform2D::identity();

    if options.trim || options.dimensions.iter().any(|d| d.is_some()) {
        // Target sizes in mm if provided
        let target_mm: [Option<f64>; 2] = options.dimensions.map(|opt_l| opt_l.map(|l| {
            // length in user units (already converted earlier when applying overrides) -> user units numeric
            // We stored viewport_user_units already incorporating overrides; for bbox scaling we only need numeric interpret of provided dimension
            // Re-parse like units::length_to_user_units, but simpler: rely on what was parsed earlier: l.number with unit
            match l.unit { svgtypes::LengthUnit::Mm => l.number,
                svgtypes::LengthUnit::Cm => UomLength::new::<centimeter>(l.number).get::<millimeter>(),
                svgtypes::LengthUnit::In => UomLength::new::<inch>(l.number).get::<millimeter>(),
                svgtypes::LengthUnit::Px => UomLength::new::<inch>(l.number / config.dpi).get::<millimeter>(),
                svgtypes::LengthUnit::Pt => UomLength::new::<inch>(l.number / 72.0).get::<millimeter>(),
                svgtypes::LengthUnit::Pc => UomLength::new::<pica_computer>(l.number).get::<millimeter>(),
                _ => UomLength::new::<inch>(l.number / config.dpi).get::<millimeter>() }
        }));

    let mut bbox = pre_bbox_mm;
        let bbox_w = bbox.width();
        let bbox_h = bbox.height();
        let mut scale = 1.0;
    if options.trim {
            match (target_mm[0], target_mm[1]) {
                (Some(w), Some(h)) if bbox_w > 0. && bbox_h > 0. => {
                    scale = (w / bbox_w).min(h / bbox_h);
                }
                (Some(w), None) if bbox_w > 0. => scale = w / bbox_w,
                (None, Some(h)) if bbox_h > 0. => scale = h / bbox_h,
                _ => {}
            }
            post_transform = Transform2D::scale(scale, scale).then(&post_transform);
            // Scaled bbox dims for alignment leftover
            bbox = lyon_geom::Box2D::new(
                lyon_geom::point(bbox.min.x * scale, bbox.min.y * scale),
                lyon_geom::point(bbox.max.x * scale, bbox.max.y * scale),
            );
        }

        // Determine alignment within either: viewport_mm (no trim) or target_mm (trim with explicit dims)
        let container_w = if options.trim {
            target_mm[0].unwrap_or(bbox.width())
        } else {
            viewport_mm[0]
        };
        let container_h = if options.trim {
            target_mm[1].unwrap_or(bbox.height())
        } else {
            viewport_mm[1]
        };

        // Horizontal alignment
    let dx_mm = match options.h_align {
            HorizontalAlign::Left => -bbox.min.x,
            HorizontalAlign::Center => (container_w - bbox.width()) / 2. - bbox.min.x,
            HorizontalAlign::Right => (container_w - bbox.width()) - bbox.min.x,
        };
        // Vertical alignment (Top is default; coordinate system has origin at bottom-left after existing pre transforms)
    let dy_mm = match options.v_align {
            VerticalAlign::Bottom => -bbox.min.y,
            VerticalAlign::Center => (container_h - bbox.height()) / 2. - bbox.min.y,
            VerticalAlign::Top => (container_h - bbox.height()) - bbox.min.y,
        };
    // Current transform stack is in user units; our math was done in mm (pre_bbox_mm).
    let mm_per_user_unit = UomLength::new::<inch>(1.0 / config.dpi).get::<millimeter>();
    let dx = dx_mm / mm_per_user_unit;
    let dy = dy_mm / mm_per_user_unit;
    post_transform = Transform2D::translation(dx, dy).then(&post_transform);
    }

    let options_clone_for_transform = options.clone();
    let options_for_visitor = options_clone_for_transform.clone();
    let mut conversion_visitor = ConversionVisitor {
        terrarium: Terrarium::new(DpiConvertingTurtle {
            inner: GCodeTurtle {
                machine,
                tolerance: config.tolerance,
                feedrate: config.feedrate,
                program: vec![],
            },
            dpi: config.dpi,
        }),
        _config: config,
    options: options_for_visitor,
        name_stack: vec![],
        viewport_dim_stack: vec![],
    };

    // Compose transforms: apply trim/alignment first, then optional user-specified origin translation.
    let alignment_requested = options_clone_for_transform.trim || options_clone_for_transform.dimensions.iter().any(|d| d.is_some());
    let default_origin_requested = config.origin == [Some(0.0), Some(0.0)];
    let apply_origin = !default_origin_requested && alignment_requested || !alignment_requested; // keep legacy behavior when no alignment/trim, otherwise skip default normalization
    let combined_transform = if apply_origin {
        post_transform.then(&origin_transform)
    } else {
        post_transform
    };
    conversion_visitor
        .terrarium
        .push_transform(combined_transform);
    conversion_visitor.begin();
    visit::depth_first_visit(doc, &mut conversion_visitor);
    conversion_visitor.end();
    conversion_visitor.terrarium.pop_transform();

    conversion_visitor.terrarium.turtle.inner.program
}

fn node_name(node: &Node , attr_to_print :  &Option<String> ) -> String {
    let mut name = node.tag_name().name().to_string();
    if let Some(id) = node.attribute("id") {
        name += "#";
        name += id;
	if let Some(extra_attr_to_print) = attr_to_print {
	    for a_attr in node.attributes() {
		if a_attr.name() == extra_attr_to_print {
		   name += " ( ";
		   name += a_attr.value() ;
		   name += " ) ";
		}
	    }
	}
    }
    name
}

#[cfg(all(test, feature = "serde"))]
mod test {
    use super::*;
    use svgtypes::LengthUnit;

    #[test]
    fn serde_conversion_options_is_correct() {
        let default_struct = ConversionOptions::default();
        let default_json = r#"{"dimensions":[null,null]}"#;

        assert_eq!(
            serde_json::to_string(&default_struct).unwrap(),
            default_json
        );
        assert_eq!(
            serde_json::from_str::<ConversionOptions>(default_json).unwrap(),
            default_struct
        );
    }

    #[test]
    fn serde_conversion_options_with_single_dimension_is_correct() {
        let mut r#struct = ConversionOptions::default();
        r#struct.dimensions[0] = Some(Length {
            number: 4.,
            unit: LengthUnit::Mm,
        });
        let json = r#"{"dimensions":[{"number":4.0,"unit":"Mm"},null]}"#;

        assert_eq!(serde_json::to_string(&r#struct).unwrap(), json);
        assert_eq!(
            serde_json::from_str::<ConversionOptions>(json).unwrap(),
            r#struct
        );
    }

    #[test]
    fn serde_conversion_options_with_both_dimensions_is_correct() {
        let mut r#struct = ConversionOptions::default();
        r#struct.dimensions = [
            Some(Length {
                number: 4.,
                unit: LengthUnit::Mm,
            }),
            Some(Length {
                number: 10.5,
                unit: LengthUnit::In,
            }),
        ];
        let json = r#"{"dimensions":[{"number":4.0,"unit":"Mm"},{"number":10.5,"unit":"In"}]}"#;

        assert_eq!(serde_json::to_string(&r#struct).unwrap(), json);
        assert_eq!(
            serde_json::from_str::<ConversionOptions>(json).unwrap(),
            r#struct
        );
    }
}
