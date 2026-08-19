use gpui::{
    App, Context, ElementId, EventEmitter, FocusHandle, KeyBinding, MouseButton, Render,
    SharedString, Window, actions, div, prelude::*,
};

use super::{M3ColorScheme, M3TypographyExt, ROBOTO_FAMILY, TypographyRole, UiScale};

actions!(m3_dialog, [DismissDialog, CycleDialogFocus]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3DialogEvent {
    Dismissed,
}

pub struct M3Dialog {
    id: ElementId,
    title: SharedString,
    message: SharedString,
    open: bool,
    needs_focus: bool,
    colors: M3ColorScheme,
    close_focus: FocusHandle,
    scale: UiScale,
}

impl M3Dialog {
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("escape", DismissDialog, Some("M3Dialog")),
            KeyBinding::new("tab", CycleDialogFocus, Some("M3Dialog")),
            KeyBinding::new("shift-tab", CycleDialogFocus, Some("M3Dialog")),
        ]);
    }

    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            message: message.into(),
            open: false,
            needs_focus: false,
            colors,
            close_focus: cx.focus_handle().tab_index(0),
            scale: UiScale::default(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.close_focus.clone()
    }

    pub fn open(&mut self, cx: &mut Context<Self>) {
        self.open = true;
        self.needs_focus = true;
        cx.notify();
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    pub fn set_scale(&mut self, scale: UiScale, cx: &mut Context<Self>) {
        self.scale = scale;
        cx.notify();
    }

    fn dismiss(&mut self, _: &DismissDialog, _: &mut Window, cx: &mut Context<Self>) {
        self.close(cx);
    }

    fn cycle_focus(&mut self, _: &CycleDialogFocus, window: &mut Window, _: &mut Context<Self>) {
        window.focus(&self.close_focus);
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        if self.open {
            self.open = false;
            cx.emit(M3DialogEvent::Dismissed);
            cx.notify();
        }
    }
}

impl EventEmitter<M3DialogEvent> for M3Dialog {}

impl Render for M3Dialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.open && self.needs_focus {
            window.focus(&self.close_focus);
            self.needs_focus = false;
        }
        let scale = self.scale;

        div().size_full().when(self.open, |root| {
            root.child(
                div()
                    .id(self.id.clone())
                    .key_context("M3Dialog")
                    .on_action(cx.listener(Self::dismiss))
                    .on_action(cx.listener(Self::cycle_focus))
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(gpui::black().opacity(0.48))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.close(cx)),
                    )
                    .child(
                        div()
                            .w(scale.px(480.0))
                            .p(scale.px(24.0))
                            .flex()
                            .flex_col()
                            .gap(scale.px(16.0))
                            .rounded(scale.px(28.0))
                            .bg(self.colors.surface_container_high)
                            .font_family(ROBOTO_FAMILY)
                            .text_color(self.colors.on_surface)
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                div()
                                    .m3_typography(TypographyRole::HeadlineSmall, scale)
                                    .child(self.title.clone()),
                            )
                            .child(
                                div()
                                    .m3_typography(TypographyRole::BodyMedium, scale)
                                    .text_color(self.colors.on_surface_variant)
                                    .child(self.message.clone()),
                            )
                            .child(
                                div()
                                    .id("m3-dialog-close")
                                    .track_focus(&self.close_focus)
                                    .tab_index(0)
                                    .h(scale.px(40.0))
                                    .px(scale.px(24.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .ml_auto()
                                    .rounded(scale.px(20.0))
                                    .bg(self.colors.primary)
                                    .text_color(self.colors.on_primary)
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| this.close(cx)))
                                    .child("Close"),
                            ),
                    ),
            )
        })
    }
}
