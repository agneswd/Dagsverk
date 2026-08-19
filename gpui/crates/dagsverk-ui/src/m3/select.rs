use std::{cell::Cell, rc::Rc};

use gpui::{
    App, Bounds, Context, Corner, EventEmitter, FocusHandle, Focusable, KeyBinding, MouseButton,
    Pixels, Render, SharedString, Window, actions, anchored, deferred, div, point, prelude::*, px,
};

use super::{M3ColorScheme, UiScale, m3_focus_shadow, m3_icon_colored, menu_elevation};

actions!(
    m3_select,
    [
        ToggleSelect,
        CloseSelect,
        SelectNext,
        SelectPrevious,
        SelectFirst,
        SelectLast
    ]
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct M3SelectEvent(pub usize);

pub struct M3Select {
    label: SharedString,
    options: Vec<SharedString>,
    selected: usize,
    highlighted: usize,
    open: bool,
    focus: FocusHandle,
    colors: M3ColorScheme,
    label_background: gpui::Hsla,
    leading_icon: Option<SharedString>,
    scale: UiScale,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl M3Select {
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("enter", ToggleSelect, Some("M3Select")),
            KeyBinding::new("space", ToggleSelect, Some("M3Select")),
            KeyBinding::new("escape", CloseSelect, Some("M3Select")),
            KeyBinding::new("down", SelectNext, Some("M3Select")),
            KeyBinding::new("up", SelectPrevious, Some("M3Select")),
            KeyBinding::new("home", SelectFirst, Some("M3Select")),
            KeyBinding::new("end", SelectLast, Some("M3Select")),
        ]);
    }

    pub fn new(
        label: impl Into<SharedString>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
        selected: usize,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        let options: Vec<_> = options.into_iter().map(Into::into).collect();
        let selected = selected.min(options.len().saturating_sub(1));
        Self {
            label: label.into(),
            options,
            selected,
            highlighted: selected,
            open: false,
            focus: cx.focus_handle().tab_index(1).tab_stop(true),
            colors,
            label_background: colors.surface_container_low,
            leading_icon: None,
            scale: UiScale::default(),
            bounds: Rc::new(Cell::new(None)),
        }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn set_options(
        &mut self,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
        selected: usize,
        cx: &mut Context<Self>,
    ) {
        let options: Vec<_> = options.into_iter().map(Into::into).collect();
        let selected = selected.min(options.len().saturating_sub(1));
        if self.options != options || self.selected != selected {
            self.options = options;
            self.selected = selected;
            self.highlighted = selected;
            cx.notify();
        }
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        if self.colors != colors {
            if self.label_background == self.colors.surface_container_low {
                self.label_background = colors.surface_container_low;
            }
            self.colors = colors;
            cx.notify();
        }
    }

    pub fn set_label_background(&mut self, background: gpui::Hsla, cx: &mut Context<Self>) {
        if self.label_background != background {
            self.label_background = background;
            cx.notify();
        }
    }

    pub fn set_leading_icon(&mut self, icon: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.leading_icon = Some(icon.into());
        cx.notify();
    }

    pub fn set_scale(&mut self, scale: UiScale, cx: &mut Context<Self>) {
        if self.scale != scale {
            self.scale = scale;
            cx.notify();
        }
    }

    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.open != open {
            self.open = open;
            self.highlighted = self.selected;
            cx.notify();
        }
    }

    fn toggle(&mut self, _: &ToggleSelect, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus);
        if self.open {
            self.choose(self.highlighted, cx);
        } else if !self.options.is_empty() {
            self.open = true;
            self.highlighted = self.selected;
            cx.notify();
        }
    }

    fn close(&mut self, _: &CloseSelect, _: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.open = false;
            cx.notify();
        }
    }

    fn next(&mut self, _: &SelectNext, _: &mut Window, cx: &mut Context<Self>) {
        self.move_highlight(1, cx);
    }

    fn previous(&mut self, _: &SelectPrevious, _: &mut Window, cx: &mut Context<Self>) {
        self.move_highlight(-1, cx);
    }

    fn first(&mut self, _: &SelectFirst, _: &mut Window, cx: &mut Context<Self>) {
        if self.open && !self.options.is_empty() {
            self.highlighted = 0;
            cx.notify();
        }
    }

    fn last(&mut self, _: &SelectLast, _: &mut Window, cx: &mut Context<Self>) {
        if self.open && !self.options.is_empty() {
            self.highlighted = self.options.len() - 1;
            cx.notify();
        }
    }

    fn move_highlight(&mut self, direction: isize, cx: &mut Context<Self>) {
        if !self.open || self.options.is_empty() {
            return;
        }
        self.highlighted = (self.highlighted as isize + direction)
            .rem_euclid(self.options.len() as isize) as usize;
        cx.notify();
    }

    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.options.len() {
            self.selected = index;
            self.highlighted = index;
            self.open = false;
            cx.emit(M3SelectEvent(index));
            cx.notify();
        }
    }
}

