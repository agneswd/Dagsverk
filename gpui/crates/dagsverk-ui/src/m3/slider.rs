use std::{cell::Cell, rc::Rc};

use gpui::{
    Bounds, BoxShadow, Context, EventEmitter, FocusHandle, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Render, Window, div, linear_color_stop,
    linear_gradient, point, prelude::*, px, relative,
};

use super::{FOCUS_OPACITY, M3ColorScheme, UiScale};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct M3SliderEvent(pub i32);

pub struct M3Slider {
    focus: FocusHandle,
    value: i32,
    min: i32,
    max: i32,
    colors: M3ColorScheme,
    track_colors: Vec<gpui::Hsla>,
    scale: UiScale,
    track_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    dragging: bool,
}

impl M3Slider {
    pub fn new(
        value: i32,
        min: i32,
        max: i32,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        assert!(min < max, "slider minimum must be lower than maximum");
        Self {
            focus: cx.focus_handle().tab_index(1).tab_stop(true),
            value: value.clamp(min, max),
            min,
            max,
            colors,
            track_colors: vec![colors.surface_container_highest, colors.primary],
            scale: UiScale::default(),
            track_bounds: Rc::new(Cell::new(None)),
            dragging: false,
        }
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub fn set_value(&mut self, value: i32, cx: &mut Context<Self>) {
        let value = value.clamp(self.min, self.max);
        if self.value != value {
            self.value = value;
            cx.notify();
        }
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        if self.colors != colors {
            self.colors = colors;
            cx.notify();
        }
    }

    pub fn set_scale(&mut self, scale: UiScale, cx: &mut Context<Self>) {
        if self.scale != scale {
            self.scale = scale;
            cx.notify();
        }
    }

    pub fn set_track_colors(
        &mut self,
        colors: impl IntoIterator<Item = gpui::Hsla>,
        cx: &mut Context<Self>,
    ) {
        let colors = colors.into_iter().collect::<Vec<_>>();
        if colors.len() >= 2 && self.track_colors != colors {
            self.track_colors = colors;
            cx.notify();
        }
    }

    fn set_value_and_emit(&mut self, value: i32, cx: &mut Context<Self>) {
        let value = value.clamp(self.min, self.max);
        if self.value != value {
            self.value = value;
            cx.emit(M3SliderEvent(value));
            cx.notify();
        }
    }

    fn update_from_position(&mut self, position: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(bounds) = self.track_bounds.get() else {
            return;
        };
        let ratio = ((position.x - bounds.left()) / bounds.size.width).clamp(0.0, 1.0);
        let value = self.min as f32 + ratio * (self.max - self.min) as f32;
        self.set_value_and_emit(value.round() as i32, cx);
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dragging = true;
        window.focus(&self.focus);
        self.update_from_position(event.position, cx);
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.dragging {
            self.update_from_position(event.position, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.dragging = false;
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let value = match event.keystroke.key.as_str() {
            "left" | "down" => self.value - 1,
            "right" | "up" => self.value + 1,
            "home" => self.min,
            "end" => self.max,
            _ => return,
        };
        self.set_value_and_emit(value, cx);
    }
}

impl EventEmitter<M3SliderEvent> for M3Slider {}

impl Render for M3Slider {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ratio = (self.value - self.min) as f32 / (self.max - self.min) as f32;
        let focused = self.focus.is_focused(window);
        let scale = self.scale;
        let track_bounds = self.track_bounds.clone();
        let track = div()
            .w_full()
            .h(scale.px(8.0))
            .flex()
            .overflow_hidden()
            .rounded(scale.px(4.0))
            .children(self.track_colors.windows(2).map(|pair| {
                div().flex_1().h_full().bg(linear_gradient(
                    90.0,
                    linear_color_stop(pair[0], 0.0),
                    linear_color_stop(pair[1], 1.0),
                ))
            }));
        let knob_shadow = focused
            .then(|| BoxShadow {
                color: self.colors.primary.opacity(FOCUS_OPACITY),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(0.0),
                spread_radius: scale.px(3.0),
            })
            .into_iter()
            .collect::<Vec<_>>();

        div()
            .track_focus(&self.focus)
            .tab_index(0)
            .tab_stop(true)
            .relative()
            .w_full()
            .h(scale.px(20.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_key_down(cx.listener(Self::on_key_down))
            .child(track)
            .child(
                div()
                    .absolute()
                    .left(relative(ratio))
                    .ml(-scale.px(8.0))
                    .size(scale.px(16.0))
                    .rounded_full()
                    .border_2()
                    .border_color(self.colors.on_surface)
                    .bg(self.colors.surface_container_highest)
                    .shadow(knob_shadow),
            )
            .on_children_prepainted(move |bounds, _, _| {
                track_bounds.set(bounds.first().copied());
            })
    }
}
