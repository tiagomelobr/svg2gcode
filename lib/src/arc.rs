use euclid::Angle;
use lyon_geom::{
    ArcFlags, CubicBezierSegment, Line, LineSegment, Point, Scalar, SvgArc, Transform, Vector,
};

pub enum ArcOrLineSegment<S> {
    Arc(SvgArc<S>),
    Line(LineSegment<S>),
}

fn arc_from_endpoints_and_tangents<S: Scalar>(
    from: Point<S>,
    from_tangent: Vector<S>,
    to: Point<S>,
    to_tangent: Vector<S>,
) -> Option<SvgArc<S>> {
    let from_to = (from - to).length();
    let incenter = {
        let from_tangent = Line {
            point: from,
            vector: from_tangent,
        };
        let to_tangent = Line {
            point: to,
            vector: to_tangent,
        };

        let intersection = from_tangent.intersection(&to_tangent)?;
        let from_intersection = (from - intersection).length();
        let to_intersection = (to - intersection).length();

        (((from * to_intersection).to_vector()
            + (to * from_intersection).to_vector()
            + (intersection * from_to).to_vector())
            / (from_intersection + to_intersection + from_to))
            .to_point()
    };

    let get_perpendicular_bisector = |a, b| {
        let vector: Vector<S> = a - b;
        let perpendicular_vector = Vector::from([-vector.y, vector.x]).normalize();
        Line {
            point: LineSegment { from: a, to: b }.sample(S::HALF),
            vector: perpendicular_vector,
        }
    };

    let from_incenter_bisector = get_perpendicular_bisector(from, incenter);
    let to_incenter_bisector = get_perpendicular_bisector(to, incenter);
    let center = from_incenter_bisector.intersection(&to_incenter_bisector)?;

    let radius = (from - center).length();

    // Use the 2D determinant + dot product to identify winding direction
    // See https://www.w3.org/TR/SVG/paths.html#PathDataEllipticalArcCommands for
    // a nice visual explanation of large arc and sweep
    let flags = {
        let from_center = (from - center).normalize();
        let to_center = (to - center).normalize();

        let det = from_center.x * to_center.y - from_center.y * to_center.x;
        let dot = from_center.dot(to_center);
        let atan2 = det.atan2(dot);
        ArcFlags {
            large_arc: atan2.abs() >= S::PI(),
            sweep: atan2.is_sign_positive(),
        }
    };

    Some(SvgArc {
        from,
        to,
        radii: Vector::splat(radius),
        // This is a circular arc
        x_rotation: Angle::zero(),
        flags,
    })
}

pub trait FlattenWithArcs<S> {
    fn flattened(&self, tolerance: S) -> Vec<ArcOrLineSegment<S>>;
}

impl<S> FlattenWithArcs<S> for CubicBezierSegment<S>
where
    S: Scalar + Copy,
{
    /// Implementation of [Modeling of Bézier Curves Using a Combination of Linear and Circular Arc Approximations](https://sci-hub.st/https://doi.org/10.1109/CGIV.2012.20)
    ///
    /// There are some slight deviations like using monotonic ranges instead of bounding by inflection points.
    ///
    /// Kaewsaiha, P., & Dejdumrong, N. (2012). Modeling of Bézier Curves Using a Combination of Linear and Circular Arc Approximations. 2012 Ninth International Conference on Computer Graphics, Imaging and Visualization. doi:10.1109/cgiv.2012.20
    ///
    fn flattened(&self, tolerance: S) -> Vec<ArcOrLineSegment<S>> {
        if (self.to - self.from).square_length() < S::EPSILON {
            return vec![];
        } else if self.is_linear(tolerance) {
            return vec![ArcOrLineSegment::Line(self.baseline())];
        }
        let mut acc = vec![];

        self.for_each_monotonic_range(&mut |range| {
            let inner_bezier = self.split_range(range);

            if (inner_bezier.to - inner_bezier.from).square_length() < S::EPSILON {
                return;
            } else if inner_bezier.is_linear(tolerance) {
                acc.push(ArcOrLineSegment::Line(inner_bezier.baseline()));
                return;
            }

            if let Some(svg_arc) = arc_from_endpoints_and_tangents(
                inner_bezier.from,
                inner_bezier.derivative(S::ZERO),
                inner_bezier.to,
                inner_bezier.derivative(S::ONE),
            )
            .filter(|svg_arc| {
                let arc = svg_arc.to_arc();
                let mut max_deviation = S::ZERO;
                // TODO: find a better way to check tolerance
                // Ideally: derivative of |f(x) - g(x)| and look at 0 crossings
                for i in 1..20 {
                    let t = S::from(i).unwrap() / S::from(20).unwrap();
                    max_deviation =
                        max_deviation.max((arc.sample(t) - inner_bezier.sample(t)).length());
                }
                max_deviation < tolerance
            }) {
                acc.push(ArcOrLineSegment::Arc(svg_arc));
            } else {
                let (left, right) = inner_bezier.split(S::HALF);
                acc.append(&mut FlattenWithArcs::flattened(&left, tolerance));
                acc.append(&mut FlattenWithArcs::flattened(&right, tolerance));
            }
        });
        acc
    }
}