impl EventEmitter<M3SelectEvent> for M3Select {}

impl Focusable for M3Select {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for M3Select {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let active = focused || self.open;
        let viewport = window.viewport_size();
        let colors = self.colors;
        let scale = self.scale;
        let last_bounds = self.bounds.get();
        let panel_position =
            last_bounds.map(|bounds| point(bounds.left(), bounds.bottom() + scale.px(8.0)));
        let panel_width = last_bounds.map_or(scale.px(368.0), |bounds| bounds.size.width);
        let value = self.options.get(self.selected).cloned().unwrap_or_default();
        let options = self
            .options
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, label)| {
                let selected = index == self.selected;
                let highlighted = index == self.highlighted;
                div()
                    .id(("m3-select-option", index))
                    .h(scale.px(48.0))
                    .px(scale.px(12.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(scale.px(8.0))
                    .cursor_pointer()
                    .bg(if highlighted {
                        colors.surface_container_highest
                    } else {
                        colors.surface_container
                    })
                    .hover(move |style| style.bg(colors.surface_container_highest))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(label)
                    .when(selected, |item| {
                        item.child(m3_icon_colored(
                            "check",
                            18.0 * scale.factor(),
                            colors.primary,
                        ))
                    })
                    .on_click(cx.listener(move |select, _, _, cx| select.choose(index, cx)))
            });

        let field = div()
            .id("m3-select")
            .key_context("M3Select")
            .relative()
            .track_focus(&self.focus)
            .tab_index(1)
            .on_action(cx.listener(Self::toggle))
            .on_action(cx.listener(Self::close))
            .on_action(cx.listener(Self::next))
            .on_action(cx.listener(Self::previous))
            .on_action(cx.listener(Self::first))
            .on_action(cx.listener(Self::last))
            .h(scale.px(56.0))
            .w_full()
            .px(scale.px(16.0))
            .flex()
            .gap(scale.px(12.0))
            .items_center()
            .justify_between()
            .rounded(scale.px(4.0))
            .border_1()
            .border_color(if active {
                colors.primary
            } else {
                colors.outline
            })
            .shadow(m3_focus_shadow(focused, colors.primary, scale))
            .cursor_pointer()
            .hover(move |style| {
                style.border_color(if active {
                    colors.primary
                } else {
                    colors.on_surface
                })
            })
            .on_click(cx.listener(|select, _, window, cx| select.toggle(&ToggleSelect, window, cx)))
            .when_some(self.leading_icon.clone(), |field, icon| {
                field.child(m3_icon_colored(
                    icon,
                    20.0 * scale.factor(),
                    colors.on_surface_variant,
                ))
            })
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .text_size(scale.px(16.0))
                    .line_height(scale.px(24.0))
                    .child(value),
            )
            .child(m3_icon_colored(
                if self.open {
                    "expand_less"
                } else {
                    "expand_more"
                },
                20.0 * scale.factor(),
                colors.on_surface_variant,
            ))
            .when(self.open, |field| {
                field
                    .child(
                        deferred(
                            anchored()
                                .position(point(px(0.0), px(0.0)))
                                .anchor(Corner::TopLeft)
                                .child(
                                    div()
                                        .id("m3-select-backdrop")
                                        .w(viewport.width)
                                        .h(viewport.height)
                                        .on_click(cx.listener(|select, _, _, cx| {
                                            select.open = false;
                                            cx.notify();
                                        })),
                                ),
                        )
                        .priority(1),
                    )
                    .child(
                        deferred(
                            anchored()
                                .position(panel_position.unwrap_or_default())
                                .anchor(Corner::TopLeft)
                                .snap_to_window_with_margin(scale.px(8.0))
                                .child(
                                    div()
                                        .id("m3-select-panel")
                                        .w(panel_width)
                                        .max_h(scale.px(384.0))
                                        .overflow_y_scroll()
                                        .p(scale.px(8.0))
                                        .flex()
                                        .flex_col()
                                        .rounded(scale.px(12.0))
                                        .bg(colors.surface_container)
                                        .shadow(menu_elevation())
                                        .children(options),
                                ),
                        )
                        .priority(2),
                    )
            });
        let bounds = self.bounds.clone();
        div()
            .w_full()
            .relative()
            .child(field)
            .child(
                div()
                    .absolute()
                    .top(scale.px(-8.0))
                    .left(scale.px(12.0))
                    .px(scale.px(4.0))
                    .bg(self.label_background)
                    .text_size(scale.px(12.0))
                    .line_height(scale.px(16.0))
                    .text_color(if active {
                        colors.primary
                    } else {
                        colors.on_surface_variant
                    })
                    .child(self.label.clone()),
            )
            .on_children_prepainted(move |children, window, _| {
                let next = children.first().copied();
                if bounds.get() != next {
                    bounds.set(next);
                    window.refresh();
                }
            })
    }
}
