//! Display a vertical list of selectable widgets.
//!
//! A [`ListBox`] has some local [`State`].

use iced::{
    keyboard::{self, KeyCode},
    mouse, Alignment, Color, Length, Point, Rectangle,
};
use iced_native::{
    event,
    layout::{flex, Limits, Node},
    overlay,
    renderer::{self, Quad, Renderer},
    touch, Clipboard, Element, Event, Layout, Padding, Shell, Widget,
};

/// A vertically aligned container that supports interactively selecting its contents.
pub struct ListBox<'a, M, R> {
    state: &'a mut State,
    style: Box<dyn StyleSheet + 'a>,
    max_width: u32,
    max_height: u32,
    width: Length,
    height: Length,
    padding: Padding,
    spacing: f32,
    align_items: Alignment,
    children: Vec<Element<'a, M, R>>,
    on_delete: Box<dyn Fn(Vec<bool>) -> M>,
}

impl<'a, M, R: Renderer> ListBox<'a, M, R> {
    /// Creates an empty [`ListBox`] with the given [`State`], and a function that produces a message when
    /// the user wants to delete the currently selected elements.
    pub fn new(state: &'a mut State, on_delete: impl Fn(Vec<bool>) -> M + 'static) -> Self {
        Self::with_children(state, Vec::new(), on_delete)
    }

    /// Creates a [`ListBox`] with the given [`State`], child elements and function that produces a message when
    /// the user wants to delete the currently selected elements.
    pub fn with_children(
        state: &'a mut State,
        children: Vec<Element<'a, M, R>>,
        on_delete: impl Fn(Vec<bool>) -> M + 'static,
    ) -> Self {
        state.selected_children.resize(children.len(), false);
        Self {
            state,
            style: Style::default().into(),
            max_width: u32::MAX,
            max_height: u32::MAX,
            width: Length::Fill,
            height: Length::Fill,
            padding: 0.into(),
            spacing: 0.0,
            align_items: Alignment::Start,
            children,
            on_delete: Box::new(on_delete),
        }
    }

    /// Sets the style of the [`ListBox`].
    pub fn style(mut self, style_sheet: impl Into<Box<dyn StyleSheet + 'a>>) -> Self {
        self.style = style_sheet.into();
        self
    }

    /// Sets the maximum width of the [`ListBox`] in pixels.
    pub fn max_width(mut self, max_width: u32) -> Self {
        self.max_width = max_width;
        self
    }

    /// Sets the maximum height of the [`ListBox`] in pixels.
    pub fn max_height(mut self, max_height: u32) -> Self {
        self.max_height = max_height;
        self
    }

    /// Sets the width of the [`ListBox`].
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the [`ListBox`].
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets the padding of the [`ListBox`].
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the vertical spacing _between_ elements for the contents of the [`ListBox`].
    pub fn spacing(mut self, units: u16) -> Self {
        self.spacing = units as f32;
        self
    }

    /// Sets the horizontal alignment of the contents of the [`ListBox`] .
    pub fn align_items(mut self, align: Alignment) -> Self {
        self.align_items = align;
        self
    }

    /// Adds an element to the [`ListBox`].
    pub fn push(mut self, child: impl Into<Element<'a, M, R>>) -> Self {
        self.children.push(child.into());
        self.state.selected_children.push(false);
        self
    }
}