impl<S> FlattenWithArcs<S> for SvgArc<S>
where
    S: Scalar,
{
    fn flattened(&self, tolerance: S) -> Vec<ArcOrLineSegment<S>> {
        if (self.to - self.from).square_length() < S::EPSILON {
            return vec![];
        } else if self.is_straight_line() {
            return vec![ArcOrLineSegment::Line(LineSegment {
                from: self.from,
                to: self.to,
            })];
        } else if (self.radii.x.abs() - self.radii.y.abs()).abs() < S::EPSILON {
            return vec![ArcOrLineSegment::Arc(*self)];
        }

        let self_arc = self.to_arc();
        if let Some(svg_arc) = arc_from_endpoints_and_tangents(
            self.from,
            self_arc.sample_tangent(S::ZERO),
            self.to,
            self_arc.sample_tangent(S::ONE),
        )
        .filter(|approx_svg_arc| {
            let approx_arc = approx_svg_arc.to_arc();
            let mut max_deviation = S::ZERO;
            // TODO: find a better way to check tolerance
            // Ideally: derivative of |f(x) - g(x)| and look at 0 crossings
            for i in 1..20 {
                let t = S::from(i).unwrap() / S::from(20).unwrap();
                max_deviation =
                    max_deviation.max((approx_arc.sample(t) - self_arc.sample(t)).length());
            }
            max_deviation < tolerance
        }) {
            vec![ArcOrLineSegment::Arc(svg_arc)]
        } else {
            let (left, right) = self_arc.split(S::HALF);
            let mut acc = FlattenWithArcs::flattened(&left.to_svg_arc(), tolerance);
            acc.append(&mut FlattenWithArcs::flattened(
                &right.to_svg_arc(),
                tolerance,
            ));
            acc
        }
    }
}

pub trait Transformed<S> {
    fn transformed(&self, transform: &Transform<S>) -> Self;
}

