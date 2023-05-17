//! Use a floating element to overlay an element over some content
//!
//! *This API requires the following crate features to be activated: `floating_element`*
use iced_native::{
    event, mouse, overlay, Clipboard, Event, Layout, Length, Point, Rectangle, Shell,
};
use iced_native::{
    widget::{Operation, Tree},
    Element, Widget,
};

pub use anchor::Anchor;
use cp_overlay::FloatingElementOverlay;
pub use offset::Offset;

/// A floating element floating over some content.
///
/// # Example
/// ```
/// # use iced_native::renderer::Null;
/// # use iced_native::widget::{button, Button, Column, Text};
/// # use iced_aw::native::floating_element;
/// #
/// # pub type FloatingElement<'a, B, Message> = floating_element::FloatingElement<'a, B, Message, Null>;
/// #[derive(Debug, Clone)]
/// enum Message {
///     ButtonPressed,
/// }
///
/// let content = Column::new();
/// let floating_element = FloatingElement::new(
///     content,
///     || Button::new(Text::new("Press Me!"))
///         .on_press(Message::ButtonPressed)
///         .into()
/// );
/// ```
#[allow(missing_debug_implementations)]
pub struct FloatingElement<'a, B, Message, Renderer>
where
    B: Fn() -> Element<'a, Message, Renderer>,
    Message: Clone,
    Renderer: iced_native::Renderer,
{
    /// The anchor of the element.
    anchor: Anchor,
    /// The offset of the element.
    offset: Offset,
    /// The visibility of the element.
    hidden: bool,
    /// The optional message that will be send when the user clicked on the backdrop.
    backdrop: Option<Message>,
    /// The optional message that will be send when the ESC key was pressed.
    esc: Option<Message>,
    /// The underlying element.
    underlay: Element<'a, Message, Renderer>,
    /// The floating element of the [`FloatingElementOverlay`](FloatingElementOverlay).
    element: B,
}

impl<'a, B, Message, Renderer> FloatingElement<'a, B, Message, Renderer>
where
    B: Fn() -> Element<'a, Message, Renderer>,
    Message: Clone,
    Renderer: iced_native::Renderer,
{
    /// Creates a new [`FloatingElement`](FloatingElement) over some content,
    /// showing the given [`Element`](iced_native::Element).
    ///
    /// It expects:
    ///     * the underlay [`Element`](iced_native::Element) on which this [`FloatingElement`](FloatingElement)
    ///         will be wrapped around.
    ///     * a function that will lazy create the [`Element`](iced_native::Element) for the overlay.
    pub fn new<U>(underlay: U, element: B) -> Self
    where
        U: Into<Element<'a, Message, Renderer>>,
    {
        FloatingElement {
            anchor: Anchor::SouthEast,
            offset: 5.0.into(),
            hidden: false,
            backdrop: None,
            esc: None,
            underlay: underlay.into(),
            element,
        }
    }

    /// Sets the [`Anchor`](Anchor) of the [`FloatingElement`](FloatingElement).
    #[must_use]
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Sets the [`Offset`](Offset) of the [`FloatingElement`](FloatingElement).
    #[must_use]
    pub fn offset<O>(mut self, offset: O) -> Self
    where
        O: Into<Offset>,
    {
        self.offset = offset.into();
        self
    }

    /// Hide or unhide the [`Element`](iced_native::Element) on the
    /// [`FloatingElement`](FloatingElement).
    #[must_use]
    pub fn hide(mut self, hide: bool) -> Self {
        self.hidden = hide;
        self
    }

    /// Sets the message that will be produced when the backdrop of the
    /// [`FloatingElement`](FloatingElement) is clicked.
    #[must_use]
    pub fn backdrop(mut self, message: Message) -> Self {
        self.backdrop = Some(message);
        self
    }

    /// Sets the message that will be produced when the Escape Key is
    /// pressed when the floating element is open.
    ///
    /// This can be used to close the modal on ESC.
    #[must_use]
    pub fn on_esc(mut self, message: Message) -> Self {
        self.esc = Some(message);
        self
    }
}

impl<'a, B, Message, Renderer> Widget<Message, Renderer>
    for FloatingElement<'a, B, Message, Renderer>
