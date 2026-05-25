use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::{Cache, Frame, Path, Stroke, Fill};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, Vector};
use crate::splines;
use crate::splines::PolynomialFunction;

#[derive(Debug, Clone)]
pub struct SplineGraph {
    pub data: Vec<Point>,
    pub line_color: Color,
    pub line_width: f32,
    pub number_of_segments: i32,
}

impl Default for SplineGraph {
    fn default() -> Self {
        Self {
            data: vec![],
            line_color: Color::from_rgb(0.2, 0.5, 0.9),
            line_width: 2.0,
            number_of_segments: 1,
        }
    }
}

impl<Message> canvas::Program<Message> for SplineGraph {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if self.data.len() < 2 {
            return vec![frame.into_geometry()];
        }

        if (self.number_of_segments < 1) {
            return vec![frame.into_geometry()];
        }

        let data: Vec<splines::Point> = self.data.iter().map(|p| p.into()).collect();
        let graph_spline_polynomial = match splines::get_graph_spline_interpolation_function(data) {
            Some(p) => p,
            None => return vec![frame.into_geometry()]
        };

        let min_x = self.data.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = self.data.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = self.data.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = self.data.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

        let x_range = max_x - min_x;
        let y_range = max_y - min_y;

        let graph_w = bounds.width;
        let graph_h = bounds.height;

        fn map_x(graph_w: f32, min_x: f32, x_range: f32, x: f32) -> f32 {
            (x - min_x) / x_range * graph_w
        }
        fn map_y(graph_h: f32, min_y: f32, y_range: f32, y: f32) -> f32 {
            (1.0 - (y - min_y) / y_range) * graph_h
        }

        // Line
        let line_path = {
            let mut builder = canvas::path::Builder::new();
            let first = self.data[0];
            builder.move_to(Point::new(
                map_x(graph_w, min_x, x_range, first.x),
                map_y(graph_h, min_y, y_range, first.y),
            ));

            for interval in & graph_spline_polynomial.intervals {
                let segment_step = (interval.end.x - interval.start.x) / (self.number_of_segments as f64);

                for segment_idx in 1..=self.number_of_segments {
                    let x = interval.start.x + (segment_idx as f64) * segment_step;

                    builder.line_to(Point::new(
                        map_x(graph_w, min_x, x_range, x as f32),
                        map_y(graph_h, min_y, y_range, graph_spline_polynomial.eval(x).unwrap_or(0.0) as f32)
                    ))
                }
            }

            builder.build()
        };
        let line_stroke = Stroke::default()
            .with_color(self.line_color)
            .with_width(self.line_width);
        frame.stroke(&line_path, line_stroke);

        vec![frame.into_geometry()]
    }
}