impl<S: Scalar> Transformed<S> for SvgArc<S> {
    /// A lot of the math here is heavily borrowed from [Vitaly Putzin's svgpath](https://github.com/fontello/svgpath).
    ///
    /// The code is Rust-ified with only one or two changes, but I plan to understand the math here and
    /// merge changes upstream to lyon-geom.
    fn transformed(&self, transform: &Transform<S>) -> Self {
        let from = transform.transform_point(self.from);
        let to = transform.transform_point(self.to);

        // Translation does not affect rotation, radii, or flags
        let [a, b, c, d, _tx, _ty] = transform.to_array();
        let (x_rotation, radii) = {
            let (sin, cos) = self.x_rotation.sin_cos();

            // Radii are axis-aligned -- rotate & transform
            let ma = [
                self.radii.x * (a * cos + c * sin),
                self.radii.x * (b * cos + d * sin),
                self.radii.y * (-a * sin + c * cos),
                self.radii.y * (-b * sin + d * cos),
            ];

            // ma * transpose(ma) = [ J L ]
            //                      [ L K ]
            // L is calculated later (if the image is not a circle)
            let J = ma[0].powi(2) + ma[2].powi(2);
            let K = ma[1].powi(2) + ma[3].powi(2);

            // the discriminant of the characteristic polynomial of ma * transpose(ma)
            let D = ((ma[0] - ma[3]).powi(2) + (ma[2] + ma[1]).powi(2))
                * ((ma[0] + ma[3]).powi(2) + (ma[2] - ma[1]).powi(2));

            // the "mean eigenvalue"
            let JK = (J + K) / S::TWO;

            // check if the image is (almost) a circle
            if D < S::EPSILON * JK {
                // if it is
                (Angle::zero(), Vector::splat(JK.sqrt()))
            } else {
                // if it is not a circle
                let L = ma[0] * ma[1] + ma[2] * ma[3];

                let D = D.sqrt();

                // {l1,l2} = the two eigen values of ma * transpose(ma)
                let l1 = JK + D / S::TWO;
                let l2 = JK - D / S::TWO;
                // the x - axis - rotation angle is the argument of the l1 - eigenvector
                let ax = if L.abs() < S::EPSILON && (l1 - K).abs() < S::EPSILON {
                    Angle::frac_pi_2()
                } else {
                    Angle::radians(
                        (if L.abs() > (l1 - K).abs() {
                            (l1 - J) / L
                        } else {
                            L / (l1 - K)
                        })
                        .atan(),
                    )
                };
                (ax, Vector::from([l1.sqrt(), l2.sqrt()]))
            }
        };
        // A mirror transform causes this flag to be flipped
        let invert_sweep = { (a * d) - (b * c) < S::ZERO };
        let flags = ArcFlags {
            sweep: if invert_sweep {
                !self.flags.sweep
            } else {
                self.flags.sweep
            },
            large_arc: self.flags.large_arc,
        };
        Self {
            from,
            to,
            radii,
            x_rotation,
            flags,
        }
    }
}

#[cfg(test)]
mod tests {
    use cairo::{Context, SvgSurface};
    use lyon_geom::{point, vector, CubicBezierSegment, Point, Vector};
    use std::path::PathBuf;
    use svgtypes::PathParser;

    use crate::arc::{ArcOrLineSegment, FlattenWithArcs};

    #[test]
    #[ignore = "Creates an image file, will revise later"]
    fn flatten_returns_expected_arcs() {
        const PATH: &str = "M  8.0549,11.9023
        c
        0.13447,1.69916 8.85753,-5.917903 7.35159,-6.170957
        z";
        let mut surf =
            SvgSurface::new(128., 128., Some(PathBuf::from("approx_circle.svg"))).unwrap();
        surf.set_document_unit(cairo::SvgUnit::Mm);
        let ctx = Context::new(&surf).unwrap();
        ctx.set_line_width(0.2);

        let mut current_position = Point::zero();

        let mut acc = 0;

        for path in PathParser::from(PATH) {
            use svgtypes::PathSegment::*;
            match path.unwrap() {
                MoveTo { x, y, abs } => {
                    if abs {
                        ctx.move_to(x, y);
                        current_position = point(x, y);
                    } else {
                        ctx.rel_move_to(x, y);
                        current_position += vector(x, y);
                    }
                }
                LineTo { x, y, abs } => {
                    if abs {
                        ctx.line_to(x, y);
                        current_position = point(x, y);
                    } else {
                        ctx.rel_line_to(x, y);
                        current_position += vector(x, y);
                    }
                }
                ClosePath { .. } => ctx.close_path(),
                CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                    abs,
                } => {
                    ctx.set_dash(&[], 0.);
                    match acc {
                        0 => ctx.set_source_rgb(1., 0., 0.),
                        1 => ctx.set_source_rgb(0., 1., 0.),
                        2 => ctx.set_source_rgb(0., 0., 1.),
                        3 => ctx.set_source_rgb(0., 0., 0.),
                        _ => unreachable!(),
                    }
                    let curve = CubicBezierSegment {
                        from: current_position,
                        ctrl1: (vector(x1, y1)
                            + if !abs {
                                current_position.to_vector()
                            } else {
                                Vector::zero()
                            })
                        .to_point(),
                        ctrl2: (vector(x2, y2)
                            + if !abs {
                                current_position.to_vector()
                            } else {
                                Vector::zero()
                            })
                        .to_point(),
                        to: (vector(x, y)
                            + if !abs {
                                current_position.to_vector()
                            } else {
                                Vector::zero()
                            })
                        .to_point(),
                    };
                    for segment in FlattenWithArcs::flattened(&curve, 0.02) {
                        match segment {
                            ArcOrLineSegment::Arc(svg_arc) => {
                                let arc = svg_arc.to_arc();
                                if svg_arc.flags.sweep {
                                    ctx.arc(
                                        arc.center.x,
                                        arc.center.y,
                                        arc.radii.x,
                                        arc.start_angle.radians,
                                        (arc.start_angle + arc.sweep_angle).radians,
                                    )
                                } else {
                                    ctx.arc_negative(
                                        arc.center.x,
                                        arc.center.y,
                                        arc.radii.x,
                                        arc.start_angle.radians,
                                        (arc.start_angle + arc.sweep_angle).radians,
                                    )
                                }
                            }
                            ArcOrLineSegment::Line(line) => ctx.line_to(line.to.x, line.to.y),
                        }
                    }

                    ctx.stroke().unwrap();

                    current_position = curve.to;
                    ctx.set_dash(&[0.1], 0.);
                    ctx.move_to(curve.from.x, curve.from.y);
                    ctx.curve_to(
                        curve.ctrl1.x,
                        curve.ctrl1.y,
                        curve.ctrl2.x,
                        curve.ctrl2.y,
                        curve.to.x,
                        curve.to.y,
                    );
                    ctx.stroke().unwrap();
                    acc += 1;
                }
                other => unimplemented!("{:?}", other),
            }
        }
    }
}