where
    B: Fn() -> Element<'a, Message, Renderer>,
    Message: 'a + Clone,
    Renderer: 'a + iced_native::Renderer,
{
    fn children(&self) -> Vec<iced_native::widget::Tree> {
        vec![Tree::new(&self.underlay), Tree::new(&(self.element)())]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.underlay, &(self.element)()]);
    }

    fn width(&self) -> Length {
        self.underlay.as_widget().width()
    }

    fn height(&self) -> Length {
        self.underlay.as_widget().height()
    }

    fn layout(
        &self,
        renderer: &Renderer,
        limits: &iced_native::layout::Limits,
    ) -> iced_native::layout::Node {
        self.underlay.as_widget().layout(renderer, limits)
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        self.underlay.as_widget_mut().on_event(
            &mut state.children[0],
            event,
            layout,
            cursor_position,
            renderer,
            clipboard,
            shell,
        )
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.underlay.as_widget().mouse_interaction(
            &state.children[0],
            layout,
            cursor_position,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        state: &iced_native::widget::Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        style: &iced_native::renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        self.underlay.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor_position,
            viewport,
        );
    }

    fn operate<'b>(
        &'b self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        self.underlay
            .as_widget()
            .operate(&mut state.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        if self.hidden {
            return self
                .underlay
                .as_widget_mut()
                .overlay(&mut state.children[0], layout, renderer);
        }

        if state.children.len() == 2 {
            let bounds = layout.bounds();

            let position = match self.anchor {
                Anchor::NorthWest => Point::new(0.0, 0.0),
                Anchor::NorthEast => Point::new(bounds.width, 0.0),
                Anchor::SouthWest => Point::new(0.0, bounds.height),
                Anchor::SouthEast => Point::new(bounds.width, bounds.height),
                Anchor::North => Point::new(bounds.center_x(), 0.0),
                Anchor::East => Point::new(bounds.width, bounds.center_y()),
                Anchor::South => Point::new(bounds.center_x(), bounds.height),
                Anchor::West => Point::new(0.0, bounds.center_y()),
                Anchor::Center => Point::new(bounds.x, bounds.y),
            };

            let position = Point::new(bounds.x + position.x, bounds.y + position.y);

            Some(
                FloatingElementOverlay::new(
                    &mut state.children[1],
                    (self.element)(),
                    self.backdrop.clone(),
                    self.esc.clone(),
                    &self.anchor,
                    &self.offset,
                )
                .overlay(position),
            )
        } else {
            None
        }
    }
}

impl<'a, B, Message, Renderer> From<FloatingElement<'a, B, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    B: 'a + Fn() -> Element<'a, Message, Renderer>,
    Message: 'a + Clone,
    Renderer: 'a + iced_native::Renderer,
{
    fn from(floating_element: FloatingElement<'a, B, Message, Renderer>) -> Self {
        Element::new(floating_element)
    }
}

mod anchor {
    //! Use a floating button to overlay a button over some content
    //!
    //! *This API requires the following crate features to be activated: `floating_button`*

    /// Positional [`Anchor`](Anchor) for the [`FloatingButton`](super::FloatingButton).
    #[derive(Copy, Clone, Debug, Hash)]
    pub enum Anchor {
        /// NorthWest [`Anchor`](Anchor) for positioning the
        /// [`Button`](iced_native::widget::button::Button) on the top left of the
        /// underlying element.
        NorthWest,

        /// NorthEast [`Anchor`](Anchor) for positioning the
        /// [`Button`](iced_native::widget::button::Button) on the top right of the
        /// underlying element.
        NorthEast,

        /// SouthWest [`Anchor`](Anchor) for positioning the
        /// [`Button`](iced_native::widget::button::Button) on the bottom left of the
        /// underlying element.
        SouthWest,

        /// SouthEast [`Anchor`](Anchor) for positioning the
        /// [`Button`](iced_native::widget::button::Button) on the bottom right of the
        /// underlying element.
        SouthEast,

        /// North [`Anchor`](Anchor) for positioning the
        /// [`Button`](iced_native::widget::button::Button) on the top of the
        /// underlying element.
        North,

        /// East [`Anchor`](Anchor) for positioning the
        /// [`Button`](iced_native::widget::button::Button) on the right of the
        /// underlying element.
        East,

        /// South [`Anchor`](Anchor) for positioning the
        /// [`Button`](iced_native::widget::button::Button) on the bottom of the
        /// underlying element.
        South,

