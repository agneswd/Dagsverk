use dagsverk_core::models::MonthViewPreference;
use dagsverk_ui::m3::{M3ColorScheme, ResolvedTheme as UiTheme, m3_card, m3_icon};
use gpui::{App, Context, KeyBinding, Render, Window, actions, div, prelude::*, px};

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

    pub fn new(model: AppModel, window: &mut Window) -> Self {
        window.set_window_title("Dagsverk GPUI Preview");
        Self {
            model,
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
        cx.notify();
    }

    fn load_month(&mut self, key: crate::state::LoadKey, cx: &mut Context<Self>) {
        match self.model.load_for_key(&key) {
            Ok(data) => {
                self.model.apply_load(&key, data);
                self.model.transient_error = None;
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
        if let Some(entry) = self.model.editor.draft.clone() {
            if let Err(error) = self.model.save_entry(entry) {
                self.model.transient_error = Some(error.to_string());
            } else if self.model.catch_up.is_some() {
                self.model.move_catch_up(1);
            }
        }
        cx.notify();
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
        let cards = [
            ("Worked", format!("{} h", summary.worked_hours)),
            ("Expected", format!("{} h", summary.expected_hours)),
            (
                "Balance",
                format!("{} min", summary.closing_balance_minutes.value()),
            ),
            ("Gross pay", summary.gross_salary.decimal().to_string()),
        ];
        div()
            .p(px(24.0))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap(px(12.0))
                    .children(cards.into_iter().map(|(label, value)| {
                        m3_card(colors)
                            .p(px(18.0))
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(colors.on_surface_variant)
                                    .child(label),
                            )
                            .child(div().text_size(px(20.0)).child(value))
                    })),
            )
            .child(
                m3_card(colors)
                    .p(px(20.0))
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(format!(
                        "{} view - {} saved entries",
                        match self.model.active_view {
                            MonthViewPreference::Ledger => "Ledger",
                            MonthViewPreference::Calendar => "Calendar",
                        },
                        self.model.entries.len()
                    ))
                    .children(self.model.entries.iter().take(31).map(|entry| {
                        div()
                            .h(px(36.0))
                            .flex()
                            .items_center()
                            .border_b_1()
                            .border_color(colors.grid_line)
                            .child(format!("{}  {:?}", entry.date, entry.status))
                    })),
            )
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
                                    .id("toggle-theme")
                                    .cursor_pointer()
                                    .child(m3_icon("dark_mode", 24.0, colors))
                                    .on_click(cx.listener(|shell, _, _, cx| {
                                        if let Err(error) = shell.model.toggle_theme() {
                                            shell.model.transient_error = Some(error.to_string());
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .id("route-content")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .rounded_tl(px(24.0))
                            .bg(colors.background)
                            .child(self.route_content(colors)),
                    ),
            )
    }
}
