use gpui::{
    App, AppContext, Context, Entity, Focusable, KeyBinding, Render, Window, actions, div,
    prelude::*, px,
};

use crate::{
    m3::{M3Button, M3ButtonVariant, M3ColorScheme, ResolvedTheme, m3_card},
    text_input::TextInput,
};

actions!(component_gallery, [Tab, TabPrevious]);

pub struct ComponentGallery {
    buttons: Vec<Entity<M3Button>>,
    input: Entity<TextInput>,
    theme: ResolvedTheme,
    activations: usize,
}

impl ComponentGallery {
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", TabPrevious, None),
        ]);
        TextInput::register_key_bindings(cx);
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_window_title("Dagsverk GPUI Preview");
        let colors = M3ColorScheme::light();
        let specs = [
            ("theme", "Toggle theme", M3ButtonVariant::Filled),
            ("filled", "Filled", M3ButtonVariant::Filled),
            ("tonal", "Tonal", M3ButtonVariant::Tonal),
            ("outlined", "Outlined", M3ButtonVariant::Outlined),
            ("text", "Text", M3ButtonVariant::Text),
            ("disabled", "Disabled", M3ButtonVariant::Filled),
        ];
        let buttons: Vec<_> = specs
            .into_iter()
            .map(|(id, label, variant)| cx.new(|cx| M3Button::new(id, label, variant, colors, cx)))
            .collect();
        buttons[5].update(cx, |button, cx| button.set_enabled(false, cx));

        for (index, button) in buttons.iter().enumerate() {
            cx.subscribe(button, move |gallery, _, _, cx| {
                if index == 0 {
                    gallery.toggle_theme(cx);
                } else {
                    gallery.activations += 1;
                    cx.notify();
                }
            })
            .detach();
        }

        let input = cx.new(|cx| TextInput::new(cx, "Text input"));
        window.focus(&input.read(cx).focus_handle(cx));
        Self {
            buttons,
            input,
            theme: ResolvedTheme::Light,
            activations: 0,
        }
    }

    fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.theme = match self.theme {
            ResolvedTheme::Light => ResolvedTheme::Dark,
            ResolvedTheme::Dark => ResolvedTheme::Light,
        };
        let colors = M3ColorScheme::resolve(self.theme);
        for button in &self.buttons {
            button.update(cx, |button, cx| button.set_colors(colors, cx));
        }
        self.input
            .update(cx, |input, cx| input.set_colors(colors, cx));
        cx.notify();
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, _: &mut Context<Self>) {
        window.focus_next();
    }

    fn tab_previous(&mut self, _: &TabPrevious, window: &mut Window, _: &mut Context<Self>) {
        window.focus_prev();
    }
}

impl Render for ComponentGallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = M3ColorScheme::resolve(self.theme);
        div()
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_previous))
            .id("component-gallery")
            .size_full()
            .overflow_y_scroll()
            .p(px(32.0))
            .bg(colors.background)
            .font_family("Roboto")
            .text_color(colors.on_surface)
            .child(
                div()
                    .max_w(px(960.0))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap(px(24.0))
                    .child(
                        div()
                            .text_size(px(24.0))
                            .line_height(px(32.0))
                            .child("Dagsverk Material 3 component gallery"),
                    )
                    .child(
                        m3_card(colors)
                            .p(px(24.0))
                            .flex()
                            .flex_col()
                            .gap(px(16.0))
                            .child("Buttons")
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap(px(12.0))
                                    .children(self.buttons.iter().cloned()),
                            )
                            .child(format!("Button activations: {}", self.activations)),
                    )
                    .child(
                        m3_card(colors)
                            .p(px(24.0))
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child("Text input")
                            .child(self.input.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.on_surface_variant)
                            .child(
                                "Tab and Shift-Tab move focus. Enter and Space activate buttons.",
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::ComponentGallery;
    use gpui::{KeyUpEvent, Keystroke, TestAppContext};

    #[gpui::test]
    fn buttons_activate_from_the_keyboard_and_disabled_buttons_do_not(cx: &mut TestAppContext) {
        cx.update(ComponentGallery::register_key_bindings);
        let (gallery, cx) = cx.add_window_view(ComponentGallery::new);
        let buttons = gallery.read_with(cx, |gallery, _| gallery.buttons.clone());

        cx.update(|window, app| window.focus(&buttons[1].read(app).focus_handle()));
        cx.refresh().expect("refresh focused button");
        cx.run_until_parked();
        cx.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("enter").expect("enter keystroke"),
        });
        assert_eq!(gallery.read_with(cx, |gallery, _| gallery.activations), 1);

        cx.update(|window, app| window.focus(&buttons[5].read(app).focus_handle()));
        cx.refresh().expect("refresh disabled button");
        cx.run_until_parked();
        cx.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").expect("space keystroke"),
        });
        assert_eq!(gallery.read_with(cx, |gallery, _| gallery.activations), 1);
    }
}
