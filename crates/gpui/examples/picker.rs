//! A GPUI example demonstrating a file-finder-style picker with:
//! - A custom text input field with selection and cursor support
//! - A searchable list with keyboard navigation (up/down arrows)
//! - Background search with proper cancellation
//! - Real-time filtering as you type

use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use gpui::{
    actions, div, prelude::*, px, rgb, rgba, size, uniform_list, white, App, Application, Bounds,
    Context, CursorStyle, ElementId, ElementInputHandler, Entity, EntityInputHandler, FocusHandle,
    Focusable, GlobalElementId, KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PaintQuad, Pixels, Point, ShapedLine, SharedString, Style, Task, TextRun,
    UTF16Selection, Window, WindowBounds, WindowOptions,
};
use unicode_segmentation::*;

actions!(
    picker_example,
    [
        // Text input actions
        Backspace,
        Delete,
        Left,
        Right,
        SelectAll,
        Paste,
        // Picker actions
        SelectNext,
        SelectPrev,
        Confirm,
        Cancel,
        Quit,
    ]
);

// Simple text input component
struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
}

impl TextInput {
    fn new(placeholder: String, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            placeholder: placeholder.into(),
            selected_range: 0..0,
            selection_reversed: false,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
        }
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace("\n", " "), window, cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify()
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

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            Point::new(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            Point::new(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;

        assert_eq!(last_layout.text, self.content);
        let utf8_index = last_layout.index_for_x(point.x - line_point.x)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
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
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = gpui::relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), gpui::hsla(0., 0., 0.5, 0.6))
        } else {
            (content, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &[run], None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some(gpui::fill(
                    Bounds::new(
                        Point::new(bounds.left() + cursor_pos, bounds.top()),
                        gpui::size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    gpui::blue(),
                )),
            )
        } else {
            (
                Some(gpui::fill(
                    Bounds::from_corners(
                        Point::new(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        Point::new(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    rgba(0x3311ff30),
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let line = prepaint.line.take().unwrap();
        line.paint(bounds.origin, window.line_height(), window, cx)
            .unwrap();

        if focus_handle.is_focused(window) {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .key_context("TextInput")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::paste))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .w_full()
            .h(px(32.))
            .px_2()
            .py_1()
            .bg(white())
            .border_1()
            .border_color(rgb(0xcccccc))
            .child(TextElement { input: cx.entity() })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// Main picker component
struct PickerExample {
    text_input: Entity<TextInput>,
    focus_handle: FocusHandle,
    all_items: Vec<String>,
    filtered_items: Vec<String>,
    selected_index: usize,
    search_task: Option<Task<()>>,
    cancel_flag: Arc<AtomicBool>,
    search_count: Arc<AtomicUsize>,
    last_query: String,
    needs_search_update: bool,
}

impl PickerExample {
    fn new(cx: &mut Context<Self>) -> Self {
        let text_input = cx.new(|cx| TextInput::new("Type to search...".to_string(), cx));

        // Observe text input changes
        cx.observe(&text_input, |picker, _text_input, cx| {
            picker.needs_search_update = true;
            cx.notify();
        })
        .detach();

        // Generate some random sample data
        let all_items = vec![
            "src/main.rs",
            "src/lib.rs",
            "src/components/button.rs",
            "src/components/input.rs",
            "src/components/list.rs",
            "src/utils/helpers.rs",
            "src/utils/formatting.rs",
            "tests/integration_test.rs",
            "tests/unit_test.rs",
            "Cargo.toml",
            "Cargo.lock",
            "README.md",
            "LICENSE",
            "docs/getting_started.md",
            "docs/api_reference.md",
            "examples/basic.rs",
            "examples/advanced.rs",
            "benches/performance.rs",
            "assets/styles.css",
            "assets/logo.png",
            "config/settings.json",
            "config/database.yml",
            "scripts/build.sh",
            "scripts/deploy.sh",
            "src/models/user.rs",
            "src/models/post.rs",
            "src/views/home.rs",
            "src/views/profile.rs",
            "src/controllers/auth.rs",
            "src/controllers/api.rs",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();

        Self {
            text_input,
            focus_handle: cx.focus_handle(),
            all_items: all_items.clone(),
            filtered_items: all_items,
            selected_index: 0,
            search_task: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            search_count: Arc::new(AtomicUsize::new(0)),
            last_query: String::new(),
            needs_search_update: false,
        }
    }

    fn check_and_update_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self
            .text_input
            .read(cx)
            .content
            .to_string()
            .to_lowercase();

        // Only update if query changed
        if query == self.last_query {
            return;
        }

        self.last_query = query.clone();

        // Cancel any in-flight search
        self.cancel_flag.store(true, Ordering::Release);

        // Allocate a new search ID for this search
        // fetch_add returns the old value, so this search gets a unique ID
        let search_id = self.search_count.fetch_add(1, Ordering::SeqCst);
        self.cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_flag = self.cancel_flag.clone();
        let all_items = self.all_items.clone();
        let search_count = self.search_count.clone();

        self.search_task = Some(cx.spawn_in(window, async move |picker, cx| {
            // Perform search on background thread
            let matches = cx
                .background_executor()
                .spawn(async move {
                    // Small delay to demonstrate async behavior (can be removed in production)
                    std::thread::sleep(std::time::Duration::from_millis(10));

                    // Check if cancelled
                    if cancel_flag.load(Ordering::Acquire) {
                        return Vec::new();
                    }

                    // Filter items
                    if query.is_empty() {
                        all_items
                    } else {
                        all_items
                            .into_iter()
                            .filter(|item| item.to_lowercase().contains(&query))
                            .collect()
                    }
                })
                .await;

            // Check if this search is still relevant or has been superseded
            // search_count holds the NEXT search ID that would be allocated,
            // so the latest search that was actually started is search_count - 1
            let current_search_count = search_count.load(Ordering::SeqCst);
            let latest_started_search_id = current_search_count.saturating_sub(1);
            if search_id < latest_started_search_id {
                // A newer search has started, discard these results
                return;
            }

            // Update matches on foreground thread
            picker
                .update(cx, |picker, cx| {
                    picker.filtered_items = matches;
                    picker.selected_index = 0;
                    cx.notify();
                })
                .ok();
        }));
    }

    fn select_next(&mut self, _: &SelectNext, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.filtered_items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_items.len();
            cx.notify();
        }
    }

    fn select_prev(&mut self, _: &SelectPrev, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.filtered_items.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_items.len() - 1;
            } else {
                self.selected_index -= 1;
            }
            cx.notify();
        }
    }

    fn confirm(&mut self, _: &Confirm, _window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(selected) = self.filtered_items.get(self.selected_index) {
            println!("Selected: {}", selected);
        }
    }

    fn cancel(&mut self, _: &Cancel, _window: &mut Window, cx: &mut Context<Self>) {
        // Clear the input - this will trigger the observer which will start a search
        self.text_input.update(cx, |input, cx| {
            input.content = "".into();
            input.selected_range = 0..0;
            cx.notify();
        });
    }
}

impl Render for PickerExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Check if we need to update the search
        if self.needs_search_update {
            self.needs_search_update = false;
            self.check_and_update_search(window, cx);
        }

        let selected_index = self.selected_index;
        let item_count = self.filtered_items.len();
        let filtered_items: Vec<String> = self.filtered_items.clone();

        div()
            .key_context("Picker")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_prev))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::cancel))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xf5f5f5))
            .child(
                // Input at the top
                div()
                    .flex()
                    .flex_col()
                    .p_2()
                    .border_b_1()
                    .border_color(rgb(0xdddddd))
                    .bg(white())
                    .child(self.text_input.clone())
                    .child(
                        div()
                            .mt_1()
                            .text_size(px(12.))
                            .text_color(rgb(0x888888))
                            .child(format!("{} matches", item_count)),
                    ),
            )
            .child(
                // List of matches
                div().flex_1().child(if item_count == 0 {
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_full()
                        .text_color(rgb(0x999999))
                        .child("No matches")
                        .into_any_element()
                } else {
                    uniform_list(
                        "picker-list",
                        item_count,
                        cx.processor(move |_this, range, _window, _cx| {
                            let mut result_items = Vec::new();
                            for ix in range {
                                if ix < filtered_items.len() {
                                    let is_selected = ix == selected_index;
                                    let item_string = format!("{}", &filtered_items[ix]);
                                    result_items.push(
                                        div()
                                            .id(ix)
                                            .px_3()
                                            .py_2()
                                            .cursor_pointer()
                                            .when(is_selected, |div| {
                                                div.bg(rgb(0x0066ff)).text_color(white())
                                            })
                                            .when(!is_selected, |div| {
                                                div.bg(white())
                                                    .hover(|div| div.bg(rgb(0xf0f0f0)))
                                            })
                                            .on_click({
                                                let item_string = item_string.clone();
                                                move |_event, _window, _cx| {
                                                    println!("Clicked: {}", item_string);
                                                }
                                            })
                                            .child(item_string),
                                    );
                                }
                            }
                            result_items
                        }),
                    )
                    .h_full()
                    .into_any_element()
                }),
            )
    }
}

impl Focusable for PickerExample {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(600.0), px(500.0)), cx);
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("down", SelectNext, None),
            KeyBinding::new("up", SelectPrev, None),
            KeyBinding::new("enter", Confirm, None),
            KeyBinding::new("escape", Cancel, None),
            KeyBinding::new("cmd-q", Quit, None),
        ]);

        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(PickerExample::new),
            )
            .unwrap();

        window
            .update(cx, |view, window, cx| {
                window.focus(&view.text_input.focus_handle(cx));
                cx.activate(true);
            })
            .unwrap();

        cx.on_action(|_: &Quit, cx| cx.quit());
    });
}
