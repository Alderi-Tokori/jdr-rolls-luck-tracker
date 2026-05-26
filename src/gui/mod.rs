use iced::{Element, Point, Task};

mod widgets;
mod dashboard;

pub struct State {
    screen: Screen,
}

enum Screen {
    Dashboard(dashboard::Dashboard),
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Dashboard(dashboard::Message),
}

impl Default for State {
    fn default() -> Self {
        Self {
            screen: Screen::Dashboard(dashboard::Dashboard {
                graph_points: vec![
                    Point::new(0.0, 2.0),
                    Point::new(1.0, 5.0),
                    Point::new(2.0, 3.0),
                    Point::new(3.0, 1.0),
                    Point::new(4.0, 4.0),
                    Point::new(6.0, 2.0),
                ]
            }),
        }
    }
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Dashboard(message) => {
            if let Screen::Dashboard(dashboard) = &mut state.screen {
                let action = dashboard.update(message);

                match action {
                    dashboard::Action::None => Task::none(),
                    dashboard::Action::Run(task) => task.map(Message::Dashboard),
                }
            } else {
                Task::none()
            }
        }
    }
}

pub fn view<'a>(state: &'a State) -> Element<'a, Message> {
    match &state.screen {
        Screen::Dashboard(dashboard) => dashboard.view().map(Message::Dashboard).into(),
    }
}
