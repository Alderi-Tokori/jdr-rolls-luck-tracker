use iced::{Color, Element, Length, Point, Task};
use iced::widget::{canvas, Column};
use crate::gui::widgets;
use crate::gui::widgets::graph::SplineGraph;

pub struct Dashboard {
    pub graph_points: Vec<Point>,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Placeholder,
    GraphClicked(Point)
}

pub enum Action {
    Run(Task<Message>),
    None,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self {
            graph_points: vec![],
        }
    }
}

impl Dashboard {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Placeholder => {
                Action::None
            },
            Message::GraphClicked(point) => {
                self.graph_points.push(Point {
                    x: self.graph_points.last().map(|p| p.x + 1.0).unwrap_or(0.0),
                    y: point.y
                });

                dbg!(& self.graph_points);
                Action::None
            }
        }
    }

    pub fn view(& self) -> Element<'_, Message> {
        iced::widget::column![
             canvas(widgets::graph::SplineGraph {
                data: Some(& self.graph_points),
                number_of_segments: 50,
                on_click: Some(Box::new(|point| Message::GraphClicked(point))),
                ..widgets::graph::SplineGraph::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
        ].into()
    }
}
