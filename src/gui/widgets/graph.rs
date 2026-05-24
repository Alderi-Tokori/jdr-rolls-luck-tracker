use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::{Cache, Frame, Path, Stroke, Fill};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, Vector};

#[derive(Debug, Clone)]
pub struct LineGraph {
    pub data: Vec<Point>,
    pub line_color: Color,
    pub line_width: f32,
    pub show_points: bool,
}

impl Default for LineGraph {
    fn default() -> Self {
        Self {
            data: vec![],
            line_color: Color::from_rgb(0.2, 0.5, 0.9),
            line_width: 2.0,
            show_points: true,
        }
    }
}

impl<Message> canvas::Program<Message> for LineGraph {
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
        let margin = 40.0;
        let graph_w = bounds.width - 2.0 * margin;
        let graph_h = bounds.height - 2.0 * margin;
        let min_x = self.data.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = self.data.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = self.data.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = self.data.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        let x_range = max_x - min_x;
        let y_range = max_y - min_y;
        fn map_x(margin: f32, graph_w: f32, min_x: f32, x_range: f32, x: f32) -> f32 {
            margin + (x - min_x) / x_range * graph_w
        }
        fn map_y(margin: f32, graph_h: f32, min_y: f32, y_range: f32, y: f32) -> f32 {
            margin + (1.0 - (y - min_y) / y_range) * graph_h
        }
        // Axes
        let axis_style = Stroke::default().with_color(Color::from_rgb(0.5, 0.5, 0.5)).with_width(1.0);
        let left = Point::new(margin, margin);
        let right = Point::new(margin + graph_w, margin);
        let top = Point::new(margin, margin + graph_h);
        frame.stroke(&Path::line(left, right), axis_style);
        frame.stroke(&Path::line(left, top), axis_style);
        // Line
        let line_path = {
            let mut builder = canvas::path::Builder::new();
            let first = self.data[0];
            builder.move_to(Point::new(
                map_x(margin, graph_w, min_x, x_range, first.x),
                map_y(margin, graph_h, min_y, y_range, first.y),
            ));
            for point in &self.data[1..] {
                builder.line_to(Point::new(
                    map_x(margin, graph_w, min_x, x_range, point.x),
                    map_y(margin, graph_h, min_y, y_range, point.y),
                ));
            }
            builder.build()
        };
        let line_stroke = Stroke::default()
            .with_color(self.line_color)
            .with_width(self.line_width);
        frame.stroke(&line_path, line_stroke);
        // Points
        if self.show_points {
            let radius = 3.0;
            let fill = Fill::from(self.line_color);
            for point in &self.data {
                let center = Point::new(
                    map_x(margin, graph_w, min_x, x_range, point.x),
                    map_y(margin, graph_h, min_y, y_range, point.y),
                );
                frame.fill(&Path::circle(center, radius), fill);
            }
        }
        vec![frame.into_geometry()]
    }
}