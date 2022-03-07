mod listbox;

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};

use listbox::ListBox;

use iced::{
    button, container, executor, scrollable, text_input, Application, Button, Color, Column,
    Command, Container, Element, Length, Scrollable, Settings, Text, TextInput,
};
use native_dialog::FileDialog;

fn main() -> iced::Result {
    App::run(Settings::with_flags(std::env::args()))
}

#[derive(Clone, Debug)]
enum Message {
    HighlightInputChanged(String),
    FileButtonPressed,
    FilesRecieved(Arc<native_dialog::Result<Vec<PathBuf>>>),
    FilesDeleted(Vec<bool>),
}

struct Entry {
    text: String,
    malformed: bool,
}

impl<T: AsRef<Path>> From<T> for Entry {
    fn from(p: T) -> Self {
        let text = p.as_ref().to_string_lossy();
        Self {
            malformed: matches!(text, Cow::Owned(_)),
            text: text.into(),
        }
    }
}

#[derive(Default)]
struct App {
    entries: Vec<Entry>,
    highlight_input_state: text_input::State,
    highlight_input_value: String,
    file_button_state: button::State,
    listbox_state: listbox::State,
    scroll_state: scrollable::State,
}

impl App {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = std::env::Args;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            App {
                entries: flags.skip(1).map(Entry::from).collect(),
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
                    self.entries.extend(paths.iter().map(Entry::from));
                }
            }
            FilesDeleted(indexes) => {
                let mut iter = indexes.iter();
                self.entries.retain(|_| !iter.next().unwrap());
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Column::with_children(vec![
            Button::new(&mut self.file_button_state, Text::new("Open Files"))
                .on_press(Message::FileButtonPressed)
                .into(),
            TextInput::new(
                &mut self.highlight_input_state,
                "Highlight Text...",
                &self.highlight_input_value,
                Message::HighlightInputChanged,
            )
            .padding(4)
            .into(),
            Container::new(
                Scrollable::new(&mut self.scroll_state).push(
                    ListBox::with_children(
                        &mut self.listbox_state,
                        self.entries
                            .iter()
                            .map(|e| {
                                if self.highlight_input_value.is_empty() {
                                    Text::new(&e.text)
                                } else {
                                    e.text.match_indices(&self.highlight_input_value).fold(
                                        Text::new(&e.text),
                                        |t, (i, _)| {
                                            t.highlight(
                                                i,
                                                i + self.highlight_input_value.len(),
                                                Color::from_rgb8(0xff, 0xc0, 0xcb),
                                            )
                                        },
                                    )
                                }
                                .into()
                            })
                            .collect(),
                        Message::FilesDeleted,
                    )
                    .width(Length::Fill)
                    .padding([1, 23])
                    .spacing(4)
                    .style(listbox::Style::light(true)),
                ),
            )
            .height(Length::Fill)
            .padding(1)
            .style(ContainerStyle)
            .into(),
        ])
        .padding(16)
        .spacing(16)
        .into()
    }
}

struct ContainerStyle;

impl container::StyleSheet for ContainerStyle {
    fn style(&self) -> container::Style {
        container::Style {
            border_width: 1.0,
            border_color: Color::from_rgb8(0xbe, 0xbe, 0xbe),
            ..Default::default()
        }
    }
}
