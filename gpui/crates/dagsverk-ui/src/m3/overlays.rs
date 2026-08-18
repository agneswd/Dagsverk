use gpui::{
    App, Context, EventEmitter, FocusHandle, KeyBinding, MouseButton, Render, SharedString, Window,
    actions, div, prelude::*, px,
};

use super::{M3ColorScheme, ROBOTO_FAMILY};

actions!(m3_menu, [DismissMenu, MenuNext, MenuPrevious]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3MenuEvent {
    Selected(usize),
    Dismissed,
}

pub struct M3Menu {
    items: Vec<SharedString>,
    open: bool,
    focused: usize,
    needs_focus: bool,
    colors: M3ColorScheme,
    focus: Vec<FocusHandle>,
}

impl M3Menu {
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("escape", DismissMenu, Some("M3Menu")),
            KeyBinding::new("down", MenuNext, Some("M3Menu")),
            KeyBinding::new("up", MenuPrevious, Some("M3Menu")),
        ]);
    }

    pub fn new(
        items: impl IntoIterator<Item = impl Into<SharedString>>,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        let items: Vec<_> = items.into_iter().map(Into::into).collect();
        let focus = (0..items.len())
            .map(|index| cx.focus_handle().tab_index(index as isize))
            .collect();
        Self {
            items,
            open: false,
            focused: 0,
            needs_focus: false,
            colors,
            focus,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, cx: &mut Context<Self>) {
        if !self.items.is_empty() {
            self.open = true;
            self.focused = 0;
            self.needs_focus = true;
            cx.notify();
        }
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    fn dismiss(&mut self, _: &DismissMenu, _: &mut Window, cx: &mut Context<Self>) {
        self.close(cx, true);
    }

    fn next(&mut self, _: &MenuNext, window: &mut Window, cx: &mut Context<Self>) {
        self.move_focus(1, window, cx);
    }

    fn previous(&mut self, _: &MenuPrevious, window: &mut Window, cx: &mut Context<Self>) {
        self.move_focus(-1, window, cx);
    }

    fn move_focus(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        self.focused =
            (self.focused as isize + direction).rem_euclid(self.items.len() as isize) as usize;
        window.focus(&self.focus[self.focused]);
        cx.notify();
    }

    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.open && index < self.items.len() {
            self.open = false;
            cx.emit(M3MenuEvent::Selected(index));
            cx.notify();
        }
    }

    fn close(&mut self, cx: &mut Context<Self>, emit: bool) {
        if self.open {
            self.open = false;
            if emit {
                cx.emit(M3MenuEvent::Dismissed);
            }
            cx.notify();
        }
    }
}

impl EventEmitter<M3MenuEvent> for M3Menu {}

impl Render for M3Menu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.open && self.needs_focus {
            window.focus(&self.focus[self.focused]);
            self.needs_focus = false;
        }

        let colors = self.colors;
        let items = self
            .items
            .iter()
            .cloned()
            .zip(self.focus.iter().cloned())
            .enumerate()
            .map(|(index, (label, focus))| {
                let focused = focus.is_focused(window);
                div()
                    .id(("m3-menu-item", index))
                    .track_focus(&focus)
                    .tab_index(index as isize)
                    .h(px(48.0))
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .rounded(px(8.0))
                    .border_2()
                    .border_color(if focused {
                        colors.primary
                    } else {
                        colors.surface_container_high
                    })
                    .hover(|style| style.bg(colors.surface_container_highest))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.select(index, cx)))
                    .child(label)
            })
            .collect::<Vec<_>>();

        div().size_full().when(self.open, |root| {
            root.child(
                div()
                    .id("m3-menu-backdrop")
                    .key_context("M3Menu")
                    .on_action(cx.listener(Self::dismiss))
                    .on_action(cx.listener(Self::next))
                    .on_action(cx.listener(Self::previous))
                    .absolute()
                    .inset_0()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.close(cx, true)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(88.0))
                            .right(px(48.0))
                            .w(px(240.0))
                            .p(px(8.0))
                            .flex()
                            .flex_col()
                            .rounded(px(16.0))
                            .bg(colors.surface_container_high)
                            .font_family(ROBOTO_FAMILY)
                            .text_color(colors.on_surface)
                            .shadow_lg()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .children(items),
                    ),
            )
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3SnackbarEvent {
    Dismissed,
}

pub struct M3SnackbarHost {
    message: Option<SharedString>,
    colors: M3ColorScheme,
    dismiss_focus: FocusHandle,
}

impl M3SnackbarHost {
    pub fn new(colors: M3ColorScheme, cx: &mut Context<Self>) -> Self {
        Self {
            message: None,
            colors,
            dismiss_focus: cx.focus_handle().tab_index(0),
        }
    }

    pub fn show(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.message = Some(message.into());
        cx.notify();
    }

    pub fn is_visible(&self) -> bool {
        self.message.is_some()
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    fn dismiss(&mut self, cx: &mut Context<Self>) {
        if self.message.take().is_some() {
            cx.emit(M3SnackbarEvent::Dismissed);
            cx.notify();
        }
    }
}

impl EventEmitter<M3SnackbarEvent> for M3SnackbarHost {}

impl Render for M3SnackbarHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dismiss_focused = self.dismiss_focus.is_focused(window);
        div()
            .size_full()
            .when_some(self.message.clone(), |root, message| {
                root.child(
                    div()
                        .absolute()
                        .left(px(24.0))
                        .bottom(px(24.0))
                        .min_w(px(320.0))
                        .h(px(48.0))
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .rounded(px(4.0))
                        .bg(self.colors.surface_container_highest)
                        .font_family(ROBOTO_FAMILY)
                        .text_color(self.colors.on_surface)
                        .shadow_lg()
                        .child(message)
                        .child(
                            div()
                                .id("m3-snackbar-dismiss")
                                .track_focus(&self.dismiss_focus)
                                .tab_index(0)
                                .px(px(8.0))
                                .border_2()
                                .border_color(if dismiss_focused {
                                    self.colors.primary
                                } else {
                                    self.colors.surface_container_highest
                                })
                                .rounded(px(8.0))
                                .text_color(self.colors.primary)
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| this.dismiss(cx)))
                                .child("Dismiss"),
                        ),
                )
            })
    }
}
