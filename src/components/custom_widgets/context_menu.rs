//! Display a dropdown list of selectable values.
use iced::{Color, Theme, Vector};
use iced_native::{
    event::{self, Event},
    keyboard, layout, mouse, overlay,
    overlay::menu::{self, Menu},
    renderer, text, touch,
    widget::tree::{self, Tree},
    Clipboard, Element, Layout, Length, Padding, Point, Rectangle, Shell, Size, Widget,
};
use iced_style::Background;
use std::borrow::Cow;

#[derive(Default)]
pub struct Appearance {
    pub shadow_offset: Vector,
    pub text_color: Color,
    pub placeholder_color: Color,
    pub background: Option<Background>,
    pub border_radius: f32,
    pub border_width: f32,
    pub border_color: Color,
    pub icon_size: f32,
}

/// A set of rules that dictate the style of a container.
pub trait StyleSheet {
    /// The supported style of the [`StyleSheet`].
    type Style: Default + Clone;

    /// Produces the active [`Appearance`] of a pick list.
    fn active(&self, style: &<Self as StyleSheet>::Style) -> Appearance;

    // Produces the hovered [`Appearance`] of a pick list.
    // fn hovered(&self, style: &<Self as StyleSheet>::Style) -> Appearance;
}

impl StyleSheet for Theme {
    type Style = ();
    fn active(&self, _style: &Self::Style) -> Appearance {
        let palette = self.extended_palette();

        Appearance {
            text_color: palette.background.weak.text,
            background: palette.background.weak.color.into(),
            placeholder_color: palette.background.strong.color,
            border_radius: 2.0,
            border_width: 1.0,
            border_color: palette.background.strong.color,
            icon_size: 0.7,
            ..Default::default()
        }
    }
}

pub fn context_menu<'a, Message, Renderer, T>(
    content: impl Into<Element<'a, Message, Renderer>>,
    options: impl Into<Cow<'a, [T]>>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> ContextMenu<'a, T, Message, Renderer>
where
    T: ToString + Eq + 'static,
    [T]: ToOwned<Owned = Vec<T>>,
    Renderer: iced_native::text::Renderer,
    Renderer::Theme: StyleSheet,
{
    ContextMenu::new(content, options, on_selected)
}

/// A widget for selecting a single value from a list of options.
#[allow(missing_debug_implementations)]
pub struct ContextMenu<'a, T, Message, Renderer>
where
    [T]: ToOwned<Owned = Vec<T>>,
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    content: Element<'a, Message, Renderer>,
    on_selected: Box<dyn Fn(T) -> Message + 'a>,
    options: Cow<'a, [T]>,
    width: Length,
    height: Length,
    padding: Padding,
    text_size: Option<f32>,
    font: Renderer::Font,
    style: <Renderer::Theme as StyleSheet>::Style,
}

impl<'a, T: 'a, Message, Renderer> ContextMenu<'a, T, Message, Renderer>
where
    T: ToString + Eq,
    [T]: ToOwned<Owned = Vec<T>>,
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    /// The default padding of a [`PickList`].
    pub const DEFAULT_PADDING: Padding = Padding::new(5.0);

    /// Creates a new [`PickList`] with the given list of options, the current
    /// selected value, and the message to produce when an option is selected.
    pub fn new(
        content: impl Into<Element<'a, Message, Renderer>>,
        options: impl Into<Cow<'a, [T]>>,
        on_selected: impl Fn(T) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_selected: Box::new(on_selected),
            options: options.into(),
            width: Length::Shrink,
            height: Length::Shrink,
            text_size: None,
            padding: Self::DEFAULT_PADDING,
            font: Default::default(),
            style: Default::default(),
        }
    }

    /// Sets the width of the [`PickList`].
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the [`PickList`].
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets the [`Padding`] of the [`PickList`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the text size of the [`PickList`].
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = Some(size);
        self
    }

    /// Sets the font of the [`PickList`].
    pub fn font(mut self, font: Renderer::Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the style of the [`PickList`].
    pub fn style(mut self, style: impl Into<<Renderer::Theme as StyleSheet>::Style>) -> Self {
        self.style = style.into();
        self
    }
}

