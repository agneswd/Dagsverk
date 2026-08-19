use gpui::{
    BoxShadow, Context, ElementId, EventEmitter, FocusHandle, KeyDownEvent, Render, SharedString,
    Window, div, point, prelude::*, px, relative,
};

use super::{
    FOCUS_OPACITY, HOVER_OPACITY, M3ColorScheme, M3TypographyExt, PRESSED_OPACITY, TypographyRole,
    UiScale, m3_icon, m3_state_layer,
};

pub fn m3_divider(colors: M3ColorScheme) -> gpui::Div {
    div().w_full().h(px(1.0)).bg(colors.outline_variant)
}

pub fn m3_progress_bar(value: f32, colors: M3ColorScheme) -> gpui::Div {
    div()
        .w_full()
        .h(px(4.0))
        .rounded_full()
        .bg(colors.surface_container_highest)
        .child(
            div()
                .h_full()
                .w(relative(value.clamp(0.0, 1.0)))
                .rounded_full()
                .bg(colors.primary),
        )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3ChoiceKind {
    Tabs,
    Segmented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct M3ChoiceEvent(pub usize);

pub struct M3ChoiceGroup {
    id: SharedString,
    items: Vec<SharedString>,
    selected: usize,
    enabled: bool,
    kind: M3ChoiceKind,
    colors: M3ColorScheme,
    focus: Vec<FocusHandle>,
    scale: UiScale,
}

impl M3ChoiceGroup {
    pub fn new(
        id: impl Into<SharedString>,
        items: impl IntoIterator<Item = impl Into<SharedString>>,
        selected: usize,
        kind: M3ChoiceKind,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        let items: Vec<_> = items.into_iter().map(Into::into).collect();
        let selected = selected.min(items.len().saturating_sub(1));
        let focus = (0..items.len())
            .map(|index| cx.focus_handle().tab_index(index as isize))
            .collect();
        Self {
            id: id.into(),
            items,
            selected,
            enabled: true,
            kind,
            colors,
            focus,
            scale: UiScale::default(),
        }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn focus_handle(&self, index: usize) -> Option<FocusHandle> {
        self.focus.get(index).cloned()
    }

    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.enabled = enabled;
        cx.notify();
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

    pub fn set_selected(&mut self, selected: usize, cx: &mut Context<Self>) {
        let selected = selected.min(self.items.len().saturating_sub(1));
        if self.selected != selected {
            self.selected = selected;
            cx.notify();
        }
    }

    pub fn set_labels(
        &mut self,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
        cx: &mut Context<Self>,
    ) {
        let labels: Vec<_> = labels.into_iter().map(Into::into).collect();
        if labels.len() == self.items.len() && labels != self.items {
            self.items = labels;
            cx.notify();
        }
    }

    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.enabled && index < self.items.len() && self.selected != index {
            self.selected = index;
            cx.emit(M3ChoiceEvent(index));
            cx.notify();
        }
    }

    fn navigate(
        &mut self,
        current: usize,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let last = self.items.len().saturating_sub(1);
        let next = match event.keystroke.key.as_str() {
            "left" => current.checked_sub(1).unwrap_or(last),
            "right" => (current + 1) % self.items.len().max(1),
            "home" => 0,
            "end" => last,
            _ => return,
        };
        self.select(next, cx);
        if let Some(focus) = self.focus.get(next) {
            window.focus(focus);
        }
    }
}

impl EventEmitter<M3ChoiceEvent> for M3ChoiceGroup {}

impl Render for M3ChoiceGroup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = self.enabled;
        let selected = self.selected;
        let colors = self.colors;
        let kind = self.kind;
        let id = self.id.clone();
        let scale = self.scale;
        let items = self
            .items
            .iter()
            .cloned()
            .zip(self.focus.iter().cloned())
            .enumerate()
            .map(|(index, (label, focus))| {
                let is_selected = index == selected;
                let is_focused = focus.is_focused(window);
                let background = if kind == M3ChoiceKind::Tabs {
                    colors.surface_container_low
                } else if is_selected {
                    colors.secondary_container
                } else {
                    colors.surface
                };
                let foreground = if kind == M3ChoiceKind::Tabs && is_selected {
                    colors.primary
                } else if is_selected {
                    colors.on_secondary_container
                } else {
                    colors.on_surface_variant
                };
                div()
                    .id((id.clone(), index))
                    .track_focus(&focus)
                    .tab_index(index as isize)
                    .tab_stop(enabled)
                    .h_full()
                    .px(scale.px(16.0))
                    .flex()
                    .when(kind == M3ChoiceKind::Tabs, |item| item.flex_1())
                    .items_center()
                    .justify_center()
                    .when(kind == M3ChoiceKind::Segmented && index > 0, |item| {
                        item.border_l_1().border_color(colors.outline_variant)
                    })
                    .when(kind == M3ChoiceKind::Tabs, |item| {
                        item.border_b_2().border_color(if is_selected {
                            colors.primary
                        } else {
                            colors.surface_container_low
                        })
                    })
                    .bg(background)
                    .shadow(choice_focus_shadow(is_focused, colors.primary, scale))
                    .m3_typography(TypographyRole::LabelLarge, scale)
                    .text_color(foreground)
                    .opacity(if enabled { 1.0 } else { 0.38 })
                    .when(enabled, |item| {
                        item.cursor_pointer()
                            .hover(move |style| {
                                style.bg(m3_state_layer(background, foreground, HOVER_OPACITY))
                            })
                            .active(move |style| {
                                style.bg(m3_state_layer(background, foreground, PRESSED_OPACITY))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| this.select(index, cx)))
                            .on_key_down(cx.listener(move |this, event, window, cx| {
                                this.navigate(index, event, window, cx)
                            }))
                    })
                    .child(label)
            })
            .collect::<Vec<_>>();

        div()
            .h(scale.px(40.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .when(kind == M3ChoiceKind::Segmented, |group| {
                group
                    .rounded(scale.px(20.0))
                    .border_1()
                    .border_color(colors.outline_variant)
            })
            .when(kind == M3ChoiceKind::Tabs, |group| {
                group
                    .w_full()
                    .rounded_t(scale.px(16.0))
                    .border_b_1()
                    .border_color(colors.grid_line)
                    .bg(colors.surface_container_low)
            })
            .children(items)
    }
}

fn choice_focus_shadow(focused: bool, color: gpui::Hsla, scale: UiScale) -> Vec<BoxShadow> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct M3ExpansionPanelEvent(pub bool);

pub struct M3ExpansionPanel {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    body: SharedString,
    expanded: bool,
    header_only: bool,
    colors: M3ColorScheme,
    focus: FocusHandle,
    scale: UiScale,
}

impl M3ExpansionPanel {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            body: body.into(),
            expanded: false,
            header_only: false,
            colors,
            focus: cx.focus_handle().tab_index(0),
            scale: UiScale::default(),
        }
    }

    pub fn expanded(&self) -> bool {
        self.expanded
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    pub fn set_scale(&mut self, scale: UiScale, cx: &mut Context<Self>) {
        self.scale = scale;
        cx.notify();
    }

    pub fn set_title(&mut self, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        let title = title.into();
        if self.title != title {
            self.title = title;
            cx.notify();
        }
    }

    pub fn set_description(
        &mut self,
        description: Option<impl Into<SharedString>>,
        cx: &mut Context<Self>,
    ) {
        let description = description.map(Into::into);
        if self.description != description {
            self.description = description;
            cx.notify();
        }
    }

    pub fn set_expanded(&mut self, expanded: bool, cx: &mut Context<Self>) {
        if self.expanded != expanded {
            self.expanded = expanded;
            cx.notify();
        }
    }

    pub fn set_header_only(&mut self, header_only: bool, cx: &mut Context<Self>) {
        if self.header_only != header_only {
            self.header_only = header_only;
            cx.notify();
        }
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        cx.emit(M3ExpansionPanelEvent(self.expanded));
        cx.notify();
    }
}

impl EventEmitter<M3ExpansionPanelEvent> for M3ExpansionPanel {}

impl gpui::Focusable for M3ExpansionPanel {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for M3ExpansionPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let scale = self.scale;
        let icon = if self.expanded {
            "expand_less"
        } else {
            "expand_more"
        };
        let header_background = if self.header_only {
            self.colors.surface_container
        } else {
            self.colors.surface_container_lowest
        };
        let header_foreground = self.colors.on_surface;
        div()
            .w_full()
            .rounded(scale.px(12.0))
            .when(!self.header_only, |panel| {
                panel
                    .border_1()
                    .border_color(self.colors.outline_variant)
                    .bg(self.colors.surface_container_lowest)
            })
            .child(
                div()
                    .id(self.id.clone())
                    .track_focus(&self.focus)
                    .tab_index(0)
                    .tab_stop(true)
                    .h(scale.px(48.0))
                    .px(scale.px(16.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .shadow(choice_focus_shadow(focused, self.colors.primary, scale))
                    .rounded(scale.px(12.0))
                    .cursor_pointer()
                    .hover(move |style| {
                        style.bg(m3_state_layer(
                            header_background,
                            header_foreground,
                            HOVER_OPACITY,
                        ))
                    })
                    .active(move |style| {
                        style.bg(m3_state_layer(
                            header_background,
                            header_foreground,
                            PRESSED_OPACITY,
                        ))
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.toggle(cx);
                        }
                    }))
                    .child(
                        div()
                            .min_w(px(0.0))
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .m3_typography(TypographyRole::TitleSmall, scale)
                                    .text_color(self.colors.on_surface)
                                    .child(self.title.clone()),
                            )
                            .when_some(self.description.clone(), |header, description| {
                                header.child(
                                    div()
                                        .m3_typography(TypographyRole::BodySmall, scale)
                                        .text_color(self.colors.on_surface_variant)
                                        .child(description),
                                )
                            }),
                    )
                    .child(m3_icon(icon, 20.0 * scale.factor(), self.colors)),
            )
            .when(self.expanded && !self.header_only, |panel| {
                panel.child(
                    div()
                        .px(scale.px(16.0))
                        .pb(scale.px(16.0))
                        .text_color(self.colors.on_surface_variant)
                        .child(self.body.clone()),
                )
            })
    }
}
