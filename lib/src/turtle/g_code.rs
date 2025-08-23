use std::borrow::Cow;
use std::fmt::Debug;

use ::g_code::{command, emit::Token};
use lyon_geom::{CubicBezierSegment, Point, QuadraticBezierSegment, SvgArc};

use super::Turtle;
use crate::arc::{detect_polygon_arcs, ArcOrLineSegment, FlattenWithArcs};
use crate::machine::Machine;

/// Maps path segments into g-code operations
#[derive(Debug)]
pub struct GCodeTurtle<'input> {
    pub machine: Machine<'input>,
    pub tolerance: f64,
    pub feedrate: f64,
    pub min_arc_radius: f64,
    pub program: Vec<Token<'input>>,
    // When true, emit the user between-layers sequence right before the next tool_on
    pub pending_between_layers: bool,
    // Polygon arc detection configuration
    pub polygon_arc_config: PolygonArcConfig,
    // Buffer for line segments to enable polygon arc detection
    line_buffer: Vec<Point<f64>>,
}

/// Configuration for polygon arc detection
#[derive(Debug, Clone)]
pub struct PolygonArcConfig {
    pub enabled: bool,
    pub min_points: usize,
    pub tolerance: f64,
}

impl<'input> GCodeTurtle<'input> {
    /// Create a new GCodeTurtle with polygon arc detection configuration
    pub fn new(
        machine: Machine<'input>,
        tolerance: f64,
        feedrate: f64,
        min_arc_radius: f64,
        polygon_arc_config: PolygonArcConfig,
    ) -> Self {
        Self {
            machine,
            tolerance,
            feedrate,
            min_arc_radius,
            program: Vec::new(),
            pending_between_layers: false,
            polygon_arc_config,
            line_buffer: Vec::new(),
        }
    }

    /// Flush the line buffer, analyzing for arcs and generating appropriate G-code
    fn flush_line_buffer(&mut self) {
        if self.line_buffer.is_empty() {
            return;
        }

        if self.polygon_arc_config.enabled
            && self.line_buffer.len() >= self.polygon_arc_config.min_points
            && self
                .machine
                .supported_functionality()
                .circular_interpolation
        {
            // Analyze buffer for arcs
            let segments = detect_polygon_arcs(
                &self.line_buffer,
                self.polygon_arc_config.tolerance,
                self.polygon_arc_config.min_points,
            );

            for segment in segments {
                match segment {
                    ArcOrLineSegment::Arc(arc) => {
                        // Double-check that this arc is valid and meets our requirements
                        if !arc.is_straight_line() && 
                           arc.radii.x >= self.min_arc_radius &&
                           arc.radii.y >= self.min_arc_radius {
                            self.program.append(&mut self.circular_interpolation(arc));
                        } else {
                            // Arc is invalid or too small, emit as line
                            self.program.append(
                                &mut command!(LinearInterpolation {
                                    X: arc.to.x,
                                    Y: arc.to.y,
                                    F: self.feedrate,
                                })
                                .into_token_vec(),
                            );
                        }
                    }
                    ArcOrLineSegment::Line(line) => {
                        self.program.append(
                            &mut command!(LinearInterpolation {
                                X: line.to.x,
                                Y: line.to.y,
                                F: self.feedrate,
                            })
                            .into_token_vec(),
                        );
                    }
                }
            }
        } else {
            // No arc detection or insufficient points - emit all as lines
            for point in self.line_buffer.iter().skip(1) {
                self.program.append(
                    &mut command!(LinearInterpolation {
                        X: point.x,
                        Y: point.y,
                        F: self.feedrate,
                    })
                    .into_token_vec(),
                );
            }
        }

        self.line_buffer.clear();
    }
    fn circular_interpolation(&self, svg_arc: SvgArc<f64>) -> Vec<Token<'input>> {
        debug_assert!((svg_arc.radii.x.abs() - svg_arc.radii.y.abs()).abs() < f64::EPSILON);
        // Geometry helpers
        let from = svg_arc.from;
        let to = svg_arc.to;
        let chord = (to - from).length();
        let radius = svg_arc.radii.x.abs();
        let arc_struct = svg_arc.to_arc();
        let sweep_angle = arc_struct.sweep_angle.radians.abs();

        // 1. Fallback to a linear move when arc is too small to be meaningful or numerically stable.
        //    (radius extremely small OR chord almost zero OR sweep negligible)
        if radius < self.min_arc_radius
            || chord < self.min_arc_radius
            || sweep_angle < 1e-6
        {
            return command!(LinearInterpolation { X: to.x, Y: to.y, F: self.feedrate })
                .into_token_vec();
        }

        // 2. Auto-split if (a) SVG flagged large arc OR (b) arc is (near) a semicircle which is
        //    ill-conditioned for R-mode validation (even though we now emit I/J, splitting keeps centers cleaner).
        //    Near-semicircle detection: chord ~ 2R OR sweep ~ PI within a tolerance.
        let near_semi = (chord - 2.0 * radius).abs() / (2.0 * radius) < 1e-5
            || (sweep_angle - std::f64::consts::PI).abs() < 1e-5;
        if svg_arc.flags.large_arc || near_semi {
            let (left, right) = arc_struct.split(0.5);
            let mut token_vec = self.circular_interpolation(left.to_svg_arc());
            token_vec.append(&mut self.circular_interpolation(right.to_svg_arc()));
            return token_vec;
        }