impl<'a, T: 'a, Message, Renderer> Widget<Message, Renderer>
    for ContextMenu<'a, T, Message, Renderer>
where
    T: Clone + ToString + Eq + 'static,
    [T]: ToOwned<Owned = Vec<T>>,
    Message: 'a,
    Renderer: text::Renderer + 'a,
    Renderer::Theme: StyleSheet,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<T>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<T>::new())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content))
    }

    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        layout(
            renderer,
            limits,
            self.width,
            self.height,
            self.padding,
            self.text_size,
            &self.font,
            &self.options,
            |renderer, limits| self.content.as_widget().layout(renderer, limits),
        )
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        if let event::Status::Captured = self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event.clone(),
            layout.children().next().unwrap(),
            cursor_position,
            renderer,
            clipboard,
            shell,
        ) {
            return event::Status::Captured;
        }

        update(
            event,
            layout,
            cursor_position,
            shell,
            self.on_selected.as_ref(),
            &self.options,
            || tree.state.downcast_mut::<State<T>>(),
        )
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse_interaction(layout, cursor_position)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: theme.active(&self.style).text_color,
            },
            content_layout,
            cursor_position,
            &bounds,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        let state = tree.state.downcast_mut::<State<T>>();

        overlay(
            layout,
            state,
            self.padding,
            self.text_size,
            self.font.clone(),
            &self.options,
            self.style,
        )
    }
}

impl<'a, T: 'a, Message, Renderer> From<ContextMenu<'a, T, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    T: Clone + ToString + Eq + 'static,
    [T]: ToOwned<Owned = Vec<T>>,
    Message: 'a,
    Renderer: text::Renderer + 'a,
    Renderer::Theme: StyleSheet,
{
    fn from(context_menu: ContextMenu<'a, T, Message, Renderer>) -> Self {
        Self::new(context_menu)
    }
}

/// The local state of a [`PickList`].
#[derive(Debug)]
pub struct State<T> {
    menu: menu::State,
    keyboard_modifiers: keyboard::Modifiers,
    is_open: bool,
    hovered_option: Option<usize>,
    last_selection: Option<T>,
}

