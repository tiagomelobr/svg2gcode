/// Approximate [Bézier curves](https://en.wikipedia.org/wiki/B%C3%A9zier_curve) with [Circular arcs](https://en.wikipedia.org/wiki/Circular_arc)
mod arc;
/// Converts an SVG to an internal representation
mod converter;
/// Emulates the state of an arbitrary machine that can run G-Code
mod machine;
/// Operations that are easier to implement while/after G-Code is generated, or would
/// otherwise over-complicate SVG conversion
mod postprocess;
/// Provides an interface for drawing lines in G-Code
/// This concept is referred to as [Turtle graphics](https://en.wikipedia.org/wiki/Turtle_graphics).
mod turtle;

pub use converter::{svg2program, ConversionConfig, ConversionOptions};
pub use machine::{Machine, MachineConfig, SupportedFunctionality};
pub use postprocess::PostprocessConfig;
pub use turtle::Turtle;

/// A cross-platform type used to store all configuration types.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Settings {
    pub conversion: ConversionConfig,
    pub machine: MachineConfig,
    pub postprocess: PostprocessConfig,
    #[cfg_attr(feature = "serde", serde(default = "Version::unknown"))]
    pub version: Version,
}

impl Settings {
    /// Try to automatically upgrade the supported version.
    ///
    /// This will return an error if:
    ///
    /// - Settings version is [`Version::Unknown`].
    /// - There are breaking changes requiring manual intervention. In which case this does a partial update to that point.
    pub fn try_upgrade(&mut self) -> Result<(), &'static str> {
        loop {
            match self.version {
                // Compatibility for M2 by default
                Version::V0 => {
                    self.machine.end_sequence = Some(format!(
                        "{} M2",
                        self.machine.end_sequence.take().unwrap_or_default()
                    ));
                    self.version = Version::V5;
                }
                Version::V5 => break Ok(()),
                Version::Unknown(_) => break Err("cannot upgrade unknown version"),
            }
        }
    }
}

/// Used to control breaking change behavior for [`Settings`].
///
/// There were already 3 non-breaking version bumps (V1 -> V4) so versioning starts off with [`Version::V5`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    /// Implicitly versioned settings from before this type was introduced.
    V0,
    /// M2 is no longer appended to the program by default
    V5,
    #[cfg_attr(feature = "serde", serde(untagged))]
    Unknown(String),
}

impl Version {
    /// Returns the most recent [`Version`]. This is useful for asking users to upgrade externally-stored settings.
    pub const fn latest() -> Self {
        Self::V5
    }

    /// Default version for old settings.
    pub const fn unknown() -> Self {
        Self::V0
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::V0 => f.write_str("V0"),
            Version::V5 => f.write_str("V5"),
            Version::Unknown(unknown) => f.write_str(unknown),
        }
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::latest()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use g_code::emit::{FormatOptions, Token};
    use pretty_assertions::assert_eq;
    use roxmltree::ParsingOptions;
    use svgtypes::{Length, LengthUnit};

    /// The values change between debug and release builds for circular interpolation,
    /// so only check within a rough tolerance
    const TOLERANCE: f64 = 1E-10;

    fn get_actual(
        input: &str,
        circular_interpolation: bool,
        dimensions: [Option<Length>; 2],
    ) -> Vec<Token<'_>> {
        let config = ConversionConfig::default();
        let options = ConversionOptions { dimensions };
        let document = roxmltree::Document::parse_with_options(
            input,
            ParsingOptions {
                allow_dtd: true,
                ..Default::default()
            },
        )
        .unwrap();

