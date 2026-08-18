use gpui::{
    App, Context, Entity, FocusHandle, Focusable, KeyBinding, MouseButton, Render, Window, actions,
    div, prelude::*, px,
};

use crate::text_input::TextInput;

actions!(preview, [Tab, TabPrevious, Activate]);

pub struct Preview {
    input: Entity<TextInput>,
    button_focus: FocusHandle,
    activations: usize,
}

impl Preview {
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", TabPrevious, None),
            KeyBinding::new("enter", Activate, Some("PreviewButton")),
            KeyBinding::new("space", Activate, Some("PreviewButton")),
        ]);
        TextInput::register_key_bindings(cx);
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_window_title("Dagsverk GPUI Preview");
        let input = cx.new(|cx| TextInput::new(cx, "Type to test the GPUI input"));
        window.focus(&input.read(cx).focus_handle(cx));
        Self {
            input,
            button_focus: cx.focus_handle().tab_index(2).tab_stop(true),
            activations: 0,
        }
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, _: &mut Context<Self>) {
        window.focus_next();
    }

    fn tab_previous(&mut self, _: &TabPrevious, window: &mut Window, _: &mut Context<Self>) {
        window.focus_prev();
    }

    fn activate(&mut self, _: &Activate, _: &mut Window, cx: &mut Context<Self>) {
        self.activations += 1;
        cx.notify();
    }
}

impl Render for Preview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let button_focused = self.button_focus.is_focused(window);
        div()
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_previous))
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .bg(gpui::rgb(0xf0f4f9))
            .font_family("Roboto")
            .text_color(gpui::rgb(0x1f1f1f))
            .child(
                div()
                    .w(px(480.))
                    .p(px(32.))
                    .flex()
                    .flex_col()
                    .gap(px(20.))
                    .rounded(px(20.))
                    .bg(gpui::rgb(0xffffff))
                    .shadow_lg()
                    .child(div().text_size(px(28.)).child("Dagsverk GPUI Preview"))
                    .child(
                        div()
                            .text_size(px(14.))
                            .child("Input, selection, clipboard, focus, and resize platform check"),
                    )
                    .child(self.input.clone())
                    .child(
                        div()
                            .id("preview-button")
                            .key_context("PreviewButton")
                            .track_focus(&self.button_focus)
                            .on_action(cx.listener(Self::activate))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    window.focus(&this.button_focus);
                                    this.activations += 1;
                                    cx.notify();
                                }),
                            )
                            .h(px(48.))
                            .px(px(24.))
                            .flex()
                            .justify_center()
                            .items_center()
                            .rounded(px(24.))
                            .bg(gpui::rgb(0x5f875f))
                            .text_color(gpui::rgb(0xffffff))
                            .border_2()
                            .border_color(if button_focused {
                                gpui::rgb(0x19351c)
                            } else {
                                gpui::rgb(0x5f875f)
                            })
                            .hover(|style| style.bg(gpui::rgb(0x4f774f)).cursor_pointer())
                            .child(format!("Test button ({})", self.activations)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(gpui::rgb(0x444746))
                            .child("Press Tab or Shift-Tab to move focus."),
                    ),
            )
    }
}