impl<M, R: Renderer> Widget<M, R> for ListBox<'_, M, R> {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, renderer: &R, limits: &Limits) -> Node {
        flex::resolve(
            flex::Axis::Vertical,
            renderer,
            &limits
                .max_width(self.max_width)
                .max_height(self.max_height)
                .width(self.width)
                .height(self.height),
            self.padding,
            self.spacing,
            self.align_items,
            &self.children,
        )
    }

    fn draw(
        &self,
        renderer: &mut R,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        let style = self.style.style();
        let bounds = layout.bounds();

        // Base background
        renderer.fill_quad(
            Quad {
                bounds,
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            style.background,
        );

        for (i, (child, child_layout)) in self.children.iter().zip(layout.children()).enumerate() {
            let mut renderer_style = renderer::Style {
                text_color: style.text_color,
            };

            let background_bounds = selection_bounds(
                self.padding,
                self.spacing,
                bounds,
                child_layout.bounds(),
                style,
            );

            if self.state.selected_children[i] {
                // Selected elements
                renderer.fill_quad(
                    Quad {
                        bounds: background_bounds,
                        border_radius: 0.0,
                        border_width: 0.0,
                        border_color: Color::TRANSPARENT,
                    },
                    style.selected_background,
                );

                if let Some(colour) = style.selected_text_color {
                    renderer_style.text_color = colour;
                }
            } else if let Some(background) = style.stripe_background {
                // Every second element has the stripe colour
                if i % 2 == 1 {
                    renderer.fill_quad(
                        Quad {
                            bounds: background_bounds,
                            border_radius: 0.0,
                            border_width: 0.0,
                            border_color: Color::TRANSPARENT,
                        },
                        background,
                    );
                }
            }

            // Children
            child.draw(
                renderer,
                &renderer_style,
                child_layout,
                cursor_position,
                viewport,
            );
        }

        // Border, rendered after everything else so that rounded corners don't get clipped
        renderer.fill_quad(
            Quad {
                bounds,
                border_radius: style.border_radius,
                border_width: style.border_width,
                border_color: style.border_color,
            },
            Color::TRANSPARENT,
        );
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
    ) -> event::Status {
        let mut iter = self
            .children
            .iter_mut()
            .zip(layout.children())
            .map(|(child, layout)| {
                (
                    child.on_event(
                        event.clone(),
                        layout,
                        cursor_position,
                        renderer,
                        clipboard,
                        shell,
                    ),
                    layout,
                )
            });

        if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            self.state.modifiers = modifiers;
        }

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key_code: KeyCode::Delete,
                ..
            }) => {
                iter.find_map(|(s, _)| matches!(s, event::Status::Captured).then(|| s))
                    .unwrap_or_else(|| {
                        if self.state.is_selected {
                            // Deselect all elements and give the previously selected values to the message
                            let len = self.state.selected_children.len();
                            shell.publish((self.on_delete)(std::mem::replace(
                                &mut self.state.selected_children,
                                vec![false; len],
                            )));
                            event::Status::Captured
                        } else {
                            event::Status::Ignored
                        }
                    })
            }
            Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let bounds = layout.bounds();
                let style = self.style.style();
                iter.enumerate()
                    .filter_map(|(i, (s, l))| match s {
                        event::Status::Ignored => {
                            selection_bounds(self.padding, self.spacing, bounds, l.bounds(), style)
                                .contains(cursor_position)
                                .then(|| Some(i))
                        }
                        event::Status::Captured => Some(None),
                    })
                    .collect::<Option<Vec<_>>>()
                    .map_or(event::Status::Captured, |v| {
                        self.state.is_selected = bounds.contains(cursor_position);
                        v.into_iter().fold(event::Status::Ignored, |_, i| {
                            self.state.select(i);
                            event::Status::Captured
                        })
                    })
            }
            _ => iter
                .find_map(|(s, _)| matches!(s, event::Status::Captured).then(|| s))
                .unwrap_or(event::Status::Ignored),
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &R,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(layout.children())
            .map(|(child, layout)| {
                child.mouse_interaction(layout, cursor_position, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn overlay(&mut self, layout: Layout<'_>, renderer: &R) -> Option<overlay::Element<'_, M, R>> {
        self.children
            .iter_mut()
            .zip(layout.children())
            .find_map(|(child, layout)| child.overlay(layout, renderer))
    }
}

impl<'a, M: 'a, R: 'a + Renderer> From<ListBox<'a, M, R>> for Element<'a, M, R> {
    fn from(l: ListBox<'a, M, R>) -> Self {
        Element::new(l)
    }
}

#[doc(hidden)]
fn selection_bounds(
    padding: Padding,
    spacing: f32,
    bounds: Rectangle,
    child_bounds: Rectangle,
    style: &Style,
) -> Rectangle {
    let y;
    let height;
    if child_bounds.y == bounds.y + padding.top as f32 {
        // The topmost element - align with top border
        y = bounds.y + style.border_width;
        height = (child_bounds.height + padding.top as f32 + (spacing * 0.5)) - style.border_width;
    } else {
        // Otherwise, align with the above element
        y = child_bounds.y - (spacing * 0.5);
        height = child_bounds.height + spacing;
    }

    // Horizontally aligned to be flush with both side borders
    Rectangle {
        x: bounds.x + style.border_width,
        y,
        width: bounds.width - (style.border_width * 2.0),
        height,
    }
}

/// The local state of a [`ListBox`].
#[derive(Default)]
pub struct State {
    is_selected: bool,
    selected_children: Vec<bool>,
    modifiers: keyboard::Modifiers,
    most_recently_selected: Option<usize>,
}

impl State {
    /// Creates a new [`State`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Performs a selection operation according to the given index and current [`Modifiers`].
    ///
    /// If [`shift`] is pressed, performs a continuous selection - all elements between the given
    /// index and the most recent selection are selected, and everything else is deselected.
    ///
    /// Otherwise, if [`command`] is pressed, performs a disjoint selection - selection of the element
    /// at the given index is toggled, and everything else remains untouched.
    ///
    /// Otherwise, performs a single selection - the element at the given index is selected, and everything
    /// else is deselected.
    ///
    /// [`Modifiers`]: keyboard::Modifiers
    /// [`shift`]: keyboard::Modifiers::shift
    /// [`command`]: keyboard::Modifiers::command
    pub fn select(&mut self, index: usize) {
        if self.modifiers.shift() && self.most_recently_selected.is_some() {
            // Continuous select
            let i = self.most_recently_selected.unwrap();
            let min = index.min(i);
            let (left, right) = self.selected_children.split_at_mut(min);
            let (middle, right) = right.split_at_mut((index.max(i) - min) + 1);
            left.fill(false);
            middle.fill(true);
            right.fill(false);
        } else if self.modifiers.command() {
            // Disjoint select
            self.selected_children[index] = !self.selected_children[index];
            self.most_recently_selected = Some(index);
        } else {
            // Single select
            self.selected_children.fill(false);
            self.selected_children[index] = true;
            self.most_recently_selected = Some(index);
        }
    }
}

/// The appearance of a [`ListBox`].
pub struct Style {
    /// The background colour for child elements.
    pub background: Color,
    /// The background colour for every second child element. If [`None`], uses the base background colour instead.
    pub stripe_background: Option<Color>,
    /// The background colour for selected elements.
    pub selected_background: Color,
    /// The text colour for unselected elements.
    pub text_color: Color,
    /// The text colour for selected elements. If [`None`], is the same as the unselected text colour.
    pub selected_text_color: Option<Color>,
    /// Controls how rounded the border's corners are.
    pub border_radius: f32,
    /// The thickness of the border.
    pub border_width: f32,
    /// The colour of the border.
    pub border_color: Color,
}

impl Style {
    /// A styling suitable for a light theme.
    pub fn light() -> Self {
        Self {
            background: Color::WHITE,
            stripe_background: Some(Color::from_rgb8(0xf5, 0xf5, 0xf5)),
            selected_background: Color::from_rgb8(0x30, 0x8e, 0xc9),
            text_color: Color::BLACK,
            selected_text_color: Some(Color::WHITE),
            border_radius: 0.0,
            border_width: 1.0,
            border_color: Color::from_rgb8(0xbe, 0xbe, 0xbe),
        }
    }

    /// A styling suitable for a dark theme.
    pub fn dark() -> Self {
        todo!();
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::light()
    }
}

/// Calculates the style to be used by a [`ListBox`].
pub trait StyleSheet {
    fn style(&self) -> &Style;
}

impl StyleSheet for Style {
    fn style(&self) -> &Style {
        self
    }
}

impl<'a, T: 'a + StyleSheet> From<T> for Box<dyn StyleSheet + 'a> {
    fn from(style: T) -> Self {
        Box::new(style)
    }
}