/// Detects circular arcs in sequences of line segments (for polygon/polyline arc detection)
pub fn detect_polygon_arcs<S>(
    points: &[Point<S>],
    tolerance: S,
    min_points: usize,
) -> Vec<ArcOrLineSegment<S>>
where
    S: Scalar + Copy,
{
    if points.len() < 2 {
        return vec![];
    }

    let mut result = Vec::new();
    let mut i = 0;

    while i < points.len() - 1 {
        // Try to detect an arc starting from point i
        if let Some((arc_length, svg_arc)) = detect_arc_starting_at(points, i, tolerance, min_points) {
            result.push(ArcOrLineSegment::Arc(svg_arc));
            i += arc_length;
        } else {
            // No arc found, emit a line segment
            result.push(ArcOrLineSegment::Line(LineSegment {
                from: points[i],
                to: points[i + 1],
            }));
            i += 1;
        }
    }

    result
}

/// Attempts to detect a circular arc starting from the given index
fn detect_arc_starting_at<S>(
    points: &[Point<S>],
    start_idx: usize,
    tolerance: S,
    min_points: usize,
) -> Option<(usize, SvgArc<S>)>
where
    S: Scalar + Copy,
{
    if start_idx + min_points > points.len() {
        return None;
    }

    // Try increasingly longer sequences starting from min_points
    for end_idx in (start_idx + min_points)..=points.len() {
        let segment = &points[start_idx..end_idx];
        
        if let Some(circle) = fit_circle_to_points(segment, tolerance) {
            // Create an SvgArc from the first to last point
            if let Some(svg_arc) = create_svg_arc_from_circle(
                segment[0],
                segment[segment.len() - 1],
                circle,
            ) {
                return Some((end_idx - start_idx - 1, svg_arc));
            }
        } else {
            // If we can't fit a circle to this sequence, 
            // try the previous shorter sequence if it was valid
            if end_idx > start_idx + min_points {
                let prev_segment = &points[start_idx..(end_idx - 1)];
                if let Some(circle) = fit_circle_to_points(prev_segment, tolerance) {
                    if let Some(svg_arc) = create_svg_arc_from_circle(
                        prev_segment[0],
                        prev_segment[prev_segment.len() - 1],
                        circle,
                    ) {
                        return Some((end_idx - start_idx - 2, svg_arc));
                    }
                }
            }
            break;
        }
    }

    None
}

