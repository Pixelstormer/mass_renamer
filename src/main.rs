use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};

use iced::{
    button, container, executor, text_input, Application, Button, Color, Column, Command,
    Container, Element, Length, Rule, Settings, Text, TextInput,
};
use native_dialog::FileDialog;

pub fn main() -> iced::Result {
    App::run(Settings::with_flags(std::env::args()))
}

#[derive(Default)]
pub struct ContainerStyle(pub container::Style);

impl ContainerStyle {
    fn whitesmoke() -> Self {
        Self(container::Style {
            background: Color::from_rgb8(0xf5, 0xf5, 0xf5).into(),
            ..Default::default()
        })
    }
}

impl container::StyleSheet for ContainerStyle {
    fn style(&self) -> container::Style {
        self.0
    }
}

#[derive(Default)]
pub struct ButtonStyle {
    pub active: button::Style,
    pub hovered: button::Style,
    pub pressed: button::Style,
    pub disabled: button::Style,
}

impl ButtonStyle {
    fn blank() -> Self {
        Self::default()
    }

    fn whitesmoke() -> Self {
        Self {
            active: button::Style {
                background: Color::from_rgb8(0xf5, 0xf5, 0xf5).into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl button::StyleSheet for ButtonStyle {
    fn active(&self) -> button::Style {
        self.active
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    HighlightInputChanged(String),
    FileButtonPressed,
    FilesRecieved(Arc<native_dialog::Result<Vec<PathBuf>>>),
    FilenamePressed(usize),
}

#[derive(Default)]
pub struct Entry {
    pub state: button::State,
    pub text: String,
    pub selected: bool,
    pub malformed: bool,
}

impl<T: AsRef<Path>> From<T> for Entry {
    fn from(p: T) -> Self {
        let text = p.as_ref().to_string_lossy();
        Self {
            malformed: matches!(text, Cow::Owned(_)),
            text: text.into(),
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct App {
    pub filenames: Vec<Entry>,
    pub highlight_input_state: text_input::State,
    pub highlight_input_value: String,
    pub file_button_state: button::State,
}

impl App {
    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = std::env::Args;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            App {
                filenames: flags.skip(1).map(Entry::from).collect(),
                ..Default::default()
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        format!("Mass Renamer - Version {}", Self::VERSION)
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        use Message::*;
        match message {
            HighlightInputChanged(input) => self.highlight_input_value = input,
            FileButtonPressed => {
                return Command::perform(
                    async { FileDialog::new().show_open_multiple_file() },
                    |r| Message::FilesRecieved(Arc::new(r)),
                );
            }
            FilesRecieved(files) => {
                if let Ok(paths) = &*files {
                    self.filenames.extend(paths.iter().map(Entry::from));
                }
            }
            FilenamePressed(index) => {
                self.filenames[index].selected = true;
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Column::new()
            .padding(16)
            .spacing(16)
            .push(
                Button::new(&mut self.file_button_state, Text::new("Open Files"))
                    .on_press(Message::FileButtonPressed),
            )
            .push(
                TextInput::new(
                    &mut self.highlight_input_state,
                    "Highlight Text...",
                    &self.highlight_input_value,
                    Message::HighlightInputChanged,
                )
                .padding(4),
            )
            .push(
                Container::new(
                    self.filenames.iter_mut().enumerate().fold(
                        Column::new()
                            .push(Text::new("Text"))
                            .push(Rule::horizontal(0)),
                        |c, (i, entry)| {
                            let mut text = Text::new(&entry.text);

                            for (index, _) in entry.text.match_indices(&self.highlight_input_value)
                            {
                                text = text.highlight(
                                    index,
                                    index + self.highlight_input_value.len(),
                                    Color::from_rgb8(0xff, 0xc0, 0xcb),
                                );
                            }

                            let mut button = Button::new(&mut entry.state, text)
                                .on_press(Message::FilenamePressed(i))
                                .width(Length::Fill);

                            button = if i % 2 == 0 {
                                button.style(ButtonStyle::blank())
                            } else {
                                button.style(ButtonStyle::whitesmoke())
                            };

                            c.push(button)
                        },
                    ),
                )
                .padding(1)
                .height(Length::Fill)
                .style(ContainerStyle(container::Style {
                    border_color: Color::from_rgb8(190, 190, 190),
                    border_width: 1.0,
                    ..Default::default()
                })),
            )
            .into()
    }
}