impl<T> State<T> {
    /// Creates a new [`State`] for a [`PickList`].
    pub fn new() -> Self {
        Self {
            menu: menu::State::default(),
            keyboard_modifiers: keyboard::Modifiers::default(),
            is_open: bool::default(),
            hovered_option: Option::default(),
            last_selection: Option::default(),
        }
    }
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Computes the layout of a [`PickList`].
pub fn layout<Renderer, T>(
    renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    height: Length,
    padding: Padding,
    text_size: Option<f32>,
    font: &Renderer::Font,
    options: &[T],
    layout_content: impl FnOnce(&Renderer, &layout::Limits) -> layout::Node,
) -> layout::Node
where
    Renderer: text::Renderer,
    T: ToString,
{
    // use std::f32;

    let limits = limits.width(width).height(height);

    let content = layout_content(renderer, &limits.width(width).height(height));
    // content.move_to(Point::new(padding.left.into(), padding.top.into()));

    // let size = limits.resolve(content.size()).pad(padding);

    // let limits = limits.width(width).height(Length::Shrink).pad(padding);

    let text_size = text_size.unwrap_or_else(|| renderer.default_size());

    let max_width = match width {
        Length::Shrink => {
            let measure = |label: &str| -> u32 {
                let (width, _) = renderer.measure(
                    label,
                    text_size,
                    font.clone(),
                    Size::new(f32::INFINITY, f32::INFINITY),
                );

                width.round() as u32
            };

            let labels = options.iter().map(ToString::to_string);

            let labels_width = labels.map(|label| measure(&label)).max().unwrap_or(100);

            let placeholder_width = content.size().width;

            labels_width.max(placeholder_width as u32)
        }
        _ => 0,
    };

    let size = {
        let intrinsic = Size::new(
            max_width as f32 + f32::from(text_size),
            content.size().height.max(f32::from(text_size)),
        );

        limits.resolve(intrinsic)
    };

    // layout::Node::new(size)
    layout::Node::with_children(size, vec![content])
}

/// Processes an [`Event`] and updates the [`State`] of a [`PickList`]
/// accordingly.
pub fn update<'a, T, Message>(
    event: Event,
    layout: Layout<'_>,
    cursor_position: Point,
    shell: &mut Shell<'_, Message>,
    on_selected: &dyn Fn(T) -> Message,
    options: &[T],
    state: impl FnOnce() -> &'a mut State<T>,
) -> event::Status
where
    T: PartialEq + Clone + 'a,
{
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) => {
            let state = state();

            let event_status = if state.is_open {
                // TODO: Encode cursor availability in the type system
                state.is_open = cursor_position.x < 0.0 || cursor_position.y < 0.0;

                event::Status::Ignored
            } else {
                event::Status::Ignored
            };

            if let Some(last_selection) = state.last_selection.take() {
                shell.publish((on_selected)(last_selection));

                state.is_open = false;

                event::Status::Ignored
            } else {
                event_status
            }
        }
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
            let state = state();

            if state.is_open {
                // TODO: Encode cursor availability in the type system
                state.is_open = cursor_position.x < 0.0 || cursor_position.y < 0.0;

                event::Status::Captured
            } else if layout.bounds().contains(cursor_position) {
                state.is_open = true;

                event::Status::Captured
            } else {
                event::Status::Ignored
            }
        }
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { y, .. },
        }) => {
            let state = state();

            if state.keyboard_modifiers.command()
                && layout.bounds().contains(cursor_position)
                && !state.is_open
            {
                fn find_next<'a, T: PartialEq>(
                    selected: &'a T,
                    mut options: impl Iterator<Item = &'a T>,
                ) -> Option<&'a T> {
                    let _ = options.find(|&option| option == selected);

                    options.next()
                }

                let next_option = if y < 0.0 {
                    options.first()
                } else if y > 0.0 {
                    options.last()
                } else {
                    None
                };

                if let Some(next_option) = next_option {
                    shell.publish((on_selected)(next_option.clone()));
                }

                event::Status::Captured
            } else {
                event::Status::Ignored
            }
        }
        Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
            let state = state();

            state.keyboard_modifiers = modifiers;

            event::Status::Ignored
        }
        _ => event::Status::Ignored,
    }
}

/// Returns the current [`mouse::Interaction`] of a [`PickList`].
pub fn mouse_interaction(layout: Layout<'_>, cursor_position: Point) -> mouse::Interaction {
    let bounds = layout.bounds();
    let is_mouse_over = bounds.contains(cursor_position);

    if is_mouse_over {
        mouse::Interaction::Pointer
    } else {
        mouse::Interaction::default()
    }
}

/// Returns the current overlay of a [`PickList`].
pub fn overlay<'a, T, Message, Renderer>(
    layout: Layout<'_>,
    state: &'a mut State<T>,
    padding: Padding,
    text_size: Option<f32>,
    font: Renderer::Font,
    options: &'a [T],
    style: <Renderer::Theme as StyleSheet>::Style,
) -> Option<overlay::Element<'a, Message, Renderer>>
where
    T: Clone + ToString,
    Message: 'a,
    Renderer: text::Renderer + 'a,
    Renderer::Theme: StyleSheet,
{
    if state.is_open {
        let bounds = layout.bounds();

        let mut menu = Menu::new(
            &mut state.menu,
            options,
            &mut state.hovered_option,
            &mut state.last_selection,
        )
        .width(bounds.width.round())
        .padding(padding)
        .font(font)
        .style(style);

        if let Some(text_size) = text_size {
            menu = menu.text_size(text_size);
        }

        Some(menu.overlay(layout.position(), bounds.height))
    } else {
        None
    }
}
