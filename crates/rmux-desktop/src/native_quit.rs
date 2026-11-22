//! Route AppKit termination requests through the same confirmation/cleanup as Quit.
#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
mod mac {
    use objc::{sel, sel_impl, runtime::{Class, Object, Sel, BOOL, YES}};
    use std::{ffi::{c_char, c_void}, sync::{OnceLock, atomic::{AtomicU8, Ordering}}};
    const IDLE: u8 = 0;
    const PENDING: u8 = 1;
    const APPROVED: u8 = 2;
    const CANCEL: usize = 0;
    const NOW: usize = 1;
    static STATE: AtomicU8 = AtomicU8::new(IDLE);
    static REQUESTS: OnceLock<async_channel::Sender<()>> = OnceLock::new();

    unsafe extern "C" {
        fn class_addMethod(class: *const Class, selector: Sel, implementation: *const c_void, types: *const c_char) -> BOOL;
    }
    fn request(state: &AtomicU8, requests: &async_channel::Sender<()>) -> usize {
        match state.compare_exchange(IDLE, PENDING, Ordering::SeqCst, Ordering::SeqCst) {
            Err(APPROVED) => NOW,
            Err(_) => CANCEL,
            Ok(_) => match requests.try_send(()) {
                Ok(()) | Err(async_channel::TrySendError::Full(())) => CANCEL,
                Err(async_channel::TrySendError::Closed(())) => {
                    state.store(IDLE, Ordering::SeqCst);
                    CANCEL
                }
            },
        }
    }
    extern "C" fn should_terminate(_: &Object, _: Sel, _: *mut Object) -> usize {
        REQUESTS.get().map_or(CANCEL, |requests| request(&STATE, requests))
    }
    // NSTerminateLater enters an AppKit loop that prevents GPUI's main-queue
    // confirmation task from running. Cancel this request and reissue terminate:
    // only after the ordinary confirmation and cleanup path approves it.
    pub fn cancel() {
        let _ = STATE.compare_exchange(PENDING, IDLE, Ordering::SeqCst, Ordering::SeqCst);
    }
    pub fn approve() { STATE.store(APPROVED, Ordering::SeqCst); }
    pub fn initialize(cx: &mut gpui::App) -> anyhow::Result<()> {
        let class = Class::get("GPUIApplicationDelegate").ok_or_else(|| anyhow::anyhow!("GPUI application delegate unavailable"))?;
        let selector = sel!(applicationShouldTerminate:);
        // Do not replace an existing framework implementation on a future update.
        anyhow::ensure!(class.instance_method(selector).is_none(), "application termination handler already exists");
        let (sender, receiver) = async_channel::bounded(1);
        REQUESTS.set(sender).map_err(|_| anyhow::anyhow!("termination handler already initialized"))?;
        // NSUInteger return, self, selector, NSApplication argument; macOS targets are 64-bit.
        let installed = unsafe { class_addMethod(class, selector, should_terminate as *const c_void, c"Q@:@".as_ptr()) };
        anyhow::ensure!(installed == YES, "could not install application termination handler");
        cx.spawn(async move |cx| {
            while receiver.recv().await.is_ok() {
                if cx.update(|cx| {
                    super::super::request_quit(cx);
                    let shutdown = cx.global::<super::super::Shutdowns>();
                    // A busy prompt gate or vanished target can reject a quit request.
                    if !shutdown.quitting && !shutdown.quit_prompt { cancel(); }
                }).is_err() { cancel(); break; }
            }
        }).detach();
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn native_quit_coalesces_requests_and_waits_for_approval() {
            let state = AtomicU8::new(IDLE);
            let (sender, receiver) = async_channel::bounded(1);
            assert_eq!(request(&state, &sender), CANCEL);
            assert_eq!(request(&state, &sender), CANCEL);
            assert_eq!(receiver.len(), 1);
            receiver.try_recv().unwrap();
            state.store(IDLE, Ordering::SeqCst); // user cancellation allows another request
            assert_eq!(request(&state, &sender), CANCEL);
            assert_eq!(receiver.len(), 1);
            state.store(APPROVED, Ordering::SeqCst);
            assert_eq!(request(&state, &sender), NOW);
            state.store(IDLE, Ordering::SeqCst);
            drop(receiver);
            assert_eq!(request(&state, &sender), CANCEL);
            assert_eq!(state.load(Ordering::SeqCst), IDLE);
        }
    }
}
pub fn initialize(cx: &mut gpui::App) {
    #[cfg(target_os = "macos")]
    if let Err(error) = mac::initialize(cx) { eprintln!("rmux: native quit confirmation unavailable: {error:#}"); }
    #[cfg(not(target_os = "macos"))]
    let _ = cx;
}
pub fn cancel() { #[cfg(target_os = "macos")] mac::cancel(); }
pub fn approve() { #[cfg(target_os = "macos")] mac::approve(); }
