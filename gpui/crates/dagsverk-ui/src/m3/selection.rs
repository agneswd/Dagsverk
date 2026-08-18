use gpui::{
    Context, ElementId, EventEmitter, FocusHandle, Focusable, Render, SharedString, Window, div,
    prelude::*, px,
};

use super::{M3ColorScheme, ROBOTO_FAMILY};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct M3SwitchEvent(pub bool);

pub struct M3Switch {
    id: ElementId,
    checked: bool,
    enabled: bool,
    colors: M3ColorScheme,
    focus: FocusHandle,
}

impl M3Switch {
    pub fn new(
        id: impl Into<ElementId>,
        checked: bool,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            checked,
            enabled: true,
            colors,
            focus: cx.focus_handle().tab_index(0),
        }
    }

    pub fn checked(&self) -> bool {
        self.checked
    }

    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.enabled = enabled;
        cx.notify();
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.enabled {
            self.checked = !self.checked;
            cx.emit(M3SwitchEvent(self.checked));
            cx.notify();
        }
    }
}

impl EventEmitter<M3SwitchEvent> for M3Switch {}

impl Focusable for M3Switch {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for M3Switch {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .tab_stop(self.enabled)
            .w(px(52.0))
            .h(px(32.0))
            .p(px(4.0))
            .flex()
            .items_center()
            .when_else(
                self.checked,
                |track| track.justify_end().bg(self.colors.primary),
                |track| {
                    track
                        .justify_start()
                        .bg(self.colors.surface_container_highest)
                },
            )
            .rounded(px(16.0))
            .border_2()
            .border_color(if focused || self.checked {
                self.colors.primary
            } else {
                self.colors.outline
            })
            .opacity(if self.enabled { 1.0 } else { 0.38 })
            .when(self.enabled, |track| {
                track
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.92))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
            })
            .child(
                div()
                    .size(if self.checked { px(24.0) } else { px(16.0) })
                    .rounded_full()
                    .bg(if self.checked {
                        self.colors.on_primary
                    } else {
                        self.colors.outline
                    }),
            )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct M3ChipEvent(pub bool);

pub struct M3Chip {
    id: ElementId,
    label: SharedString,
    selected: bool,
    enabled: bool,
    colors: M3ColorScheme,
    focus: FocusHandle,
}

impl M3Chip {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        selected: bool,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            selected,
            enabled: true,
            colors,
            focus: cx.focus_handle().tab_index(0),
        }
    }

    pub fn selected(&self) -> bool {
        self.selected
    }

    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.enabled = enabled;
        cx.notify();
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.enabled {
            self.selected = !self.selected;
            cx.emit(M3ChipEvent(self.selected));
            cx.notify();
        }
    }
}

impl EventEmitter<M3ChipEvent> for M3Chip {}

impl Focusable for M3Chip {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for M3Chip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .tab_stop(self.enabled)
            .h(px(32.0))
            .px(px(16.0))
            .flex()
            .items_center()
            .rounded(px(8.0))
            .border_2()
            .border_color(if focused {
                self.colors.primary
            } else {
                self.colors.outline
            })
            .bg(if self.selected {
                self.colors.secondary_container
            } else {
                self.colors.background
            })
            .font_family(ROBOTO_FAMILY)
            .text_size(px(14.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(if self.selected {
                self.colors.on_secondary_container
            } else {
                self.colors.on_surface_variant
            })
            .opacity(if self.enabled { 1.0 } else { 0.38 })
            .when(self.enabled, |chip| {
                chip.cursor_pointer()
                    .hover(|style| style.opacity(0.92))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
            })
            .child(self.label.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3Status {
    Neutral,
    Success,
    Warning,
    Error,
}

pub fn m3_status_chip(
    label: impl Into<SharedString>,
    status: M3Status,
    colors: M3ColorScheme,
) -> gpui::Div {
    let (background, foreground) = match status {
        M3Status::Neutral => (colors.surface_container_high, colors.on_surface_variant),
        M3Status::Success => (colors.success_container, colors.on_success_container),
        M3Status::Warning => (colors.warning_container, colors.on_warning_container),
        M3Status::Error => (colors.error_container, colors.on_error_container),
    };
    div()
        .h(px(26.0))
        .px(px(10.0))
        .flex()
        .items_center()
        .rounded(px(13.0))
        .bg(background)
        .font_family(ROBOTO_FAMILY)
        .text_size(px(12.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(foreground)
        .child(label.into())
}
