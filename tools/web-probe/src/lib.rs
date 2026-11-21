//! Compile the actual desktop text input against a candidate web backend.
//! This is a compatibility probe, not a replacement demo or second UI.
use gpui::{prelude::*, *};
use std::ops::Range;
const FONT_FAMILY: &str = "Inter";
#[path = "../../../crates/rmux-desktop/src/control_hint.rs"]
mod control_hint;
mod pane_view { pub(crate) use crate::control_hint::ControlHint; }

#[path = "../../../crates/rmux-desktop/src/composition.rs"]
mod composition;
#[path = "../../../crates/rmux-desktop/src/edit_history.rs"]
mod edit_history;
#[path = "../../../crates/rmux-desktop/src/text_field.rs"]
mod text_field;
mod text_layout;

actions!(rmux, [EditorUndo, EditorRedo, Paste, CopySelection, EditorCut, EditorSelectAll]);
mod clipboard {
    pub fn write(item: gpui::ClipboardItem, cx: &mut gpui::App) {
        cx.write_to_clipboard(item);
    }
}
struct InputProbe {
    input: Entity<text_field::TextField>,
}
impl Render for InputProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().font_family(FONT_FAMILY).size_full().bg(rgb(0x252525)).p_4()
            .child(self.input.clone())
    }
}
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    #[cfg(target_family = "wasm")]
    gpui_platform::web_init();
    gpui_platform::application().run(|cx| {
        cx.text_system().add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!("../assets/Inter-Regular.ttf").as_slice())]).expect("bundled Inter font");
        cx.bind_keys([
            KeyBinding::new("cmd-a", EditorSelectAll, Some("TextInput")),
            KeyBinding::new("cmd-z", EditorUndo, Some("TextInput")),
            KeyBinding::new("cmd-shift-z", EditorRedo, Some("TextInput")),
            KeyBinding::new("cmd-v", Paste, Some("TextInput")),
            KeyBinding::new("cmd-c", CopySelection, Some("TextInput")),
            KeyBinding::new("cmd-x", EditorCut, Some("TextInput")),
        ]);
        cx.open_window(WindowOptions::default(), |window, cx| cx.new(|cx| {
            let input = cx.new(|cx| text_field::TextField::new("", cx));
            input.update(cx, |input, cx| {
                input.placeholder = Some("Type a command or workspace name…");
                input.focus.focus(window, cx);
            });
            InputProbe { input }
        })).expect("open probe window");
    });
}
