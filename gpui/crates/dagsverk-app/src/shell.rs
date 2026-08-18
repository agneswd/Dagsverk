use dagsverk_core::{
    calculations::normalize_time,
    models::{CurrencyPreference, Minutes, MonthViewPreference, WorkEntryStatus},
};
use dagsverk_ui::{
    m3::{M3ColorScheme, ResolvedTheme as UiTheme, m3_icon},
    text_input::TextInput,
    views::timesheet::{MonthView, MonthViewData, MonthViewEvent, summary_banner},
};
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, KeyBinding, Render, Window, actions,
    div, prelude::*, px,
};

use crate::state::{AppModel, ResolvedTheme, Route};

actions!(
    dagsverk,
    [
        ShowLedger,
        ShowCalendar,
        ShowSettings,
        PreviousMonth,
        NextMonth,
        StartCatchUp,
        SaveActive,
        CloseSurface
    ]
);

pub struct AppShell {
    model: AppModel,
    month_view: Entity<MonthView>,
    start_input: Entity<TextInput>,
    end_input: Entity<TextInput>,
    notes_input: Entity<TextInput>,
    focus: FocusHandle,
    sidebar_collapsed: bool,
}

impl AppShell {
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("ctrl-1", ShowLedger, None),
            KeyBinding::new("ctrl-2", ShowCalendar, None),
            KeyBinding::new("ctrl-,", ShowSettings, None),
            KeyBinding::new("pageup", PreviousMonth, None),
            KeyBinding::new("pagedown", NextMonth, None),
            KeyBinding::new("ctrl-m", StartCatchUp, None),
            KeyBinding::new("ctrl-s", SaveActive, None),
            KeyBinding::new("escape", CloseSurface, None),
        ]);
    }

    pub fn new(model: AppModel, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_window_title("Dagsverk GPUI Preview");
        let month_view = cx.new(|_| MonthView::new(month_view_data(&model)));
        let start_input = cx.new(|cx| TextInput::new(cx, "Start time"));
        let end_input = cx.new(|cx| TextInput::new(cx, "End time"));
        let notes_input = cx.new(|cx| TextInput::new(cx, "Notes"));
        let focus = cx.focus_handle();
        window.focus(&focus);
        cx.subscribe(&month_view, |shell, _, event: &MonthViewEvent, cx| {
            shell.model.open_editor(event.0);
            shell.sync_editor_inputs(cx);
            shell.refresh_month_view(cx);
        })
        .detach();
        Self {
            model,
            month_view,
            start_input,
            end_input,
            notes_input,
            focus,
            sidebar_collapsed: false,
        }
    }

    fn colors(&self) -> M3ColorScheme {
        M3ColorScheme::resolve(match self.model.resolved_theme {
            ResolvedTheme::Light => UiTheme::Light,
            ResolvedTheme::Dark => UiTheme::Dark,
        })
    }

    fn set_route(&mut self, route: Route, cx: &mut Context<Self>) {
        self.model.route = route;
        self.model.close_catch_up();
        cx.notify();
    }

    fn set_view(&mut self, view: MonthViewPreference, cx: &mut Context<Self>) {
        if let Err(error) = self.model.set_view(view) {
            self.model.transient_error = Some(error.to_string());
        } else {
            self.model.route = Route::Timesheet;
        }
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn refresh_month_view(&mut self, cx: &mut Context<Self>) {
        let data = month_view_data(&self.model);
        self.month_view
            .update(cx, |month_view, cx| month_view.set_data(data, cx));
    }

    fn sync_editor_inputs(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.model.editor.draft.as_ref() else {
            return;
        };
        let colors = self.colors();
        let start = draft
            .start_time
            .unwrap_or(self.model.settings.default_start_time)
            .to_string();
        let end = draft
            .end_time
            .unwrap_or(self.model.settings.default_end_time)
            .to_string();
        self.start_input.update(cx, |input, cx| {
            input.set_text(start, cx);
            input.set_colors(colors, cx);
        });
        self.end_input.update(cx, |input, cx| {
            input.set_text(end, cx);
            input.set_colors(colors, cx);
        });
        let notes = draft.notes.clone().unwrap_or_default();
        self.notes_input.update(cx, |input, cx| {
            input.set_text(notes, cx);
            input.set_colors(colors, cx);
        });
    }

    fn save_editor(&mut self, cx: &mut Context<Self>) {
        let Some(mut draft) = self.model.editor.draft.clone() else {
            return;
        };
        if draft.status == WorkEntryStatus::Worked {
            let start = normalize_time(self.start_input.read(cx).text());
            let end = normalize_time(self.end_input.read(cx).text());
            let (Some(start), Some(end)) = (start, end) else {
                self.model.editor.validation_error =
                    Some("Enter valid start and end times.".to_owned());
                cx.notify();
                return;
            };
            draft.start_time = Some(start);
            draft.end_time = Some(end);
        } else if draft.status == WorkEntryStatus::Off {
            draft.start_time = None;
            draft.end_time = None;
            draft.lunch_minutes = Minutes::ZERO;
            draft.project_name = None;
        }
        let notes = self.notes_input.read(cx).text().trim().to_owned();
        draft.notes = (!notes.is_empty()).then_some(notes);
        match self.model.save_entry(draft) {
            Ok(()) => {
                if self.model.catch_up.is_some() {
                    self.model.move_catch_up(1);
                    self.sync_editor_inputs(cx);
                } else {
                    self.model.close_editor();
                }
                self.refresh_month_view(cx);
            }
            Err(error) => self.model.editor.validation_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn load_month(&mut self, key: crate::state::LoadKey, cx: &mut Context<Self>) {
        match self.model.load_for_key(&key) {
            Ok(data) => {
                self.model.apply_load(&key, data);
                self.model.transient_error = None;
                self.refresh_month_view(cx);
            }
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn previous_month(&mut self, _: &PreviousMonth, _: &mut Window, cx: &mut Context<Self>) {
        let key = self.model.previous_month();
        self.load_month(key, cx);
    }

    fn next_month(&mut self, _: &NextMonth, _: &mut Window, cx: &mut Context<Self>) {
        let key = self.model.next_month();
        self.load_month(key, cx);
    }

    fn start_catch_up(&mut self, _: &StartCatchUp, _: &mut Window, cx: &mut Context<Self>) {
        self.model.start_catch_up();
        cx.notify();
    }

    fn save_active(&mut self, _: &SaveActive, _: &mut Window, cx: &mut Context<Self>) {
        self.save_editor(cx);
    }

    fn close_surface(&mut self, _: &CloseSurface, _: &mut Window, cx: &mut Context<Self>) {
        self.model.close_catch_up();
        cx.notify();
    }

    fn navigation_item(
        &self,
        id: &'static str,
        icon: &'static str,
        label: &'static str,
        route: Route,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.model.route == route;
        div()
            .id(id)
            .h(px(56.0))
            .mx(px(12.0))
            .px(px(16.0))
            .flex()
            .items_center()
            .gap(px(16.0))
            .rounded(px(28.0))
            .cursor_pointer()
            .bg(if selected {
                colors.secondary_container
            } else {
                colors.surface_container_low
            })
            .child(m3_icon(icon, 24.0, colors))
            .when(!self.sidebar_collapsed, |item| item.child(label))
            .on_click(cx.listener(move |shell, _, _, cx| shell.set_route(route, cx)))
    }

    fn route_content(&self, colors: M3ColorScheme) -> gpui::Div {
        match self.model.route {
            Route::Timesheet => self.timesheet(colors),
            Route::Projects => self.placeholder_page(
                "Projects",
                format!("{} projects", self.model.projects.len()),
                colors,
            ),
            Route::Settings => self.placeholder_page(
                "Settings",
                "Workspace and application settings are connected to the state model.",
                colors,
            ),
            Route::DataBackups => self.placeholder_page(
                "Data & backups",
                "Backup, restore, and import services are available in dagsverk-data.",
                colors,
            ),
        }
    }

    fn placeholder_page(
        &self,
        title: impl Into<gpui::SharedString>,
        detail: impl Into<gpui::SharedString>,
        colors: M3ColorScheme,
    ) -> gpui::Div {
        div()
            .p(px(32.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(div().text_size(px(24.0)).child(title.into()))
            .child(
                div()
                    .text_color(colors.on_surface_variant)
                    .child(detail.into()),
            )
    }

    fn timesheet(&self, colors: M3ColorScheme) -> gpui::Div {
        let summary = self.model.summary();
        let tax = self.model.tax_estimate();
        let currency = match self.model.settings.currency_preference {
            CurrencyPreference::Sek => "SEK",
            CurrencyPreference::Eur => "EUR",
            CurrencyPreference::Usd => "USD",
            CurrencyPreference::Gbp => "GBP",
            CurrencyPreference::Nok => "NOK",
            CurrencyPreference::Dkk => "DKK",
        };
        div()
            .p(px(24.0))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(summary_banner(
                &summary,
                &tax,
                currency,
                self.model.is_month_unstarted(),
                colors,
            ))
            .child(self.month_view.clone())
    }

    fn editor_panel(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let Some(draft) = self.model.editor.draft.as_ref() else {
            return div();
        };
        let date = draft.date;
        let status = draft.status;
        let lunch = draft.lunch_minutes.value();
        let error = self.model.editor.validation_error.clone();
        div()
            .w(px(400.0))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(colors.outline_variant)
            .bg(colors.surface_container_lowest)
            .child(
                div()
                    .h(px(64.0))
                    .px(px(24.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(colors.grid_line)
                    .child(div().text_size(px(18.0)).child(date.to_string()))
                    .child(
                        div()
                            .id("close-editor")
                            .cursor_pointer()
                            .child(m3_icon("close", 24.0, colors))
                            .on_click(cx.listener(|shell, _, _, cx| {
                                shell.model.close_catch_up();
                                shell.refresh_month_view(cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("editor-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(18.0))
                    .child("Status")
                    .child(
                        div()
                            .flex()
                            .rounded(px(20.0))
                            .border_1()
                            .border_color(colors.outline_variant)
                            .children(
                                [
                                    ("status-worked", "Worked", WorkEntryStatus::Worked),
                                    ("status-off", "Day Off", WorkEntryStatus::Off),
                                    ("status-incomplete", "Unlogged", WorkEntryStatus::Incomplete),
                                ]
                                .into_iter()
                                .map(|(id, label, value)| {
                                    div()
                                        .id(id)
                                        .h(px(40.0))
                                        .px(px(12.0))
                                        .flex_1()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .rounded(px(18.0))
                                        .bg(if status == value {
                                            colors.secondary_container
                                        } else {
                                            colors.surface_container_lowest
                                        })
                                        .child(label)
                                        .on_click(cx.listener(move |shell, _, _, cx| {
                                            if let Some(draft) = shell.model.editor.draft.as_mut() {
                                                draft.status = value;
                                            }
                                            cx.notify();
                                        }))
                                }),
                            ),
                    )
                    .when(status == WorkEntryStatus::Worked, |panel| {
                        panel
                            .child("Presets")
                            .child(
                                div().flex().gap(px(8.0)).children(
                                    [
                                        ("08:00-16:30", "08:00", "16:30"),
                                        ("08:30-17:00", "08:30", "17:00"),
                                        ("09:00-17:30", "09:00", "17:30"),
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    .map(
                                        |(index, (label, start, end))| {
                                            div()
                                                .id(("preset", index))
                                                .h(px(36.0))
                                                .px(px(10.0))
                                                .flex()
                                                .items_center()
                                                .rounded(px(18.0))
                                                .border_1()
                                                .border_color(colors.outline_variant)
                                                .cursor_pointer()
                                                .text_size(px(12.0))
                                                .child(label)
                                                .on_click(cx.listener(move |shell, _, _, cx| {
                                                    shell.start_input.update(cx, |input, cx| {
                                                        input.set_text(start, cx)
                                                    });
                                                    shell.end_input.update(cx, |input, cx| {
                                                        input.set_text(end, cx)
                                                    });
                                                    if let Some(draft) =
                                                        shell.model.editor.draft.as_mut()
                                                    {
                                                        draft.status = WorkEntryStatus::Worked;
                                                        draft.lunch_minutes = Minutes::new(30);
                                                    }
                                                    cx.notify();
                                                }))
                                        },
                                    ),
                                ),
                            )
                            .child("Start")
                            .child(self.start_input.clone())
                            .child("End")
                            .child(self.end_input.clone())
                            .child("Lunch")
                            .child(div().flex().gap(px(8.0)).children(
                                [0_i64, 30, 45, 60].into_iter().map(|minutes| {
                                    div()
                                        .id(("lunch", minutes as usize))
                                        .h(px(36.0))
                                        .px(px(14.0))
                                        .flex()
                                        .items_center()
                                        .rounded(px(18.0))
                                        .cursor_pointer()
                                        .bg(if lunch == minutes {
                                            colors.secondary_container
                                        } else {
                                            colors.surface_container
                                        })
                                        .child(format!("{minutes}m"))
                                        .on_click(cx.listener(move |shell, _, _, cx| {
                                            if let Some(draft) = shell.model.editor.draft.as_mut() {
                                                draft.lunch_minutes = Minutes::new(minutes);
                                            }
                                            cx.notify();
                                        }))
                                }),
                            ))
                    })
                    .child("Notes")
                    .child(self.notes_input.clone())
                    .when_some(error, |panel, error| {
                        panel.child(div().text_color(colors.error).child(error))
                    }),
            )
            .child(
                div()
                    .h(px(72.0))
                    .px(px(24.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(colors.grid_line)
                    .child(
                        div()
                            .id("reset-entry")
                            .cursor_pointer()
                            .text_color(colors.error)
                            .child("Reset")
                            .on_click(cx.listener(move |shell, _, _, cx| {
                                if let Err(error) = shell.model.delete_entry(date) {
                                    shell.model.editor.validation_error = Some(error.to_string());
                                }
                                shell.refresh_month_view(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("save-entry")
                            .h(px(40.0))
                            .px(px(20.0))
                            .flex()
                            .items_center()
                            .rounded(px(20.0))
                            .cursor_pointer()
                            .bg(colors.primary)
                            .text_color(colors.on_primary)
                            .child(if self.model.catch_up.is_some() {
                                "Save and next"
                            } else {
                                "Save entry"
                            })
                            .on_click(cx.listener(|shell, _, _, cx| shell.save_editor(cx))),
                    ),
            )
    }
}

impl Focusable for AppShell {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for AppShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = self.colors();
        let scale = self.model.interface_scale.clamp(0.8, 1.5);
        let sidebar_width = if self.sidebar_collapsed { 80.0 } else { 256.0 } * scale;
        let workspace_name = self
            .model
            .active_workspace()
            .map_or_else(|| "Dagsverk".to_owned(), |workspace| workspace.name.clone());
        let month = format!(
            "{} {:04}",
            [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December"
            ][self.model.current_month.month as usize - 1],
            self.model.current_month.year
        );

        div()
            .track_focus(&self.focus)
            .key_context("Dagsverk")
            .on_action(cx.listener(|shell, _: &ShowLedger, _, cx| {
                shell.set_view(MonthViewPreference::Ledger, cx)
            }))
            .on_action(cx.listener(|shell, _: &ShowCalendar, _, cx| {
                shell.set_view(MonthViewPreference::Calendar, cx)
            }))
            .on_action(
                cx.listener(|shell, _: &ShowSettings, _, cx| shell.set_route(Route::Settings, cx)),
            )
            .on_action(cx.listener(Self::previous_month))
            .on_action(cx.listener(Self::next_month))
            .on_action(cx.listener(Self::start_catch_up))
            .on_action(cx.listener(Self::save_active))
            .on_action(cx.listener(Self::close_surface))
            .size_full()
            .flex()
            .font_family("Roboto")
            .text_color(colors.on_surface)
            .bg(colors.background)
            .child(
                div()
                    .w(px(sidebar_width))
                    .h_full()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .bg(colors.surface_container_low)
                    .child(
                        div()
                            .id("toggle-sidebar")
                            .h(px(72.0 * scale))
                            .px(px(24.0))
                            .flex()
                            .items_center()
                            .gap(px(18.0))
                            .cursor_pointer()
                            .child(m3_icon("menu", 24.0, colors))
                            .when(!self.sidebar_collapsed, |item| item.child("Dagsverk"))
                            .on_click(cx.listener(|shell, _, _, cx| {
                                shell.sidebar_collapsed = !shell.sidebar_collapsed;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .h(px(64.0))
                            .mx(px(12.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .rounded(px(16.0))
                            .bg(colors.surface_container)
                            .when(!self.sidebar_collapsed, |item| item.child(workspace_name)),
                    )
                    .child(self.navigation_item(
                        "nav-timesheet",
                        "schedule",
                        "Timesheet",
                        Route::Timesheet,
                        colors,
                        cx,
                    ))
                    .child(self.navigation_item(
                        "nav-projects",
                        "folder",
                        "Projects",
                        Route::Projects,
                        colors,
                        cx,
                    ))
                    .child(div().flex_1())
                    .child(self.navigation_item(
                        "nav-settings",
                        "settings",
                        "Settings",
                        Route::Settings,
                        colors,
                        cx,
                    )),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(64.0 * scale))
                            .px(px(24.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .bg(colors.surface)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(16.0))
                                    .child(
                                        div()
                                            .id("previous-month")
                                            .cursor_pointer()
                                            .child(m3_icon("chevron_left", 24.0, colors))
                                            .on_click(cx.listener(|shell, _, _, cx| {
                                                let key = shell.model.previous_month();
                                                shell.load_month(key, cx);
                                            })),
                                    )
                                    .child(div().text_size(px(18.0)).child(month))
                                    .child(
                                        div()
                                            .id("next-month")
                                            .cursor_pointer()
                                            .child(m3_icon("chevron_right", 24.0, colors))
                                            .on_click(cx.listener(|shell, _, _, cx| {
                                                let key = shell.model.next_month();
                                                shell.load_month(key, cx);
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .h(px(40.0))
                                    .p(px(2.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(20.0))
                                    .border_1()
                                    .border_color(colors.outline_variant)
                                    .child(
                                        div()
                                            .id("view-ledger")
                                            .h_full()
                                            .px(px(16.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(18.0))
                                            .cursor_pointer()
                                            .bg(
                                                if self.model.active_view
                                                    == MonthViewPreference::Ledger
                                                {
                                                    colors.secondary_container
                                                } else {
                                                    colors.surface
                                                },
                                            )
                                            .child("Ledger")
                                            .on_click(cx.listener(|shell, _, _, cx| {
                                                shell.set_view(MonthViewPreference::Ledger, cx)
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("view-calendar")
                                            .h_full()
                                            .px(px(16.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(18.0))
                                            .cursor_pointer()
                                            .bg(
                                                if self.model.active_view
                                                    == MonthViewPreference::Calendar
                                                {
                                                    colors.secondary_container
                                                } else {
                                                    colors.surface
                                                },
                                            )
                                            .child("Calendar")
                                            .on_click(cx.listener(|shell, _, _, cx| {
                                                shell.set_view(MonthViewPreference::Calendar, cx)
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .id("toggle-theme")
                                    .cursor_pointer()
                                    .child(m3_icon("dark_mode", 24.0, colors))
                                    .on_click(cx.listener(|shell, _, _, cx| {
                                        if let Err(error) = shell.model.toggle_theme() {
                                            shell.model.transient_error = Some(error.to_string());
                                        }
                                        shell.refresh_month_view(cx);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .child(
                                div()
                                    .id("route-content")
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_y_scroll()
                                    .rounded_tl(px(24.0))
                                    .bg(colors.background)
                                    .child(self.route_content(colors)),
                            )
                            .when(self.model.editor.is_open, |content| {
                                content.child(self.editor_panel(colors, cx))
                            }),
                    ),
            )
    }
}

fn month_view_data(model: &AppModel) -> MonthViewData {
    MonthViewData {
        month: model.current_month,
        entries: model.entries.clone(),
        settings: model.settings.clone(),
        projects: model.projects.clone(),
        today: model.today(),
        selected_date: model.selected_date,
        month_started: !model.is_month_unstarted(),
        mode: model.active_view,
        colors: M3ColorScheme::resolve(match model.resolved_theme {
            ResolvedTheme::Light => UiTheme::Light,
            ResolvedTheme::Dark => UiTheme::Dark,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{DateTime, Utc};
    use dagsverk_core::{clock::FixedClock, models::MonthViewPreference, tax::TaxEngine};
    use dagsverk_data::Database;
    use gpui::TestAppContext;
    use tempfile::tempdir;

    use super::AppShell;
    use crate::state::AppModel;

    #[gpui::test]
    fn global_view_shortcuts_persist_from_the_focused_shell(cx: &mut TestAppContext) {
        let directory = tempdir().expect("temporary data directory");
        let now = DateTime::parse_from_rfc3339("2026-08-18T10:00:00Z")
            .expect("fixed time")
            .with_timezone(&Utc);
        let clock = FixedClock::new(now);
        let repository = Arc::new(
            Database::open(directory.path().join("dagsverk.db"), clock)
                .expect("temporary database"),
        );
        let mut model = AppModel::new(repository, Arc::new(clock), TaxEngine::default(), false);
        model.initialize().expect("application state");

        cx.update(AppShell::register_key_bindings);
        let (shell, cx) = cx.add_window_view(|window, cx| AppShell::new(model, window, cx));
        cx.simulate_keystrokes("ctrl-2");
        assert_eq!(
            shell.read_with(cx, |shell, _| shell.model.active_view),
            MonthViewPreference::Calendar
        );
    }
}
