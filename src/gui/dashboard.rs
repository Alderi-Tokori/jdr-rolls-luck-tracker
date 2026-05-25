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
            }
        }
    }

    pub fn view(& self) -> Element<'_, Message> {
        iced::widget::column![
             canvas(widgets::graph::SplineGraph {
                data: Some(& self.graph_points),
                number_of_segments: 50,
                ..widgets::graph::SplineGraph::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
        ].into()
    }
}