        /// West [`Anchor`](Anchor) for positioning the
        /// [`Button`](iced_native::widget::button::Button) on the left of the
        /// underlying element.
        West,

        /// Center [`Anchor`](Anchor) for positioning the
        /// [`Button`](iced_native::widget::button::Button) on the center of the
        /// underlying element.
        Center,
    }
}

mod offset {
    //! Use a floating button to overlay a button over some content
    //!
    //! *This API requires the following crate features to be activated: `floating_button`*

    use iced_native::Point;

    /// The [`Offset`](Offset) for the [`FloatingButton`](super::FloatingButton).
    #[derive(Copy, Clone, Debug)]
    pub struct Offset {
        /// Offset on the x-axis from the [`Anchor`](super::Anchor)
        pub x: f32,
        /// Offset on the y-axis from the [`Anchor`](super::Anchor)
        pub y: f32,
    }

    impl From<f32> for Offset {
        fn from(float: f32) -> Self {
            Self { x: float, y: float }
        }
    }

    impl From<[f32; 2]> for Offset {
        fn from(array: [f32; 2]) -> Self {
            Self {
                x: array[0],
                y: array[1],
            }
        }
    }

    impl From<Offset> for Point {
        fn from(offset: Offset) -> Self {
            Self::new(offset.x, offset.y)
        }
    }

    impl From<&Offset> for Point {
        fn from(offset: &Offset) -> Self {
            Self::new(offset.x, offset.y)
        }
    }
}

mod cp_overlay {
    //! Use a floating element to overlay a element over some content
    //!
    //! *This API requires the following crate features to be activated: `floating_element`*
    use iced_native::{
        event, layout::Limits, overlay, Clipboard, Event, Layout, Point, Shell, Size,
    };
    use iced_native::{keyboard, mouse, touch, Rectangle, Vector};
    use iced_native::{widget::Tree, Element};

    use super::{Anchor, Offset};