/// Simple circle representation
#[derive(Debug, Clone, Copy)]
struct Circle<S> {
    center: Point<S>,
    radius: S,
}

/// Fits a circle to a sequence of points using least squares approach
fn fit_circle_to_points<S>(points: &[Point<S>], tolerance: S) -> Option<Circle<S>>
where
    S: Scalar + Copy,
{
    if points.len() < 3 {
        return None;
    }

    // Use the first three points to get an initial circle
    let initial_circle = circle_from_three_points(points[0], points[1], points[2])?;
    
    // Reject very small circles - they're likely noise or nearly straight lines
    let min_radius = tolerance * S::from(10.0).unwrap();
    if initial_circle.radius < min_radius {
        return None;
    }

    // Check if all points lie on this circle within tolerance
    let mut max_deviation = S::ZERO;
    for &point in points.iter() {
        let deviation = ((point - initial_circle.center).length() - initial_circle.radius).abs();
        if deviation > max_deviation {
            max_deviation = deviation;
        }
    }

    if max_deviation <= tolerance {
        Some(initial_circle)
    } else {
        None
    }
}

/// Calculates circle from three points using circumcenter formula
fn circle_from_three_points<S>(p1: Point<S>, p2: Point<S>, p3: Point<S>) -> Option<Circle<S>>
where
    S: Scalar + Copy,
{
    let a = p2 - p1;
    let b = p3 - p1;

    // Check if points are collinear (cross product is zero)
    let cross = a.x * b.y - a.y * b.x;
    if cross.abs() < S::EPSILON {
        return None;
    }

    // Calculate circumcenter using perpendicular bisectors
    let a_len_sq = a.square_length();
    let b_len_sq = b.square_length();
    let two_cross = cross * S::from(2.0).unwrap();

    let center = Point::new(
        p1.x + (b.y * a_len_sq - a.y * b_len_sq) / two_cross,
        p1.y + (a.x * b_len_sq - b.x * a_len_sq) / two_cross,
    );

    let radius = (center - p1).length();

    Some(Circle { center, radius })
}

/// Creates an SvgArc from endpoints and circle parameters
fn create_svg_arc_from_circle<S>(
    from: Point<S>,
    to: Point<S>,
    circle: Circle<S>,
) -> Option<SvgArc<S>>
where
    S: Scalar + Copy,
{
    // Check for degenerate cases
    let chord_length = (to - from).length();
    if chord_length < S::EPSILON || circle.radius < S::EPSILON {
        return None;
    }
    
    // Check if points are too close to the center (would create invalid arc)
    let from_to_center = (from - circle.center).length();
    let to_to_center = (to - circle.center).length();
    if (from_to_center - circle.radius).abs() > circle.radius * S::from(0.1).unwrap() ||
       (to_to_center - circle.radius).abs() > circle.radius * S::from(0.1).unwrap() {
        return None;
    }

    // Calculate vectors from center to endpoints
    let from_vec = (from - circle.center).normalize();
    let to_vec = (to - circle.center).normalize();

    // Calculate the sweep angle using cross product and dot product
    let cross = from_vec.x * to_vec.y - from_vec.y * to_vec.x;
    let dot = from_vec.dot(to_vec);
    let angle = cross.atan2(dot);

    // Reject very small angles (nearly straight lines)
    if angle.abs() < S::from(0.01).unwrap() {
        return None;
    }
    
    // Reject if the chord is nearly equal to diameter (semicircle or larger)
    // This can be numerically unstable
    if chord_length > circle.radius * S::from(1.9).unwrap() {
        return None;
    }

    let flags = ArcFlags {
        large_arc: angle.abs() >= S::PI(),
        sweep: angle.is_sign_positive(),
    };

    let svg_arc = SvgArc {
        from,
        to,
        radii: Vector::splat(circle.radius),
        x_rotation: Angle::zero(),
        flags,
    };
    
    // Final check: verify this isn't considered a straight line by Lyon
    if svg_arc.is_straight_line() {
        return None;
    }

    Some(svg_arc)
}
