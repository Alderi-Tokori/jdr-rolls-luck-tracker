mod widgets;

use iced::{Length, Point};
use iced::widget::{button, column, text, Column, Canvas, canvas};

#[derive(Default)]
pub struct Counter {
    value: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Increment,
    Decrement,
}

impl Counter {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Increment => {
                self.value += 1;
            }
            Message::Decrement => {
                self.value -= 1;
            }
        }
    }

    pub fn view(&self) -> Column<'_, Message> {
        column![
             canvas(widgets::graph::SplineGraph {
                data: vec![
                    Point::new(0.0, 2.0),
                    Point::new(1.0, 3.0),
                    Point::new(2.0, 4.0),
                    Point::new(3.0, 1.0),
                    Point::new(4.0, 3.0),
                    Point::new(5.0, 5.0),
                    Point::new(6.0, 2.0),
                ],
                number_of_segments: 50,
                ..widgets::graph::SplineGraph::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
        ]
    }
}