        let machine = Machine::new(
            SupportedFunctionality {
                circular_interpolation,
            },
            None,
            None,
            None,
            None,
            None,
        );
        converter::svg2program(&document, &config, options, machine)
    }

    fn assert_close(left: Vec<Token<'_>>, right: Vec<Token<'_>>) {
        let mut code = String::new();
        g_code::emit::format_gcode_fmt(left.iter(), FormatOptions::default(), &mut code).unwrap();
        assert_eq!(left.len(), right.len(), "{code}");
        for (i, pair) in left.into_iter().zip(right.into_iter()).enumerate() {
            match pair {
                (Token::Field(l), Token::Field(r)) => {
                    assert_eq!(l.letters, r.letters);
                    if let (Some(l_value), Some(r_value)) = (l.value.as_f64(), r.value.as_f64()) {
                        assert!(
                            (l_value - r_value).abs() < TOLERANCE,
                            "Values differ significantly at {i}: {l} vs {r} ({})",
                            (l_value - r_value).abs()
                        );
                    } else {
                        assert_eq!(l, r);
                    }
                }
                (l, r) => {
                    assert_eq!(l, r, "Differs at {i}");
                }
            }
        }
    }

    #[test]
    fn square_produces_expected_gcode() {
        let expected = g_code::parse::file_parser(include_str!("../tests/square.gcode"))
            .unwrap()
            .iter_emit_tokens()
            .collect::<Vec<_>>();
        let actual = get_actual(include_str!("../tests/square.svg"), false, [None; 2]);

        assert_close(actual, expected);
    }

    #[test]
    fn square_dimension_override_produces_expected_gcode() {
        let side_length = Length {
            number: 10.,
            unit: LengthUnit::Mm,
        };

        let expected = g_code::parse::file_parser(include_str!("../tests/square.gcode"))
            .unwrap()
            .iter_emit_tokens()
            .collect::<Vec<_>>();

        for square in [
            include_str!("../tests/square.svg"),
            include_str!("../tests/square_dimensionless.svg"),
        ] {
            assert_close(
                get_actual(square, false, [Some(side_length); 2]),
                expected.clone(),
            );
            assert_close(
                get_actual(square, false, [Some(side_length), None]),
                expected.clone(),
            );
            assert_close(
                get_actual(square, false, [None, Some(side_length)]),
                expected.clone(),
            );
        }
    }

    #[test]
    fn square_transformed_produces_expected_gcode() {
        let square_transformed = include_str!("../tests/square_transformed.svg");
        let expected =
            g_code::parse::file_parser(include_str!("../tests/square_transformed.gcode"))
                .unwrap()
                .iter_emit_tokens()
                .collect::<Vec<_>>();
        let actual = get_actual(square_transformed, false, [None; 2]);

        assert_close(actual, expected)
    }

    #[test]
    fn square_transformed_nested_produces_expected_gcode() {
        let square_transformed = include_str!("../tests/square_transformed_nested.svg");
        let expected =
            g_code::parse::file_parser(include_str!("../tests/square_transformed_nested.gcode"))
                .unwrap()
                .iter_emit_tokens()
                .collect::<Vec<_>>();
        let actual = get_actual(square_transformed, false, [None; 2]);

        assert_close(actual, expected)
    }

    #[test]
    fn square_viewport_produces_expected_gcode() {
        let square_viewport = include_str!("../tests/square_viewport.svg");
        let expected = g_code::parse::file_parser(include_str!("../tests/square_viewport.gcode"))
            .unwrap()
            .iter_emit_tokens()
            .collect::<Vec<_>>();
        let actual = get_actual(square_viewport, false, [None; 2]);

        assert_close(actual, expected);
    }

    #[test]
    fn circular_interpolation_produces_expected_gcode() {
        let circular_interpolation = include_str!("../tests/circular_interpolation.svg");
        let expected =
            g_code::parse::file_parser(include_str!("../tests/circular_interpolation.gcode"))
                .unwrap()
                .iter_emit_tokens()
                .collect::<Vec<_>>();
        let actual = get_actual(circular_interpolation, true, [None; 2]);

        assert_close(actual, expected)
    }

    #[test]
    fn svg_with_smooth_curves_produces_expected_gcode() {
        let svg = include_str!("../tests/smooth_curves.svg");

        let expected = g_code::parse::file_parser(include_str!("../tests/smooth_curves.gcode"))
            .unwrap()
            .iter_emit_tokens()
            .collect::<Vec<_>>();

        let file = if cfg!(debug) {
            include_str!("../tests/smooth_curves_circular_interpolation.gcode")
        } else {
            include_str!("../tests/smooth_curves_circular_interpolation_release.gcode")
        };
        let expected_circular_interpolation = g_code::parse::file_parser(file)
            .unwrap()
            .iter_emit_tokens()
            .collect::<Vec<_>>();
        assert_close(get_actual(svg, false, [None; 2]), expected);

        assert_close(
            get_actual(svg, true, [None; 2]),
            expected_circular_interpolation,
        );
    }

    #[test]
    fn shapes_produces_expected_gcode() {
        let shapes = include_str!("../tests/shapes.svg");
        let expected = g_code::parse::file_parser(include_str!("../tests/shapes.gcode"))
            .unwrap()
            .iter_emit_tokens()
            .collect::<Vec<_>>();
        let actual = get_actual(shapes, false, [None; 2]);

        assert_close(actual, expected)
    }

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_v1_config_succeeds() {
        let json = r#"
        {
            "conversion": {
              "tolerance": 0.002,
              "feedrate": 300.0,
              "dpi": 96.0
            },
            "machine": {
              "supported_functionality": {
                "circular_interpolation": true
              },
              "tool_on_sequence": null,
              "tool_off_sequence": null,
              "begin_sequence": null,
              "between_layers_sequence": null,
              "end_sequence": null
            },
            "postprocess": {
              "origin": [
                0.0,
                0.0
              ]
            }
          }
        "#;
        serde_json::from_str::<Settings>(json).unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_v2_config_succeeds() {
        let json = r#"
        {
            "conversion": {
              "tolerance": 0.002,
              "feedrate": 300.0,
              "dpi": 96.0
            },
            "machine": {
              "supported_functionality": {
                "circular_interpolation": true
              },
              "tool_on_sequence": null,
              "tool_off_sequence": null,
              "begin_sequence": null,
              "between_layers_sequence": null,
              "end_sequence": null
            },
            "postprocess": { }
          }
        "#;
        serde_json::from_str::<Settings>(json).unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_v3_config_succeeds() {
        let json = r#"
        {
            "conversion": {
              "tolerance": 0.002,
              "feedrate": 300.0,
              "dpi": 96.0
            },
            "machine": {
              "supported_functionality": {
                "circular_interpolation": true
              },
              "tool_on_sequence": null,
              "tool_off_sequence": null,
              "begin_sequence": null,
              "between_layers_sequence": null,
              "end_sequence": null
            },
            "postprocess": {
                "checksums": false,
                "line_numbers": false
            }
          }
        "#;
        serde_json::from_str::<Settings>(json).unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_v4_config_succeeds() {
        let json = r#"
        {
            "conversion": {
              "tolerance": 0.002,
              "feedrate": 300.0,
              "dpi": 96.0
            },
            "machine": {
              "supported_functionality": {
                "circular_interpolation": true
              },
              "tool_on_sequence": null,
              "tool_off_sequence": null,
              "begin_sequence": null,
              "between_layers_sequence": null,
              "end_sequence": null
            },
            "postprocess": {
                "checksums": false,
                "line_numbers": false,
                "newline_before_comment": false
            }
          }
        "#;
        serde_json::from_str::<Settings>(json).unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_v5_config_succeeds() {
        let json = r#"
        {
            "conversion": {
              "tolerance": 0.002,
              "feedrate": 300.0,
              "dpi": 96.0
            },
            "machine": {
              "supported_functionality": {
                "circular_interpolation": true
              },
              "tool_on_sequence": null,
              "tool_off_sequence": null,
              "begin_sequence": null,
              "between_layers_sequence": null,
              "end_sequence": null
            },
            "postprocess": {
                "checksums": false,
                "line_numbers": false,
                "newline_before_comment": false
            },
            "version": "V5"
          }
        "#;
        serde_json::from_str::<Settings>(json).unwrap();
    }
}
