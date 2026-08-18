use gpui::{
    Context, ElementId, EventEmitter, FocusHandle, Render, SharedString, Window, div, prelude::*,
    px, relative,
};

use super::{M3ColorScheme, ROBOTO_FAMILY, m3_icon};

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
        }
    }

    pub fn selected(&self) -> usize {
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

    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.enabled && index < self.items.len() && self.selected != index {
            self.selected = index;
            cx.emit(M3ChoiceEvent(index));
            cx.notify();
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
        let items = self
            .items
            .iter()
            .cloned()
            .zip(self.focus.iter().cloned())
            .enumerate()
            .map(|(index, (label, focus))| {
                let is_selected = index == selected;
                let is_focused = focus.is_focused(window);
                div()
                    .id((id.clone(), index))
                    .track_focus(&focus)
                    .tab_index(index as isize)
                    .tab_stop(enabled)
                    .h(px(40.0))
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(kind == M3ChoiceKind::Segmented, |item| {
                        item.rounded(px(20.0))
                            .border_1()
                            .border_color(if is_focused {
                                colors.primary
                            } else {
                                colors.outline
                            })
                    })
                    .when(kind == M3ChoiceKind::Tabs, |item| {
                        item.border_b_2()
                            .border_color(if is_selected || is_focused {
                                colors.primary
                            } else {
                                colors.background
                            })
                    })
                    .bg(if is_selected {
                        colors.secondary_container
                    } else {
                        colors.background
                    })
                    .font_family(ROBOTO_FAMILY)
                    .text_size(px(14.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(if is_selected {
                        colors.on_secondary_container
                    } else {
                        colors.on_surface_variant
                    })
                    .opacity(if enabled { 1.0 } else { 0.38 })
                    .when(enabled, |item| {
                        item.cursor_pointer()
                            .hover(|style| style.opacity(0.92))
                            .on_click(cx.listener(move |this, _, _, cx| this.select(index, cx)))
                    })
                    .child(label)
            })
            .collect::<Vec<_>>();

        div().flex().items_center().children(items)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct M3ExpansionPanelEvent(pub bool);

pub struct M3ExpansionPanel {
    id: ElementId,
    title: SharedString,
    body: SharedString,
    expanded: bool,
    colors: M3ColorScheme,
    focus: FocusHandle,
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
            body: body.into(),
            expanded: false,
            colors,
            focus: cx.focus_handle().tab_index(0),
        }
    }

    pub fn expanded(&self) -> bool {
        self.expanded
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        cx.emit(M3ExpansionPanelEvent(self.expanded));
        cx.notify();
    }
}

impl EventEmitter<M3ExpansionPanelEvent> for M3ExpansionPanel {}

impl Render for M3ExpansionPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        div()
            .w_full()
            .rounded(px(12.0))
            .border_1()
            .border_color(self.colors.outline_variant)
            .bg(self.colors.surface_container_lowest)
            .child(
                div()
                    .id(self.id.clone())
                    .track_focus(&self.focus)
                    .tab_index(0)
                    .h(px(48.0))
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_2()
                    .border_color(if focused {
                        self.colors.primary
                    } else {
                        self.colors.surface_container_lowest
                    })
                    .rounded(px(12.0))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
                    .child(self.title.clone())
                    .child(m3_icon("expand_more", 20.0, self.colors)),
            )
            .when(self.expanded, |panel| {
                panel.child(
                    div()
                        .px(px(16.0))
                        .pb(px(16.0))
                        .text_color(self.colors.on_surface_variant)
                        .child(self.body.clone()),
                )
            })
    }
}
