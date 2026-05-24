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
            button("+").on_press(Message::Increment),
            text(self.value),
            button("-").on_press(Message::Decrement),
             canvas(widgets::graph::LineGraph {
                data: vec![
                    Point::new(0.0, 0.0),
                    Point::new(1.0, 2.0),
                    Point::new(2.0, 1.5),
                    Point::new(3.0, 3.0),
                    Point::new(4.0, 4.5),
                ],
                ..widgets::graph::LineGraph::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
        ]
    }
}