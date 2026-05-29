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
                    Point {
                        x: 0.0,
                        y: 2.0,
                    },
                    Point {
                        x: 1.0,
                        y: 3.0,
                    },
                    Point {
                        x: 2.0,
                        y: 3.0,
                    },
                    Point {
                        x: 3.0,
                        y: 4.0,
                    },
                    Point {
                        x: 4.0,
                        y: 1.0,
                    },
                    Point {
                        x: 5.0,
                        y: 3.0,
                    },
                    Point {
                        x: 6.0,
                        y: 5.0,
                    },
                    Point {
                        x: 7.0,
                        y: 2.0,
                    },
                    Point {
                        x: 8.0,
                        y: 3.2375002,
                    },
                    Point {
                        x: 9.0,
                        y: 3.2375002,
                    },
                    Point {
                        x: 10.0,
                        y: 4.3916664,
                    },
                    Point {
                        x: 11.0,
                        y: 2.7041667,
                    },
                    Point {
                        x: 12.0,
                        y: 2.7041667,
                    },
                    Point {
                        x: 13.0,
                        y: 2.7041667,
                    },
                    Point {
                        x: 14.0,
                        y: 4.258333,
                    },
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
