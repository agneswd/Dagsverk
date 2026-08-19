use gpui::{
    App, AppContext, Context, Entity, Focusable, KeyBinding, Render, Window, actions, div,
    prelude::*,
};

use crate::{
    m3::{
        M3Button, M3ButtonVariant, M3Chip, M3ChoiceGroup, M3ChoiceKind, M3ColorScheme, M3Dialog,
        M3ExpansionPanel, M3IconButton, M3Menu, M3Select, M3SnackbarHost, M3Status, M3Switch,
        ResolvedTheme, UiScale, m3_card, m3_divider, m3_icon, m3_progress_bar, m3_status_chip,
    },
    text_input::{TextInput, TextInputEvent},
};

actions!(component_gallery, [Tab, TabPrevious]);

pub struct ComponentGallery {
    buttons: Vec<Entity<M3Button>>,
    icon_button: Entity<M3IconButton>,
    input: Entity<TextInput>,
    textarea: Entity<TextInput>,
    select: Entity<M3Select>,
    switch: Entity<M3Switch>,
    chips: Vec<Entity<M3Chip>>,
    tabs: Entity<M3ChoiceGroup>,
    segmented: Entity<M3ChoiceGroup>,
    expansion: Entity<M3ExpansionPanel>,
    dialog: Entity<M3Dialog>,
    menu: Entity<M3Menu>,
    snackbar: Entity<M3SnackbarHost>,
    theme: ResolvedTheme,
    activations: usize,
    input_changes: usize,
    scale: UiScale,
}

