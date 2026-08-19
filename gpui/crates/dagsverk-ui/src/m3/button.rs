use gpui::{
    BoxShadow, Context, ElementId, EventEmitter, FocusHandle, Focusable, Render, SharedString,
    Window, div, point, prelude::*, px,
};

use super::{
    DISABLED_CONTAINER_OPACITY, DISABLED_CONTENT_OPACITY, FOCUS_OPACITY, HOVER_OPACITY,
    M3ColorScheme, M3Metrics, M3TypographyExt, PRESSED_OPACITY, TypographyRole, UiScale,
    m3_icon_colored,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3ButtonVariant {
    Elevated,
    Filled,
    Tonal,
    Outlined,
    Text,
    DestructiveText,
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
    leading_icon: Option<SharedString>,
    trailing_icon: Option<SharedString>,
    full_width: bool,
    loading: bool,
    scale: UiScale,
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
            leading_icon: None,
            trailing_icon: None,
            full_width: false,
            loading: false,
            scale: UiScale::default(),
            colors,
            focus: cx.focus_handle().tab_index(0),
        }
    }

    pub fn set_leading_icon(
        &mut self,
        icon: Option<impl Into<SharedString>>,
        cx: &mut Context<Self>,
    ) {
        self.leading_icon = icon.map(Into::into);
        cx.notify();
    }

    pub fn set_trailing_icon(
        &mut self,
        icon: Option<impl Into<SharedString>>,
        cx: &mut Context<Self>,
    ) {
        self.trailing_icon = icon.map(Into::into);
        cx.notify();
    }

    pub fn set_full_width(&mut self, full_width: bool, cx: &mut Context<Self>) {
        self.full_width = full_width;
        cx.notify();
    }

    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    pub fn set_scale(&mut self, scale: UiScale, cx: &mut Context<Self>) {
        self.scale = scale;
        cx.notify();
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
            M3ButtonVariant::Elevated => (
                self.colors.surface_container_low,
                self.colors.primary,
                self.colors.surface_container_low,
            ),
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
            M3ButtonVariant::DestructiveText => (
                self.colors.background,
                self.colors.error,
                self.colors.background,
            ),
        };
        let metrics = M3Metrics::resolve(self.scale);
        let hover = m3_state_layer(background, foreground, HOVER_OPACITY);
        let pressed = m3_state_layer(background, foreground, PRESSED_OPACITY);
        let disabled_background = m3_state_layer(
            self.colors.background,
            self.colors.on_surface,
            DISABLED_CONTAINER_OPACITY,
        );
        let disabled_foreground = self.colors.on_surface.opacity(DISABLED_CONTENT_OPACITY);
        let displayed_foreground = if self.enabled {
            foreground
        } else {
            disabled_foreground
        };
        let displayed_leading = self
            .loading
            .then(|| SharedString::from("progress_activity"))
            .or_else(|| self.leading_icon.clone());

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .tab_stop(self.enabled)
            .h(metrics.control_height)
            .px(self.scale.px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .gap(self.scale.px(8.0))
            .rounded(self.scale.px(20.0))
            .border_1()
            .border_color(if self.enabled {
                border
            } else {
                disabled_background
            })
            .shadow(m3_focus_shadow(focused, self.colors.primary, self.scale))
            .bg(if self.enabled {
                background
            } else {
                disabled_background
            })
            .m3_typography(TypographyRole::LabelLarge, self.scale)
            .text_color(if self.enabled {
                foreground
            } else {
                disabled_foreground
            })
            .when(self.full_width, |button| button.w_full())
            .when(self.enabled && !self.loading, |button| {
                button
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .active(move |style| style.bg(pressed))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(M3ButtonEvent::Pressed)))
            })
            .when_some(displayed_leading, |button, icon| {
                button.child(m3_icon_colored(
                    icon,
                    18.0 * self.scale.factor(),
                    displayed_foreground,
                ))
            })
            .child(self.label.clone())
            .when_some(self.trailing_icon.clone(), |button, icon| {
                button.child(m3_icon_colored(
                    icon,
                    18.0 * self.scale.factor(),
                    displayed_foreground,
                ))
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3IconButtonEvent {
    Pressed,
}

pub struct M3IconButton {
    id: ElementId,
    icon: SharedString,
    enabled: bool,
    selected: bool,
    compact: bool,
    scale: UiScale,
    colors: M3ColorScheme,
    focus: FocusHandle,
}

impl M3IconButton {
    pub fn new(
        id: impl Into<ElementId>,
        icon: impl Into<SharedString>,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            enabled: true,
            selected: false,
            compact: false,
            scale: UiScale::default(),
            colors,
            focus: cx.focus_handle().tab_index(0),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.enabled = enabled;
        cx.notify();
    }

    pub fn set_selected(&mut self, selected: bool, cx: &mut Context<Self>) {
        self.selected = selected;
        cx.notify();
    }

    pub fn set_compact(&mut self, compact: bool, cx: &mut Context<Self>) {
        self.compact = compact;
        cx.notify();
    }

    pub fn set_scale(&mut self, scale: UiScale, cx: &mut Context<Self>) {
        self.scale = scale;
        cx.notify();
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }
}

impl EventEmitter<M3IconButtonEvent> for M3IconButton {}

impl Focusable for M3IconButton {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for M3IconButton {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let foreground = if self.selected {
            self.colors.on_secondary_container
        } else {
            self.colors.on_surface_variant
        };
        let background = if self.selected {
            self.colors.secondary_container
        } else {
            self.colors.surface
        };
        let icon_size = if self.compact { 18.0 } else { 24.0 } * self.scale.factor();
        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .tab_stop(self.enabled)
            .size(M3Metrics::resolve(self.scale).icon_button_size)
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(background)
            .shadow(m3_focus_shadow(
                self.focus.is_focused(window),
                self.colors.primary,
                self.scale,
            ))
            .text_color(foreground)
            .opacity(if self.enabled {
                1.0
            } else {
                DISABLED_CONTENT_OPACITY
            })
            .when(self.enabled, |button| {
                button
                    .cursor_pointer()
                    .hover(move |style| {
                        style.bg(m3_state_layer(background, foreground, HOVER_OPACITY))
                    })
                    .active(move |style| {
                        style.bg(m3_state_layer(background, foreground, PRESSED_OPACITY))
                    })
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(M3IconButtonEvent::Pressed)))
            })
            .child(m3_icon_colored(self.icon.clone(), icon_size, foreground))
    }
}