        // 3. Emit using I/J center offsets (avoids R ambiguity/validation issues in controllers for tight arcs).
        let center = arc_struct.center;
        let i = center.x - from.x;
        let j = center.y - from.y;

        match svg_arc.flags.sweep {
            true => command!(CounterclockwiseCircularInterpolation {
                X: to.x,
                Y: to.y,
                I: i,
                J: j,
                F: self.feedrate,
            })
            .into_token_vec(),
            false => command!(ClockwiseCircularInterpolation {
                X: to.x,
                Y: to.y,
                I: i,
                J: j,
                F: self.feedrate,
            })
            .into_token_vec(),
        }
    }

    fn tool_on(&mut self) {
        // Inject deferred between-layers sequence (after travel, before tool activation)
        if self.pending_between_layers {
            // Add a blank line for readability before between-layers sequence
            self.program.push(Token::Comment { is_inline: false, inner: std::borrow::Cow::Borrowed("") });
            self.program.extend(self.machine.between_layers());
            // Do NOT emit absolute here; the tool_on sequence below will restore absolute
            self.pending_between_layers = false;
        }
        self.program.extend(self.machine.tool_on());
        self.program.extend(self.machine.absolute());
    }

    fn tool_off(&mut self) {
        self.program.extend(self.machine.tool_off());
        self.program.extend(self.machine.absolute());
    }
}

impl<'input> Turtle for GCodeTurtle<'input> {
    fn begin(&mut self) {
        self.program
            .append(&mut command!(UnitsMillimeters {}).into_token_vec());
        self.program.extend(self.machine.absolute());
        self.program.extend(self.machine.program_begin());
        self.program.extend(self.machine.absolute());
    }

    fn end(&mut self) {
        // Flush any remaining line buffer
        self.flush_line_buffer();
        self.program.extend(self.machine.tool_off());
        self.program.extend(self.machine.absolute());
        self.program.extend(self.machine.program_end());
    }

    fn comment(&mut self, comment: String) {
        self.program.push(Token::Comment {
            is_inline: false,
            inner: Cow::Owned(comment),
        });
    }

    fn between_layers(&mut self) {
    // Mark for deferred emission. Actual G-Code emitted right before next tool_on() call.
    self.pending_between_layers = true;
    }

    fn move_to(&mut self, to: Point<f64>) {
        // Flush any pending line buffer before moving
        self.flush_line_buffer();
        self.tool_off();
        self.program
            .append(&mut command!(RapidPositioning { X: to.x, Y: to.y }).into_token_vec());
        
        // Start new buffer with the move destination
        self.line_buffer.clear();
        self.line_buffer.push(to);
    }

    fn line_to(&mut self, to: Point<f64>) {
        self.tool_on();
        
        if self.polygon_arc_config.enabled {
            // If buffer is empty, we need to track the starting position
            if self.line_buffer.is_empty() {
                // This should be the current position, but we need to get it somehow
                // For now, we'll use the 'to' point as both start and end if buffer is empty
                self.line_buffer.push(to);
            }
            
            // Add point to buffer for potential arc detection
            self.line_buffer.push(to);
            
            // Flush buffer if it gets too large to prevent memory issues
            const MAX_BUFFER_SIZE: usize = 1000;
            if self.line_buffer.len() > MAX_BUFFER_SIZE {
                self.flush_line_buffer();
                // Keep the last point as start of new buffer
                self.line_buffer.push(to);
            }
        } else {
            // Direct line generation (original behavior)
            self.program.append(
                &mut command!(LinearInterpolation {
                    X: to.x,
                    Y: to.y,
                    F: self.feedrate,
                })
                .into_token_vec(),
            );
        }
    }

    fn arc(&mut self, svg_arc: SvgArc<f64>) {
        // Flush line buffer before processing arc
        self.flush_line_buffer();
        
        if svg_arc.is_straight_line() {
            self.line_to(svg_arc.to);
            return;
        }

        self.tool_on();

        if self
            .machine
            .supported_functionality()
            .circular_interpolation
        {
            FlattenWithArcs::flattened(&svg_arc, self.tolerance)
                .into_iter()
                .for_each(|segment| match segment {
                    ArcOrLineSegment::Arc(arc) => {
                        self.program.append(&mut self.circular_interpolation(arc))
                    }
                    ArcOrLineSegment::Line(line) => {
                        self.line_to(line.to);
                    }
                });
        } else {
            svg_arc
                .to_arc()
                .flattened(self.tolerance)
                .for_each(|point| self.line_to(point));
        };
    }

    fn cubic_bezier(&mut self, cbs: CubicBezierSegment<f64>) {
        // Flush line buffer before processing bezier
        self.flush_line_buffer();
        
        self.tool_on();

        if self
            .machine
            .supported_functionality()
            .circular_interpolation
        {
            FlattenWithArcs::<f64>::flattened(&cbs, self.tolerance)
                .into_iter()
                .for_each(|segment| match segment {
                    ArcOrLineSegment::Arc(arc) => {
                        self.program.append(&mut self.circular_interpolation(arc))
                    }
                    ArcOrLineSegment::Line(line) => self.line_to(line.to),
                });
        } else {
            cbs.flattened(self.tolerance)
                .for_each(|point| self.line_to(point));
        };
    }

    fn quadratic_bezier(&mut self, qbs: QuadraticBezierSegment<f64>) {
        self.cubic_bezier(qbs.to_cubic());
    }
}
