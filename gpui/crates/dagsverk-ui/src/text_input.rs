//! Text editing behavior adapted from GPUI 0.2.2's official `input` example.

use std::ops::Range;

use gpui::{
    App, Bounds, BoxShadow, ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId,
    InspectorElementId, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    PaintQuad, Pixels, Point, ShapedLine, SharedString, Style, TextRun, UTF16Selection,
    UnderlineStyle, Window, actions, div, fill, point, prelude::*, px, relative, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::m3::{M3ColorScheme, UiScale, m3_icon_colored};

actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        InsertNewline,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
    ]
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputEvent {
    Changed(String),
}

pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Vec<LayoutLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    colors: M3ColorScheme,
    multiline: bool,
    leading_icon: Option<&'static str>,
    suffix: Option<SharedString>,
    supporting_text: Option<SharedString>,
    error_text: Option<SharedString>,
    scale: UiScale,
}

#[derive(Clone)]
struct LayoutLine {
    start: usize,
    line: ShapedLine,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>, placeholder: impl Into<SharedString>) -> Self {
        Self {
            focus_handle: cx.focus_handle().tab_index(1).tab_stop(true),
            content: "".into(),
            placeholder: placeholder.into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: Vec::new(),
            last_bounds: None,
            is_selecting: false,
            colors: M3ColorScheme::light(),
            multiline: false,
            leading_icon: None,
            suffix: None,
            supporting_text: None,
            error_text: None,
            scale: UiScale::default(),
        }
    }

    pub fn new_multiline(cx: &mut Context<Self>, placeholder: impl Into<SharedString>) -> Self {
        let mut input = Self::new(cx, placeholder);
        input.multiline = true;
        input
    }

    pub fn set_colors(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) {
        if self.colors != colors {
            self.colors = colors;
            cx.notify();
        }
    }

    pub fn set_leading_icon(&mut self, icon: &'static str, cx: &mut Context<Self>) {
        self.leading_icon = Some(icon);
        cx.notify();
    }

    pub fn set_suffix(&mut self, suffix: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.suffix = Some(suffix.into());
        cx.notify();
    }

    pub fn set_supporting_text(
        &mut self,
        supporting_text: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        if self.supporting_text != supporting_text {
            self.supporting_text = supporting_text;
            cx.notify();
        }
    }

    pub fn set_error(&mut self, error_text: Option<SharedString>, cx: &mut Context<Self>) {
        if self.error_text != error_text {
            self.error_text = error_text;
            cx.notify();
        }
    }

    pub fn set_scale(&mut self, scale: UiScale, cx: &mut Context<Self>) {
        if self.scale != scale {
            self.scale = scale;
            cx.notify();
        }
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn error_text(&self) -> Option<&str> {
        self.error_text.as_deref().map(|value| &**value)
    }

    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = text.into();
        self.selected_range = self.content.len()..self.content.len();
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
    }

    pub fn register_key_bindings(cx: &mut App) {
        use gpui::KeyBinding;

        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, Some("TextInput")),
            KeyBinding::new("delete", Delete, Some("TextInput")),
            KeyBinding::new("left", Left, Some("TextInput")),
            KeyBinding::new("right", Right, Some("TextInput")),
            KeyBinding::new("shift-left", SelectLeft, Some("TextInput")),
            KeyBinding::new("shift-right", SelectRight, Some("TextInput")),
            KeyBinding::new("cmd-a", SelectAll, Some("TextInput")),
            KeyBinding::new("cmd-v", Paste, Some("TextInput")),
            KeyBinding::new("cmd-c", Copy, Some("TextInput")),
            KeyBinding::new("cmd-x", Cut, Some("TextInput")),
            KeyBinding::new("ctrl-a", SelectAll, Some("TextInput")),
            KeyBinding::new("ctrl-v", Paste, Some("TextInput")),
            KeyBinding::new("ctrl-c", Copy, Some("TextInput")),
            KeyBinding::new("ctrl-x", Cut, Some("TextInput")),
            KeyBinding::new("home", Home, Some("TextInput")),
            KeyBinding::new("end", End, Some("TextInput")),
            KeyBinding::new("enter", InsertNewline, Some("TextInput")),
            KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some("TextInput")),
        ]);
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn insert_newline(&mut self, _: &InsertNewline, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline {
            self.replace_text_in_range(None, "\n", window, cx);
        }
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.multiline {
                text
            } else {
                text.replace(['\n', '\r'], " ")
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        let offset = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(offset, cx);
        } else {
            self.move_to(offset, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        let line_height = self.scale.px(24.0);
        let line_index = (((position.y - bounds.top()) / line_height) as usize)
            .min(self.last_layout.len().saturating_sub(1));
        self.last_layout.get(line_index).map_or(0, |layout| {
            layout.start + layout.line.closest_index_for_x(position.x - bounds.left())
        })
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > offset).then_some(index))
            .unwrap_or(self.content.len())
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        self.content
            .chars()
            .scan((0, 0), |(utf8, utf16), character| {
                let current = (*utf8, *utf16);
                *utf8 += character.len_utf8();
                *utf16 += character.len_utf16();
                Some(current)
            })
            .find_map(|(utf8, utf16)| (utf16 >= offset).then_some(utf8))
            .unwrap_or(self.content.len())
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        self.content[..offset].encode_utf16().count()
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        self.content = format!(
            "{}{}{}",
            &self.content[..range.start],
            new_text,
            &self.content[range.end..]
        )
        .into();
        let cursor = range.start + new_text.len();
        self.selected_range = cursor..cursor;
        self.marked_range = None;
        cx.emit(TextInputEvent::Changed(self.content.to_string()));
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        self.replace_text_in_range(range_utf16, new_text, window, cx);
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|selected| self.range_from_utf16(selected))
            .map(|selected| range.start + selected.start..range.start + selected.end)
            .unwrap_or_else(|| {
                let cursor = range.start + new_text.len();
                cursor..cursor
            });
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        let layout = self.last_layout.iter().find(|layout| {
            range.start >= layout.start && range.start <= layout.start + layout.line.len()
        })?;
        let line_height = self.scale.px(24.0);
        let line_index = self
            .last_layout
            .iter()
            .position(|candidate| candidate.start == layout.start)?;
        Some(Bounds::from_corners(
            point(
                bounds.left() + layout.line.x_for_index(range.start - layout.start),
                bounds.top() + line_height * line_index,
            ),
            point(
                bounds.left()
                    + layout.line.x_for_index(
                        range.end.min(layout.start + layout.line.len()) - layout.start,
                    ),
                bounds.top() + line_height * (line_index + 1),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        let line_height = self.scale.px(24.0);
        let line_index = (((point.y - bounds.top()) / line_height) as usize)
            .min(self.last_layout.len().saturating_sub(1));
        let layout = self.last_layout.get(line_index)?;
        let index = layout.start + layout.line.index_for_x(point.x - bounds.left())?;
        Some(self.offset_to_utf16(index))
    }
}

impl EventEmitter<TextInputEvent> for TextInput {}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    lines: Vec<LayoutLine>,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> PrepaintState {
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();
        let (display_text, text_color) = (content, style.color);
        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line_height = input.scale.px(24.0);
        let lines = display_text
            .split('\n')
            .scan(0, |start, text| {
                let line_start = *start;
                *start += text.len() + 1;
                let line_end = line_start + text.len();
                let marked = input.marked_range.as_ref().and_then(|marked| {
                    let start = marked.start.max(line_start);
                    let end = marked.end.min(line_end);
                    (start < end).then_some(start - line_start..end - line_start)
                });
                let line_runs = marked.map_or_else(
                    || {
                        vec![TextRun {
                            len: text.len(),
                            ..run.clone()
                        }]
                    },
                    |marked| {
                        vec![
                            TextRun {
                                len: marked.start,
                                ..run.clone()
                            },
                            TextRun {
                                len: marked.end - marked.start,
                                underline: Some(UnderlineStyle {
                                    color: Some(run.color),
                                    thickness: input.scale.px(1.0),
                                    wavy: false,
                                }),
                                ..run.clone()
                            },
                            TextRun {
                                len: text.len() - marked.end,
                                ..run.clone()
                            },
                        ]
                        .into_iter()
                        .filter(|run| run.len > 0)
                        .collect()
                    },
                );
                Some(LayoutLine {
                    start: line_start,
                    line: window.text_system().shape_line(
                        text.to_owned().into(),
                        style.font_size.to_pixels(window.rem_size()),
                        &line_runs,
                        None,
                    ),
                })
            })
            .collect::<Vec<_>>();
        let selection = if selected_range.is_empty() {
            Vec::new()
        } else {
            lines
                .iter()
                .enumerate()
                .filter_map(|(index, layout)| {
                    let line_end = layout.start + layout.line.len();
                    let start = selected_range.start.max(layout.start);
                    let end = selected_range.end.min(line_end);
                    (start < end).then(|| {
                        fill(
                            Bounds::from_corners(
                                point(
                                    bounds.left() + layout.line.x_for_index(start - layout.start),
                                    bounds.top() + line_height * index,
                                ),
                                point(
                                    bounds.left() + layout.line.x_for_index(end - layout.start),
                                    bounds.top() + line_height * (index + 1),
                                ),
                            ),
                            input.colors.primary_container,
                        )
                    })
                })
                .collect()
        };
        let cursor_quad = selected_range.is_empty().then(|| {
            let (index, layout) = lines
                .iter()
                .enumerate()
                .find(|(_, layout)| cursor <= layout.start + layout.line.len())
                .unwrap_or_else(|| (0, &lines[0]));
            fill(
                Bounds::new(
                    point(
                        bounds.left()
                            + layout.line.x_for_index(cursor.saturating_sub(layout.start)),
                        bounds.top() + line_height * index,
                    ),
                    size(input.scale.px(2.0), line_height),
                ),
                input.colors.primary,
            )
        });
        PrepaintState {
            lines,
            cursor: cursor_quad,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        for selection in prepaint.selection.drain(..) {
            window.paint_quad(selection);
        }
        let line_height = self.input.read(cx).scale.px(24.0);
        for (index, layout) in prepaint.lines.iter().enumerate() {
            let _ = layout.line.paint(
                point(bounds.left(), bounds.top() + line_height * index),
                line_height,
                window,
                cx,
            );
        }
        self.input.update(cx, |input, _cx| {
            input.last_layout = prepaint.lines.clone();
            input.last_bounds = Some(bounds);
        });
        if focus.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let has_error = self.error_text.is_some();
        let outline = if has_error {
            self.colors.error
        } else if focused {
            self.colors.primary
        } else {
            self.colors.outline
        };
        let hover_outline = if has_error {
            self.colors.error
        } else {
            self.colors.on_surface
        };
        let field = div()
            .key_context("TextInput")
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::insert_newline))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .h(if self.multiline {
                self.scale.px(88.0)
            } else {
                self.scale.px(56.0)
            })
            .w_full()
            .px(self.scale.px(16.0))
            .flex()
            .items_center()
            .gap(self.scale.px(8.0))
            .justify_center()
            .overflow_hidden()
            .rounded(self.scale.px(4.0))
            .border_1()
            .border_color(outline)
            .shadow(if focused || has_error {
                vec![BoxShadow {
                    color: outline,
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(0.0),
                    spread_radius: self.scale.px(1.0),
                }]
            } else {
                Vec::new()
            })
            .hover(move |style| style.border_color(hover_outline))
            .bg(self.colors.surface_container_lowest)
            .text_color(self.colors.on_surface)
            .text_size(self.scale.px(16.0))
            .line_height(self.scale.px(24.0))
            .when_some(self.leading_icon, |field, icon| {
                field.child(m3_icon_colored(
                    icon,
                    20.0 * self.scale.factor(),
                    self.colors.on_surface_variant,
                ))
            })
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(self.scale.px(12.0))
                            .line_height(self.scale.px(16.0))
                            .text_color(self.colors.on_surface_variant)
                            .child(self.placeholder.clone()),
                    )
                    .child(
                        div()
                            .h(if self.multiline {
                                self.scale.px(48.0)
                            } else {
                                self.scale.px(24.0)
                            })
                            .w_full()
                            .child(TextElement { input: cx.entity() }),
                    ),
            )
            .when_some(self.suffix.clone(), |field, suffix| {
                field.child(
                    div()
                        .text_size(self.scale.px(14.0))
                        .text_color(self.colors.on_surface_variant)
                        .child(suffix),
                )
            });
        let supporting = self.error_text.clone().or(self.supporting_text.clone());
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(self.scale.px(4.0))
            .child(field)
            .when_some(supporting, |wrapper, message| {
                wrapper.child(
                    div()
                        .px(self.scale.px(16.0))
                        .text_size(self.scale.px(12.0))
                        .line_height(self.scale.px(16.0))
                        .text_color(if has_error {
                            self.colors.error
                        } else {
                            self.colors.on_surface_variant
                        })
                        .child(message),
                )
            })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