pub fn m3_focus_shadow(focused: bool, color: gpui::Hsla, scale: UiScale) -> Vec<BoxShadow> {
    focused
        .then(|| BoxShadow {
            color: color.opacity(FOCUS_OPACITY),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: scale.px(3.0),
        })
        .into_iter()
        .collect()
}

pub fn m3_state_layer(background: gpui::Hsla, foreground: gpui::Hsla, opacity: f32) -> gpui::Hsla {
    background.blend(foreground.opacity(opacity))
}

#[cfg(test)]
mod tests {
    use super::{UiScale, m3_focus_shadow, m3_state_layer};

    #[test]
    fn state_layer_changes_the_button_color() {
        let background: gpui::Hsla = gpui::rgb(0x5f875f).into();
        let foreground: gpui::Hsla = gpui::white();
        assert_ne!(m3_state_layer(background, foreground, 0.08), background);
        assert_ne!(
            m3_state_layer(background, foreground, 0.08),
            m3_state_layer(background, foreground, 0.12)
        );
    }

    #[test]
    fn focus_uses_a_non_layout_shadow() {
        let color: gpui::Hsla = gpui::rgb(0x5f875f).into();
        assert!(m3_focus_shadow(false, color, UiScale::default()).is_empty());
        assert_eq!(m3_focus_shadow(true, color, UiScale::default()).len(), 1);
    }
}
