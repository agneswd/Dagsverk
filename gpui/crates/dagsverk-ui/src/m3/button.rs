use gpui::{
    Context, ElementId, EventEmitter, FocusHandle, Focusable, Render, SharedString, Window, div,
    prelude::*, px,
};

use super::{M3ColorScheme, ROBOTO_FAMILY};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3ButtonVariant {
    Filled,
    Tonal,
    Outlined,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3ButtonEvent {
    Pressed,
}

pub struct M3Button {
    id: ElementId,
    label: SharedString,
    variant: M3ButtonVariant,
    enabled: bool,
    colors: M3ColorScheme,
    focus: FocusHandle,
}

impl M3Button {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        variant: M3ButtonVariant,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant,
            enabled: true,
            colors,
            focus: cx.focus_handle().tab_index(0),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.enabled != enabled {
            self.enabled = enabled;
            cx.notify();
        }
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        if self.colors != colors {
            self.colors = colors;
            cx.notify();
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }
}

impl EventEmitter<M3ButtonEvent> for M3Button {}

impl Focusable for M3Button {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for M3Button {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let (background, foreground, border) = match self.variant {
            M3ButtonVariant::Filled => (
                self.colors.primary,
                self.colors.on_primary,
                self.colors.primary,
            ),
            M3ButtonVariant::Tonal => (
                self.colors.secondary_container,
                self.colors.on_secondary_container,
                self.colors.secondary_container,
            ),
            M3ButtonVariant::Outlined => (
                self.colors.background,
                self.colors.primary,
                self.colors.outline,
            ),
            M3ButtonVariant::Text => (
                self.colors.background,
                self.colors.primary,
                self.colors.background,
            ),
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .tab_stop(self.enabled)
            .h(px(40.0))
            .px(px(24.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(20.0))
            .border_2()
            .border_color(if focused { self.colors.primary } else { border })
            .bg(background)
            .font_family(ROBOTO_FAMILY)
            .text_size(px(14.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .line_height(px(20.0))
            .text_color(foreground)
            .opacity(if self.enabled { 1.0 } else { 0.38 })
            .when(self.enabled, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.92))
                    .active(|style| style.opacity(0.84))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(M3ButtonEvent::Pressed)))
            })
            .child(self.label.clone())
    }
}
