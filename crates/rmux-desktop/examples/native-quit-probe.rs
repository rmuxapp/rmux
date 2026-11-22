//! Isolated AppKit handshake probe: cancel once, then approve a repeated request.
#[cfg(target_os = "macos")]
#[path = "../src/native_quit.rs"]
mod native_quit;
#[cfg(target_os = "macos")]
#[derive(Default)]
struct Shutdowns { quitting: bool, quit_prompt: bool, requests: usize }
#[cfg(target_os = "macos")]
impl gpui::Global for Shutdowns {}
#[cfg(target_os = "macos")]
fn request_quit(cx: &mut gpui::App) {
    let state = cx.global_mut::<Shutdowns>();
    state.requests += 1;
    if state.requests == 1 {
        native_quit::cancel();
        let timer = cx.background_executor().timer(std::time::Duration::from_millis(50));
        cx.spawn(async move |cx| { timer.await; cx.update(|cx| cx.quit()).unwrap(); }).detach();
    } else {
        assert_eq!(state.requests, 2);
        state.quitting = true;
        native_quit::approve();
        cx.quit();
        println!("PASS AppKit quit routing, cancellation, repeat request and approval");
    }
}
#[cfg(target_os = "macos")]
fn main() {
    gpui::Application::new().run(|cx| {
        cx.set_global(Shutdowns::default());
        native_quit::initialize(cx);
        cx.on_app_quit(|cx| {
            assert_eq!(cx.global::<Shutdowns>().requests, 2, "termination bypassed the handler");
            async {}
        }).detach();
        cx.quit(); // Native terminate:, without rmux's action or cleanup path.
    });
}
#[cfg(not(target_os = "macos"))]
fn main() { eprintln!("This probe requires macOS."); }