    /// The internal overlay of a [`FloatingElement`](crate::FloatingElement) for
    /// rendering a [`Element`](iced_native::Element) as an overlay.
    #[allow(missing_debug_implementations)]
    pub struct FloatingElementOverlay<'a, Message: Clone, Renderer: iced_native::Renderer> {
        /// The state of the element.
        state: &'a mut Tree,
        /// The floating element
        element: Element<'a, Message, Renderer>,
        /// The optional message that will be send when the user clicks on the backdrop.
        backdrop: Option<Message>,
        /// The optional message that will be send when the ESC key was pressed.
        esc: Option<Message>,
        /// The anchor of the element.
        anchor: &'a Anchor,
        /// The offset of the element.
        offset: &'a Offset,
    }

    impl<'a, Message, Renderer> FloatingElementOverlay<'a, Message, Renderer>
    where
        Message: Clone + 'a,
        Renderer: iced_native::Renderer + 'a,
    {
        /// Creates a new [`FloatingElementOverlay`] containing the given
        /// [`Element`](iced_native::Element).
        pub fn new<B>(
            state: &'a mut Tree,
            element: B,
            backdrop: Option<Message>,
            esc: Option<Message>,
            anchor: &'a Anchor,
            offset: &'a Offset,
        ) -> Self
        where
            B: Into<Element<'a, Message, Renderer>>,
        {
            FloatingElementOverlay {
                state,
                element: element.into(),
                backdrop,
                esc,
                anchor,
                offset,
            }
        }

        /// Turns the [`FloatingElementOverlay`](FloatingElementOverlay) into an
        /// overlay [`Element`](iced_native::overlay::Element) at the given target
        /// position.
        #[must_use]
        pub fn overlay(self, position: Point) -> overlay::Element<'a, Message, Renderer> {
            overlay::Element::new(position, Box::new(self))
        }
    }

    impl<'a, Message, Renderer> iced_native::Overlay<Message, Renderer>
        for FloatingElementOverlay<'a, Message, Renderer>
    where
        Message: Clone + 'a,
        Renderer: iced_native::Renderer + 'a,
    {
        fn layout(
            &self,
            renderer: &Renderer,
            bounds: Size,
            position: Point,
        ) -> iced_native::layout::Node {
            let limits = Limits::new(Size::ZERO, bounds);
            let mut element = self.element.as_widget().layout(renderer, &limits);

            match self.anchor {
                Anchor::NorthWest => element.move_to(Point::new(
                    position.x + self.offset.x,
                    position.y + self.offset.y,
                )),
                Anchor::NorthEast => element.move_to(Point::new(
                    position.x - element.bounds().width - self.offset.x,
                    position.y + self.offset.y,
                )),
                Anchor::SouthWest => element.move_to(Point::new(
                    position.x + self.offset.x,
                    position.y - element.bounds().height - self.offset.y,
                )),
                Anchor::SouthEast => element.move_to(Point::new(
                    position.x - element.bounds().width - self.offset.x,
                    position.y - element.bounds().height - self.offset.y,
                )),
                Anchor::North => element.move_to(Point::new(
                    position.x + self.offset.x - element.bounds().width / 2.0,
                    position.y + self.offset.y,
                )),
                Anchor::East => element.move_to(Point::new(
                    position.x - element.bounds().width - self.offset.x,
                    position.y - element.bounds().height / 2.0,
                )),
                Anchor::South => element.move_to(Point::new(
                    position.x + self.offset.x - element.bounds().width / 2.0,
                    position.y - element.bounds().height - self.offset.y,
                )),
                Anchor::West => element.move_to(Point::new(
                    position.x + self.offset.x,
                    position.y - element.bounds().height / 2.0,
                )),
                Anchor::Center => {
                    // Center position
                    let max_size = limits.max();
                    let container_half_width = max_size.width / 2.0;
                    let container_half_height = max_size.height / 2.0;
                    let element_half_width = element.bounds().width / 2.0;
                    let element_half_height = element.bounds().height / 2.0;

                    let position = position
                        + Vector::new(
                            container_half_width - element_half_width,
                            container_half_height - element_half_height,
                        );

                    element.move_to(position)
                }
            }

            element
        }

        fn on_event(
            &mut self,
            event: Event,
            layout: Layout<'_>,
            cursor_position: Point,
            renderer: &Renderer,
            clipboard: &mut dyn Clipboard,
            shell: &mut Shell<Message>,
        ) -> event::Status {
            // self.element.as_widget_mut().on_event(
            //     self.state,
            //     event,
            //     layout,
            //     cursor_position,
            //     renderer,
            //     clipboard,
            //     shell,
            // )

            // TODO clean this up
            let esc_status = self
                .esc
                .as_ref()
                .map_or(event::Status::Ignored, |esc| match event {
                    Event::Keyboard(keyboard::Event::KeyPressed { key_code, .. }) => {
                        if key_code == keyboard::KeyCode::Escape {
                            shell.publish(esc.to_owned());
                            event::Status::Captured
                        } else {
                            event::Status::Ignored
                        }
                    }
                    _ => event::Status::Ignored,
                });

            let backdrop_status = self.backdrop.as_ref().zip(layout.children().next()).map_or(
                event::Status::Ignored,
                |(backdrop, layout)| match event {
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    | Event::Touch(touch::Event::FingerPressed { .. }) => {
                        if layout.bounds().contains(cursor_position) {
                            event::Status::Ignored
                        } else {
                            shell.publish(backdrop.to_owned());
                            event::Status::Captured
                        }
                    }
                    _ => event::Status::Ignored,
                },
            );

            match esc_status.merge(backdrop_status) {
                event::Status::Ignored => self.element.as_widget_mut().on_event(
                    self.state,
                    event,
                    layout,
                    cursor_position,
                    renderer,
                    clipboard,
                    shell,
                ),
                event::Status::Captured => event::Status::Captured,
            }
        }

        fn mouse_interaction(
            &self,
            layout: Layout<'_>,
            cursor_position: Point,
            viewport: &Rectangle,
            renderer: &Renderer,
        ) -> iced_native::mouse::Interaction {
            self.element.as_widget().mouse_interaction(
                self.state,
                layout,
                cursor_position,
                viewport,
                renderer,
            )
        }

        fn draw(
            &self,
            renderer: &mut Renderer,
            theme: &Renderer::Theme,
            style: &iced_native::renderer::Style,
            layout: Layout<'_>,
            cursor_position: Point,
        ) {
            self.element.as_widget().draw(
                self.state,
                renderer,
                theme,
                style,
                layout,
                cursor_position,
                &layout.bounds(),
            );
        }
    }
}