impl ComponentGallery {
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", TabPrevious, None),
        ]);
        TextInput::register_key_bindings(cx);
        M3Dialog::register_key_bindings(cx);
        M3Menu::register_key_bindings(cx);
        M3Select::register_key_bindings(cx);
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_scale(window, UiScale::default(), cx)
    }

    pub fn new_with_scale(window: &mut Window, scale: UiScale, cx: &mut Context<Self>) -> Self {
        window.set_window_title("Dagsverk GPUI Preview");
        let colors = M3ColorScheme::light();
        let specs = [
            ("theme", "Toggle theme", M3ButtonVariant::Filled),
            ("filled", "Filled", M3ButtonVariant::Filled),
            ("tonal", "Tonal", M3ButtonVariant::Tonal),
            ("outlined", "Outlined", M3ButtonVariant::Outlined),
            ("text", "Text", M3ButtonVariant::Text),
            ("disabled", "Disabled", M3ButtonVariant::Filled),
            ("dialog", "Open dialog", M3ButtonVariant::Tonal),
            ("menu", "Open menu", M3ButtonVariant::Outlined),
            ("snackbar", "Show snackbar", M3ButtonVariant::Text),
        ];
        let buttons: Vec<_> = specs
            .into_iter()
            .map(|(id, label, variant)| cx.new(|cx| M3Button::new(id, label, variant, colors, cx)))
            .collect();
        for button in &buttons {
            button.update(cx, |button, cx| button.set_scale(scale, cx));
        }
        buttons[5].update(cx, |button, cx| button.set_enabled(false, cx));
        buttons[1].update(cx, |button, cx| button.set_leading_icon(Some("check"), cx));
        let icon_button = cx.new(|cx| M3IconButton::new("gallery-icon", "settings", colors, cx));
        icon_button.update(cx, |button, cx| button.set_scale(scale, cx));
        cx.subscribe(&icon_button, |gallery, _, _, cx| {
            gallery.activations += 1;
            cx.notify();
        })
        .detach();
        let dialog = cx.new(|cx| {
            M3Dialog::new(
                "gallery-dialog",
                "Material dialog",
                "Escape, the backdrop, or Close dismisses this dialog. Focus stays on its action.",
                colors,
                cx,
            )
        });
        let menu = cx.new(|cx| {
            M3Menu::new(
                ["Fill normal workdays", "Copy month", "Reset month"],
                colors,
                cx,
            )
        });
        let snackbar = cx.new(|cx| M3SnackbarHost::new(colors, cx));
        dialog.update(cx, |dialog, cx| dialog.set_scale(scale, cx));
        menu.update(cx, |menu, cx| menu.set_scale(scale, cx));
        snackbar.update(cx, |snackbar, cx| snackbar.set_scale(scale, cx));

        for (index, button) in buttons.iter().enumerate() {
            let dialog = dialog.clone();
            let menu = menu.clone();
            let snackbar = snackbar.clone();
            cx.subscribe(button, move |gallery, _, _, cx| {
                if index == 0 {
                    gallery.toggle_theme(cx);
                } else if index == 6 {
                    dialog.update(cx, |dialog, cx| dialog.open(cx));
                } else if index == 7 {
                    menu.update(cx, |menu, cx| menu.open(cx));
                } else if index == 8 {
                    snackbar.update(cx, |snackbar, cx| snackbar.show("Settings saved.", cx));
                } else {
                    gallery.activations += 1;
                    cx.notify();
                }
            })
            .detach();
        }

        let input = cx.new(|cx| TextInput::new(cx, "Text input"));
        let textarea = cx.new(|cx| TextInput::new_multiline(cx, "Notes (Optional)"));
        input.update(cx, |input, cx| input.set_scale(scale, cx));
        textarea.update(cx, |input, cx| input.set_scale(scale, cx));
        input.update(cx, |input, cx| {
            input.set_error(Some("Example validation error".into()), cx)
        });
        textarea.update(cx, |input, cx| {
            input.set_supporting_text(Some("New lines are preserved.".into()), cx)
        });
        let select = cx.new(|cx| {
            M3Select::new(
                "Project",
                ["General", "Customer work", "Internal"],
                0,
                colors,
                cx,
            )
        });
        select.update(cx, |select, cx| select.set_scale(scale, cx));
        cx.subscribe(&input, |gallery, _, event: &TextInputEvent, cx| {
            let TextInputEvent::Changed(_) = event;
            gallery.input_changes += 1;
            cx.notify();
        })
        .detach();
        let switch = cx.new(|cx| M3Switch::new("gallery-switch", true, colors, cx));
        switch.update(cx, |switch, cx| switch.set_scale(scale, cx));
        let chips = vec![
            cx.new(|cx| M3Chip::new("chip-selected", "Selected", true, colors, cx)),
            cx.new(|cx| M3Chip::new("chip-unselected", "Filter chip", false, colors, cx)),
        ];
        for chip in &chips {
            chip.update(cx, |chip, cx| chip.set_scale(scale, cx));
        }
        let tabs = cx.new(|cx| {
            M3ChoiceGroup::new(
                "gallery-tabs",
                ["General", "Schedule", "Application"],
                0,
                M3ChoiceKind::Tabs,
                colors,
                cx,
            )
        });
        let segmented = cx.new(|cx| {
            M3ChoiceGroup::new(
                "gallery-segmented",
                ["Ledger", "Calendar"],
                0,
                M3ChoiceKind::Segmented,
                colors,
                cx,
            )
        });
        tabs.update(cx, |tabs, cx| tabs.set_scale(scale, cx));
        segmented.update(cx, |segmented, cx| segmented.set_scale(scale, cx));
        let expansion = cx.new(|cx| {
            M3ExpansionPanel::new(
                "gallery-expansion",
                "Overtime rule",
                "Scheduled workdays, 18:00-22:00, 50% premium",
                colors,
                cx,
            )
        });
        expansion.update(cx, |panel, cx| panel.set_scale(scale, cx));
        window.focus(&input.read(cx).focus_handle(cx));
        Self {
            buttons,
            icon_button,
            input,
            textarea,
            select,
            switch,
            chips,
            tabs,
            segmented,
            expansion,
            dialog,
            menu,
            snackbar,
            theme: ResolvedTheme::Light,
            activations: 0,
            input_changes: 0,
            scale,
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
        self.icon_button
            .update(cx, |button, cx| button.set_colors(colors, cx));
        self.input
            .update(cx, |input, cx| input.set_colors(colors, cx));
        self.textarea
            .update(cx, |input, cx| input.set_colors(colors, cx));
        self.select
            .update(cx, |select, cx| select.set_colors(colors, cx));
        self.switch
            .update(cx, |switch, cx| switch.set_colors(colors, cx));
        for chip in &self.chips {
            chip.update(cx, |chip, cx| chip.set_colors(colors, cx));
        }
        self.tabs.update(cx, |tabs, cx| tabs.set_colors(colors, cx));
        self.segmented
            .update(cx, |segmented, cx| segmented.set_colors(colors, cx));
        self.expansion
            .update(cx, |panel, cx| panel.set_colors(colors, cx));
        self.dialog
            .update(cx, |dialog, cx| dialog.set_colors(colors, cx));
        self.menu.update(cx, |menu, cx| menu.set_colors(colors, cx));
        self.snackbar
            .update(cx, |snackbar, cx| snackbar.set_colors(colors, cx));
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
        let scale = self.scale;
        div()
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_previous))
            .id("component-gallery")
            .relative()
            .size_full()
            .overflow_y_scroll()
            .p(scale.px(32.0))
            .bg(colors.background)
            .font_family("Roboto")
            .text_color(colors.on_surface)
            .child(
                div()
                    .max_w(scale.px(960.0))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap(scale.px(24.0))
                    .child(
                        div()
                            .text_size(scale.px(24.0))
                            .line_height(scale.px(32.0))
                            .child("Dagsverk Material 3 component gallery"),
                    )
                    .child(
                        m3_card(colors)
                            .p(scale.px(24.0))
                            .flex()
                            .flex_col()
                            .gap(scale.px(16.0))
                            .child("Buttons")
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap(scale.px(12.0))
                                    .children(self.buttons.iter().cloned())
                                    .child(self.icon_button.clone()),
                            )
                            .child(format!("Button activations: {}", self.activations)),
                    )
                    .child(
                        m3_card(colors)
                            .p(scale.px(24.0))
                            .flex()
                            .flex_col()
                            .gap(scale.px(12.0))
                            .child("Text input")
                            .child(self.input.clone())
                            .child(self.textarea.clone())
                            .child(self.select.clone()),
                    )
                    .child(
                        m3_card(colors)
                            .p(scale.px(24.0))
                            .flex()
                            .flex_col()
                            .gap(scale.px(16.0))
                            .child("Selection and status")
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(scale.px(12.0))
                                    .child(self.switch.clone())
                                    .children(self.chips.iter().cloned()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(scale.px(8.0))
                                    .child(m3_status_chip("Neutral", M3Status::Neutral, colors))
                                    .child(m3_status_chip("Worked", M3Status::Success, colors))
                                    .child(m3_status_chip("Warning", M3Status::Warning, colors))
                                    .child(m3_status_chip("Error", M3Status::Error, colors)),
                            ),
                    )
                    .child(
                        m3_card(colors)
                            .p(scale.px(24.0))
                            .flex()
                            .flex_col()
                            .gap(scale.px(16.0))
                            .child("Tabs and progress")
                            .child(self.tabs.clone())
                            .child(m3_divider(colors))
                            .child(self.segmented.clone())
                            .child(m3_progress_bar(0.64, colors))
                            .child(self.expansion.clone()),
                    )
                    .child(
                        m3_card(colors)
                            .p(scale.px(24.0))
                            .flex()
                            .items_center()
                            .gap(scale.px(16.0))
                            .child("Material Symbols")
                            .child(m3_icon("schedule", 24.0 * scale.factor(), colors))
                            .child(m3_icon("calendar_month", 24.0 * scale.factor(), colors))
                            .child(m3_icon("settings", 24.0 * scale.factor(), colors)),
                    )
                    .child(
                        div()
                            .text_size(scale.px(12.0))
                            .text_color(colors.on_surface_variant)
                            .child(
                                "Tab and Shift-Tab move focus. Enter and Space activate buttons.",
                            ),
                    ),
            )
            .child(self.dialog.clone())
            .child(self.menu.clone())
            .child(self.snackbar.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::ComponentGallery;
    use crate::m3::UiScale;
    use gpui::{Focusable, KeyUpEvent, Keystroke, TestAppContext};

    #[gpui::test]
    fn gallery_controls_handle_keyboard_focus_and_disabled_state(cx: &mut TestAppContext) {
        cx.update(ComponentGallery::register_key_bindings);
        let (gallery, cx) = cx.add_window_view(ComponentGallery::new);
        let buttons = gallery.read_with(cx, |gallery, _| gallery.buttons.clone());
        let icon_button = gallery.read_with(cx, |gallery, _| gallery.icon_button.clone());
        let switch = gallery.read_with(cx, |gallery, _| gallery.switch.clone());
        let tabs = gallery.read_with(cx, |gallery, _| gallery.tabs.clone());
        let segmented = gallery.read_with(cx, |gallery, _| gallery.segmented.clone());
        let expansion = gallery.read_with(cx, |gallery, _| gallery.expansion.clone());
        let dialog = gallery.read_with(cx, |gallery, _| gallery.dialog.clone());
        let menu = gallery.read_with(cx, |gallery, _| gallery.menu.clone());
        let snackbar = gallery.read_with(cx, |gallery, _| gallery.snackbar.clone());
        let input = gallery.read_with(cx, |gallery, _| gallery.input.clone());
        let textarea = gallery.read_with(cx, |gallery, _| gallery.textarea.clone());
        let select = gallery.read_with(cx, |gallery, _| gallery.select.clone());

        assert_eq!(
            input.read_with(cx, |input, _| input.error_text().map(str::to_owned)),
            Some("Example validation error".to_owned())
        );

        cx.update(|window, app| window.focus(&input.read(app).focus_handle(app)));
        cx.refresh().expect("refresh focused text input");
        cx.simulate_keystrokes("a");
        assert_eq!(gallery.read_with(cx, |gallery, _| gallery.input_changes), 1);

        cx.update(|window, app| window.focus(&textarea.read(app).focus_handle(app)));
        cx.refresh().expect("refresh focused text area");
        cx.simulate_keystrokes("a enter b");
        assert_eq!(
            textarea.read_with(cx, |input, _| input.text().to_owned()),
            "a\nb"
        );

        cx.update(|window, app| window.focus(&select.read(app).focus_handle(app)));
        cx.refresh().expect("refresh focused select");
        cx.simulate_keystrokes("enter down enter");
        assert_eq!(select.read_with(cx, |select, _| select.selected()), 1);

        cx.update(|window, app| window.focus(&buttons[1].read(app).focus_handle()));
        cx.refresh().expect("refresh focused button");
        cx.run_until_parked();
        cx.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("enter").expect("enter keystroke"),
        });
        assert_eq!(gallery.read_with(cx, |gallery, _| gallery.activations), 1);

        cx.update(|window, app| window.focus(&icon_button.read(app).focus_handle()));
        cx.refresh().expect("refresh focused icon button");
        cx.run_until_parked();
        cx.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("enter").expect("enter keystroke"),
        });
        assert_eq!(gallery.read_with(cx, |gallery, _| gallery.activations), 2);

        cx.update(|window, app| window.focus(&buttons[5].read(app).focus_handle()));
        cx.refresh().expect("refresh disabled button");
        cx.run_until_parked();
        cx.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").expect("space keystroke"),
        });
        assert_eq!(gallery.read_with(cx, |gallery, _| gallery.activations), 2);

        cx.update(|window, app| window.focus(&switch.read(app).focus_handle()));
        cx.refresh().expect("refresh focused switch");
        cx.run_until_parked();
        cx.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").expect("space keystroke"),
        });
        assert!(!switch.read_with(cx, |switch, _| switch.checked()));

        let segmented_focus = segmented
            .read_with(cx, |segmented, _| segmented.focus_handle(0))
            .expect("first segmented item has focus");
        cx.update(|window, _| window.focus(&segmented_focus));
        cx.refresh().expect("refresh segmented focus");
        cx.simulate_keystrokes("right");
        assert_eq!(
            segmented.read_with(cx, |segmented, _| segmented.selected()),
            1
        );

        let tab_focus = tabs
            .read_with(cx, |tabs, _| tabs.focus_handle(0))
            .expect("first tab has focus");
        cx.update(|window, _| window.focus(&tab_focus));
        cx.refresh().expect("refresh tab focus");
        cx.simulate_keystrokes("right");
        assert_eq!(tabs.read_with(cx, |tabs, _| tabs.selected()), 1);

        cx.update(|window, app| window.focus(&expansion.read(app).focus_handle()));
        cx.refresh().expect("refresh expansion focus");
        cx.simulate_keystrokes("enter");
        assert!(expansion.read_with(cx, |expansion, _| expansion.expanded()));

        dialog.update(cx, |dialog, cx| dialog.open(cx));
        cx.refresh().expect("refresh open dialog");
        cx.run_until_parked();
        assert!(cx.update(|window, app| dialog.read(app).focus_handle().is_focused(window)));
        cx.simulate_keystrokes("tab");
        assert!(cx.update(|window, app| dialog.read(app).focus_handle().is_focused(window)));
        cx.simulate_keystrokes("escape");
        assert!(!dialog.read_with(cx, |dialog, _| dialog.is_open()));

        menu.update(cx, |menu, cx| menu.open(cx));
        cx.refresh().expect("refresh open menu");
        cx.run_until_parked();
        cx.simulate_keystrokes("escape");
        assert!(!menu.read_with(cx, |menu, _| menu.is_open()));

        snackbar.update(cx, |snackbar, cx| snackbar.show("Saved", cx));
        assert!(snackbar.read_with(cx, |snackbar, _| snackbar.is_visible()));
    }

    #[gpui::test]
    fn gallery_uses_requested_interface_scale(cx: &mut TestAppContext) {
        let scale = UiScale::from_percent(150).expect("supported scale");
        let (gallery, cx) =
            cx.add_window_view(|window, cx| ComponentGallery::new_with_scale(window, scale, cx));

        assert_eq!(gallery.read_with(cx, |gallery, _| gallery.scale), scale);
    }
}
