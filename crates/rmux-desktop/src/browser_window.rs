//! Opt-in native browser window. Split-tree ownership remains a separate gate.
use super::*;
use crate::text_field::TextField;
use wry::{WebViewBuilderExtDarwin, WebViewExtMacOS};
use std::{cell::RefCell, rc::Rc};

actions!(browser, [FocusAddress, Back, Forward, Reload, ZoomIn, ZoomOut, ResetZoom, FindInPage, FindNext, FindPrevious, ToggleInspector, StopLoading]);
const ZOOM_LEVELS: &[u16] = &[25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400, 500];
fn zoom_level(current: u16, direction: i8) -> u16 {
    match direction.cmp(&0) {
        std::cmp::Ordering::Greater => ZOOM_LEVELS.iter().copied().find(|level| *level > current).unwrap_or(500),
        std::cmp::Ordering::Less => ZOOM_LEVELS.iter().rev().copied().find(|level| *level < current).unwrap_or(25),
        std::cmp::Ordering::Equal => 100,
    }
}
#[derive(Clone, Copy)]
enum Navigation { Back, Forward, Reload }

fn address(text: &str) -> anyhow::Result<String> {
    let text = text.trim();
    anyhow::ensure!(!text.is_empty() && text.len() <= 4096 && !text.chars().any(char::is_control), "Enter an HTTP or HTTPS address");
    // A URL embedded in a query/path must not make a bare hostname look like
    // an explicitly schemed address. Preserve explicit HTTP/HTTPS choices.
    let explicit = text.find("://").is_some_and(|index| !text[..index].contains(['/', '?', '#']))
        || text.get(..6).is_some_and(|prefix| prefix.eq_ignore_ascii_case("about:"));
    let value = if explicit { text.to_owned() } else { format!("https://{text}") };
    let mut url = url::Url::parse(&value)?;
    if !explicit {
        let loopback = match url.host() {
            Some(url::Host::Domain(host)) => {
                let host = host.trim_end_matches('.');
                host == "localhost" || host.ends_with(".localhost")
            }
            Some(url::Host::Ipv4(host)) => host.is_loopback(),
            Some(url::Host::Ipv6(host)) => host.is_loopback(),
            None => false,
        };
        if loopback {
            // Reparse rather than set_scheme: HTTPS parsing can discard an
            // explicit :443, which must stay a port when choosing HTTP.
            url = url::Url::parse(&format!("http://{text}"))?;
        }
    }
    anyhow::ensure!(matches!(url.scheme(), "http" | "https") && url.host_str().is_some() || value == "about:blank", "Only HTTP, HTTPS and about:blank are supported");
    anyhow::ensure!(url.username().is_empty() && url.password().is_none(), "Use a URL without embedded credentials");
    Ok(url.into())
}
// Preserve unsent drafts even after focus leaves the address field. A successful
// submission becomes the new baseline, allowing normalized URLs and redirects.
fn should_sync_address(draft: &str, baseline: &str, committed: &str, focused: bool, composing: bool) -> bool {
    !focused && !composing && !committed.is_empty() && committed != baseline && draft == baseline
}

fn navigation_error(uri: &str) -> Option<&'static str> {
    if uri.len() > 4096 { return Some("This address exceeds the browser's 4096-byte limit."); }
    if uri.chars().any(char::is_control) { return Some("This address contains unsupported control characters."); }
    if uri == "about:blank" { return None; }
    let Ok(url) = url::Url::parse(uri) else { return Some("This address is not a valid URL."); };
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Some("Only HTTP, HTTPS and about:blank navigation are supported.");
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Some("Use an address without embedded credentials.");
    }
    None
}
fn allowed(url: &str) -> bool { navigation_error(url).is_none() }

fn rect(bounds: Bounds<Pixels>) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::LogicalPosition::new(f64::from(f32::from(bounds.origin.x)), f64::from(f32::from(bounds.origin.y))).into(),
        size: wry::dpi::LogicalSize::new(f64::from(f32::from(bounds.size.width)), f64::from(f32::from(bounds.size.height))).into(),
    }
}
// Coalesce independent native callbacks without losing a load event when a
// title event arrives before the single queued wakeup is consumed.
#[derive(Default)]
struct FindState {
    visible: bool,
    case_sensitive: bool,
    generation: u64,
    pending: Option<u64>,
    status: &'static str,
}
impl FindState {
    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.status = "";
    }
    fn begin(&mut self) -> Option<u64> {
        if self.pending.is_some() { return None; }
        self.invalidate();
        self.pending = Some(self.generation);
        self.status = "Searching…";
        self.pending
    }
    fn complete(&mut self, generation: u64, found: bool) -> bool {
        if self.pending != Some(generation) { return false; }
        self.pending = None;
        if self.generation != generation { return false; }
        self.status = if found { "Match found" } else { "No matches" };
        true
    }
    fn navigated(&mut self) {
        self.invalidate();
        self.pending = None;
    }
}
#[derive(Default)]
struct PageUpdates {
    load: Option<(bool, String)>,
    title: Option<String>,
    process_alive: Option<bool>,
    notice: Option<&'static str>,
    found: Option<(u64, bool)>,
}
impl PageUpdates {
    fn loaded(&mut self, finished: bool, url: String) {
        if url.len() > 4096 { return; }
        if !finished { self.title = Some(String::new()); self.process_alive = Some(true); self.notice = None; }
        self.load = Some((finished, url));
    }
    fn terminated(&mut self) {
        self.process_alive = Some(false);
        self.notice = None;
        self.load = None;
        self.title = None;
    }
    fn titled(&mut self, title: &str) {
        self.title = Some(title.chars().take(256).map(|ch| if ch.is_control() { ' ' } else { ch }).collect());
    }
}
#[derive(Default)]
struct PageState {
    status: String,
    committed: String,
    title: String,
    failed: bool,
    loading: bool,
    stopped: bool,
}
impl PageState {
    fn stop(&mut self) {
        self.loading = false;
        self.stopped = true;
        if !self.failed { self.status = "Loading stopped".into(); }
    }
    /// Returns whether a native metadata batch changed visible chrome.
    fn apply(&mut self, update: PageUpdates) -> bool {
        let mut changed = false;
        if let Some(alive) = update.process_alive {
            let failed = !alive;
            changed |= self.failed != failed;
            self.failed = failed;
            if alive {
                changed |= self.stopped;
                self.stopped = false;
            }
            if !alive {
                changed |= self.loading;
                self.loading = false;
                changed |= !self.title.is_empty();
                self.title.clear();
                let status = "Browser content process stopped. Reload to recover.";
                if self.status != status { self.status = status.into(); changed = true; }
            }
        }
        if !self.failed {
            if let Some((finished, url)) = update.load.filter(|(finished, _)| !*finished || !self.stopped) {
                changed |= self.loading != !finished;
                self.loading = !finished;
                if !finished { self.stopped = false; }
                if self.committed != url { self.committed = url; changed = true; }
                let status = if finished { "Loaded" } else { "Loading…" };
                if self.status != status { self.status = status.into(); changed = true; }
            }
            if let Some(title) = update.title {
                if self.title != title { self.title = title; changed = true; }
            }
        }
        if !self.failed {
            if let Some(notice) = update.notice {
                if self.status != notice { self.status = notice.into(); changed = true; }
            }
        }
        changed
    }
}
#[derive(Default)]
struct Host { view: Option<wry::WebView>, bounds: Option<Bounds<Pixels>>, pending_url: Option<String>, inspector_used: bool }
impl Drop for Host {
    fn drop(&mut self) {
        // Close a detached inspector before releasing its owning page.
        if self.inspector_used {
            if let Some(view) = &self.view { view.close_devtools(); }
        }
    }
}
struct BrowserView {
    host: Rc<RefCell<Host>>,
    input: Entity<TextField>,
    address_baseline: String,
    find_input: Entity<TextField>,
    find: FindState,
    page: PageState,
    zoom_percent: u16,
    latest: Arc<std::sync::Mutex<PageUpdates>>,
    wake: async_channel::Sender<()>,
    _updates: Task<()>,
}
impl BrowserView {
    fn new(initial: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextField::new(initial.as_deref().unwrap_or("about:blank"), cx));
        cx.subscribe_in(&input, window, |this, _, event: &text_field::EditEvent, window, cx| {
            if let text_field::EditEvent::Submitted { text, .. } = event {
                this.submit(text, window, cx);
            } else if matches!(event, text_field::EditEvent::Cancelled) {
                if !this.page.committed.is_empty() {
                    this.address_baseline = this.page.committed.clone();
                    this.input.update(cx, |input, cx| input.replace_draft(&this.page.committed, cx));
                }
                if let Some(view) = &this.host.borrow().view { let _ = view.focus(); }
            }
        }).detach();
        cx.subscribe_in(&cx.entity(), window, |this, _, _: &BrowserMetadataChanged, window, cx| {
            this.sync_address(window, cx);
        }).detach();
        let address_focus = input.read(cx).focus.clone();
        cx.on_blur(&address_focus, window, |this, window, cx| this.sync_address(window, cx)).detach();
        let find_input = cx.new(|cx| TextField::new("", cx));
        cx.subscribe_in(&find_input, window, |this, _, event: &text_field::EditEvent, window, cx| {
            match event {
                text_field::EditEvent::Submitted { reverse, .. } => this.find_page(*reverse, cx),
                text_field::EditEvent::Cancelled => this.close_find(window, cx),
                text_field::EditEvent::Changed(_) => { this.find.invalidate(); cx.notify(); }
            }
        }).detach();
        input.read(cx).focus.focus(window);
        let latest = Arc::new(std::sync::Mutex::new(PageUpdates::default()));
        let (wake, receive) = async_channel::bounded(1);
        let updates = latest.clone();
        let task = cx.spawn(async move |entity, cx| {
            let interval = std::time::Duration::from_millis(100);
            let mut next_update = std::time::Instant::now();
            while receive.recv().await.is_ok() {
                let delay = next_update.saturating_duration_since(std::time::Instant::now());
                if !delay.is_zero() { cx.background_executor().timer(delay).await; }
                let update = updates.lock().ok().map(|mut value| std::mem::take(&mut *value));
                if let Some(update) = update {
                    if update.load.is_none() && update.title.is_none() && update.process_alive.is_none() && update.notice.is_none() && update.found.is_none() { continue; }
                    if entity.update(cx, |this: &mut Self, cx| {
                        if matches!(update.load, Some((false, _))) || update.process_alive == Some(false) {
                            this.find.navigated();
                            cx.notify();
                        }
                        if let Some((generation, found)) = update.found {
                            let pending = this.find.pending;
                            if this.find.complete(generation, found) || this.find.pending != pending { cx.notify(); }
                        }
                        if this.page.apply(update) { cx.emit(BrowserMetadataChanged); cx.notify(); }
                    }).is_err() { break; }
                    next_update = std::time::Instant::now() + interval;
                }
            }
        });
        let address_baseline = initial.clone().unwrap_or_else(|| "about:blank".into());
        Self { host: Rc::new(RefCell::new(Host { pending_url: initial, view: None, bounds: None, inspector_used: false })), input, address_baseline, find_input, find: FindState::default(), page: PageState { status: "Enter an address to create an ephemeral browser.".into(), ..Default::default() }, zoom_percent: 100, latest, wake, _updates: task }
    }
    fn sync_address(&mut self, window: &Window, cx: &mut Context<Self>) {
        let input = self.input.read(cx);
        if should_sync_address(input.value(), &self.address_baseline, &self.page.committed,
            input.focus.is_focused(window), input.is_composing()) {
            self.address_baseline = self.page.committed.clone();
            self.input.update(cx, |input, cx| input.replace_draft(&self.address_baseline, cx));
        }
    }
    fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(view) = &self.host.borrow().view { let _ = view.focus_parent(); }
        self.find.visible = true;
        self.find_input.update(cx, |input, cx| input.select_all_text(cx));
        self.find_input.read(cx).focus.focus(window);
        cx.notify();
    }
    fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.find_input.read(cx).is_composing() { return; }
        self.find.visible = false;
        self.find.invalidate();
        if let Some(view) = &self.host.borrow().view { let _ = view.focus(); }
        else { self.input.read(cx).focus.focus(window); }
        cx.notify();
    }
    fn find_page(&mut self, backwards: bool, cx: &mut Context<Self>) {
        if self.find_input.read(cx).is_composing() { return; }
        let query = self.find_input.read(cx).value();
        if query.is_empty() || self.page.failed { return; }
        let host = self.host.borrow();
        let Some(view) = &host.view else { return; };
        let Some(main_thread) = objc2::MainThreadMarker::new() else { return; };
        let Some(generation) = self.find.begin() else { return; };
        let latest = self.latest.clone();
        let wake = self.wake.clone();
        let callback = block2::RcBlock::new(move |result: std::ptr::NonNull<objc2_web_kit::WKFindResult>| {
            // WebKit supplies a live result object for the duration of this callback.
            let found = unsafe { result.as_ref().matchFound() };
            if let Ok(mut update) = latest.lock() {
                if update.found.is_none_or(|(previous, _)| generation >= previous) {
                    update.found = Some((generation, found));
                }
            }
            let _ = wake.try_send(());
        });
        // Called on GPUI's main thread. WebKit copies the completion block;
        // the callback owns only bounded metadata/channel handles, not the view.
        unsafe {
            let config = objc2_web_kit::WKFindConfiguration::new(main_thread);
            config.setBackwards(backwards);
            config.setCaseSensitive(self.find.case_sensitive);
            config.setWraps(true);
            view.webview().findString_withConfiguration_completionHandler(
                &objc2_foundation::NSString::from_str(query), Some(&config), &callback);
        }
        cx.notify();
    }
    fn submit(&mut self, text: &str, window: &Window, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        match address(text).and_then(|url| self.navigate(&url, window)) {
            Ok(()) => {
                self.address_baseline = text.to_owned();
                self.page.status = "Navigation requested".into();
            },
            Err(error) => self.page.status = error.to_string(),
        }
        cx.notify();
    }
    fn navigate(&mut self, url: &str, window: &Window) -> anyhow::Result<()> {
        let mut host = self.host.borrow_mut();
        if let Some(view) = &host.view { view.load_url(url)?; view.focus()?; return Ok(()); }
        let bounds = host.bounds.ok_or_else(|| anyhow::anyhow!("Browser layout is not ready"))?;
        let latest = self.latest.clone();
        let wake = self.wake.clone();
        let title_latest = latest.clone();
        let title_wake = wake.clone();
        let termination_latest = latest.clone();
        let termination_wake = wake.clone();
        let blocked = {
            let latest = self.latest.clone();
            let wake = self.wake.clone();
            move |message: &'static str| {
                if let Ok(mut value) = latest.lock() { value.notice = Some(message); }
                let _ = wake.try_send(());
            }
        };
        let blocked_popup = blocked.clone();
        let blocked_download = blocked.clone();
        let view = wry::WebViewBuilder::new().with_incognito(true).with_devtools(true).with_url(url)
            .with_bounds(rect(bounds)).with_navigation_handler(move |url| {
                if let Some(reason) = navigation_error(&url) { blocked(reason); false } else { true }
            })
            .with_new_window_req_handler(move |_, _| {
                blocked_popup("Pop-up windows are not supported yet.");
                wry::NewWindowResponse::Deny
            })
            .with_download_started_handler(move |_, _| {
                blocked_download("Downloads are not supported yet.");
                false
            })
            .with_on_web_content_process_terminate_handler(move || {
                if let Ok(mut value) = termination_latest.lock() { value.terminated(); }
                let _ = termination_wake.try_send(());
            })
            .with_document_title_changed_handler(move |title| {
                if let Ok(mut value) = title_latest.lock() { value.titled(&title); }
                let _ = title_wake.try_send(());
            })
            .with_on_page_load_handler(move |event, url| {
                // A single coalesced wakeup and bounded URL avoid unbounded page events.
                if url.len() <= 4096 {
                    if let Ok(mut value) = latest.lock() { value.loaded(matches!(event, wry::PageLoadEvent::Finished), url); }
                    let _ = wake.try_send(());
                }
            }).build_as_child(window)?;
        if self.zoom_percent != 100 { view.zoom(f64::from(self.zoom_percent) / 100.)?; }
        host.view = Some(view);
        Ok(())
    }
    fn stop_loading(&mut self, cx: &mut Context<Self>) {
        if let Some(view) = &self.host.borrow().view {
            // The native query covers the interval before the coalesced start
            // callback reaches our chrome. A completed idle page is a no-op.
            if !self.page.loading && !unsafe { view.webview().isLoading() } { return; }
            // GPUI dispatches this action on the main thread. Stop native work
            // before dropping any queued pre-cancel loading notification.
            unsafe { view.webview().stopLoading(); }
            if let Ok(mut update) = self.latest.lock() {
                update.load = None;
                if update.process_alive == Some(true) { update.process_alive = None; }
            }
            self.page.stop();
            cx.notify();
        }
    }
    fn toggle_inspector(&mut self, cx: &mut Context<Self>) {
        let mut host = self.host.borrow_mut();
        if let Some(view) = &host.view {
            if view.is_devtools_open() { view.close_devtools(); }
            else { view.open_devtools(); }
            host.inspector_used = true;
        } else {
            self.page.status = "Open a page before opening Web Inspector.".into();
        }
        cx.notify();
    }
    fn zoom(&mut self, direction: i8, cx: &mut Context<Self>) {
        let next = zoom_level(self.zoom_percent, direction);
        if next == self.zoom_percent { return; }
        if let Some(view) = &self.host.borrow().view {
            if let Err(error) = view.zoom(f64::from(next) / 100.) {
                self.page.status = format!("Could not change page zoom: {error}");
                cx.notify();
                return;
            }
        }
        self.zoom_percent = next;
        cx.notify();
    }
    fn focus_address(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        if let Some(view) = &self.host.borrow().view { let _ = view.focus_parent(); }
        if !self.page.committed.is_empty() { self.address_baseline = self.page.committed.clone(); }
        self.input.update(cx, |input, cx| {
            if !self.page.committed.is_empty() { input.replace_draft(&self.page.committed, cx); }
            input.select_all_text(cx);
        });
        self.input.read(cx).focus.focus(window);
    }
    fn navigation(&mut self, operation: Navigation) {
        let host = self.host.borrow();
        if let Some(view) = &host.view {
            let result = match operation {
                Navigation::Back if view.can_go_back().unwrap_or(false) => view.go_back(),
                Navigation::Forward if view.can_go_forward().unwrap_or(false) => view.go_forward(),
                Navigation::Reload => view.reload(),
                _ => return,
            };
            if let Err(error) = result { self.page.status = error.to_string(); }
        }
    }
}
struct BrowserMetadataChanged;
impl EventEmitter<BrowserMetadataChanged> for BrowserView {}

impl Render for BrowserView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let host = self.host.clone();
        let entity = cx.weak_entity();
        let (back, forward, loaded) = self.host.borrow().view.as_ref().map(|view|
            (view.can_go_back().unwrap_or(false), view.can_go_forward().unwrap_or(false), true)
        ).unwrap_or((false, false, false));
        let find_enabled = loaded && !self.page.failed && self.find.pending.is_none()
            && !self.find_input.read(cx).value().is_empty();
        let focus = Some(self.input.read(cx).focus.clone());
        let reload_control = if self.page.loading {
            pane_view::action_control("browser-stop", "Stop Loading", StopLoading, focus.clone()).child("×")
                .on_click(cx.listener(|this, _, _, cx| this.stop_loading(cx)))
        } else {
            pane_view::action_control("browser-reload", if self.page.failed { "Reload page to recover" } else { "Reload" }, Reload, focus.clone()).child("↻")
                .when(!loaded, |button| button.opacity(0.4).cursor_default())
                .when(loaded, |button| button.on_click(cx.listener(|this, _, _, cx| { this.navigation(Navigation::Reload); cx.notify(); })))
        };
        let detail = format!("{} · {}", self.page.status, self.page.committed);
        div().size_full().flex().flex_col().key_context("Browser")
            .bg(crate::chrome_style::surface()).text_color(crate::chrome_style::foreground()).text_size(px(14.))
            .child(div().h(px(44.)).flex_shrink_0().px_2().flex().items_center().gap_1()
                .child(pane_view::action_control("browser-back", "Back", Back, focus.clone()).child("‹")
                    .when(!back, |button| button.opacity(0.4).cursor_default())
                    .when(back, |button| button.on_click(cx.listener(|this, _, _, cx| { this.navigation(Navigation::Back); cx.notify(); }))))
                .child(pane_view::action_control("browser-forward", "Forward", Forward, focus.clone()).child("›")
                    .when(!forward, |button| button.opacity(0.4).cursor_default())
                    .when(forward, |button| button.on_click(cx.listener(|this, _, _, cx| { this.navigation(Navigation::Forward); cx.notify(); }))))
                .child(reload_control)
                .child(div().flex_1().min_w_0().px_2().py_1().rounded_md().border_1().border_color(crate::chrome_style::border()).bg(crate::chrome_style::elevated()).child(self.input.clone()))
                .child(pane_view::action_control("browser-inspector", "Toggle Web Inspector", ToggleInspector, focus.clone()).child("{}")
                    .when(!loaded, |button| button.opacity(0.4).cursor_default())
                    .when(loaded, |button| button.on_click(cx.listener(|this, _, _, cx| this.toggle_inspector(cx)))))
                .child(pane_view::action_control("browser-find", "Find in Page", FindInPage, focus.clone()).child("⌕")
                    .on_click(cx.listener(|this, _, window, cx| this.open_find(window, cx))))
                .child(pane_view::control_button("browser-go").child("→")
                    .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Go to address · Enter".into())).into())
                    .on_click(cx.listener(|this, _, window, cx| {
                        let text = this.input.read(cx).value().to_owned();
                        this.submit(&text, window, cx);
                    }))))
            .when(self.find.visible, |root| root.child(div().h(px(40.)).flex_shrink_0().px_2().flex().items_center().gap_1()
                .child(div().text_xs().px_1().child("Find"))
                .child(div().flex_1().min_w_0().px_2().py_1().rounded_md().border_1().border_color(crate::chrome_style::border()).bg(crate::chrome_style::elevated()).child(self.find_input.clone()))
                .child(div().w(px(90.)).text_xs().truncate().child(self.find.status))
                .child(pane_view::control_button("browser-find-case").child("Aa")
                    .when(self.find.case_sensitive, |button| button.bg(crate::chrome_style::border()))
                    .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Match case".into())).into())
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.find_input.read(cx).is_composing() { return; }
                        this.find.case_sensitive = !this.find.case_sensitive;
                        this.find.invalidate(); cx.notify();
                    })))
                .child(pane_view::action_control("browser-find-previous", "Previous Match", FindPrevious, Some(self.find_input.read(cx).focus.clone())).child("↑")
                    .when(!find_enabled, |button| button.opacity(0.4).cursor_default())
                    .on_click(cx.listener(|this, _, _, cx| this.find_page(true, cx))))
                .child(pane_view::action_control("browser-find-next", "Next Match", FindNext, Some(self.find_input.read(cx).focus.clone())).child("↓")
                    .when(!find_enabled, |button| button.opacity(0.4).cursor_default())
                    .on_click(cx.listener(|this, _, _, cx| this.find_page(false, cx))))
                .child(pane_view::control_button("browser-find-close").child("×")
                    .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Close Find · Escape".into())).into())
                    .on_click(cx.listener(|this, _, window, cx| this.close_find(window, cx))))))
            .child(div().h(px(24.)).flex_shrink_0().px_3().flex().items_center().gap_2().text_xs()
                .child(div().id("browser-status").flex_1().min_w_0().truncate()
                    .when(self.page.failed, |status| status.text_color(crate::chrome_style::warning()))
                    .tooltip({ let detail = detail.clone(); move |_, cx| cx.new(|_| pane_view::ControlHint(detail.clone().into())).into() })
                    .child(detail))
                .child(pane_view::action_control("browser-zoom", "Actual Size", ResetZoom, focus).w(px(48.)).h(px(24.))
                    .child(format!("{}%", self.zoom_percent))
                    .on_click(cx.listener(|this, _, _, cx| this.zoom(0, cx)))))
            .child(div().flex_1().min_h_0().child(canvas(move |bounds, window, cx| {
                let mut host = host.borrow_mut();
                if host.bounds != Some(bounds) {
                    let result = host.view.as_ref().map(|view| view.set_bounds(rect(bounds))).transpose();
                    match result { Ok(_) => host.bounds = Some(bounds), Err(error) => eprintln!("browser bounds: {error}") }
                }
                let pending = host.pending_url.take();
                drop(host);
                if let Some(url) = pending {
                    // Native child creation needs measured bounds. Defer the
                    // entity update until prepaint has released its host borrow.
                    window.defer(cx, move |window, cx| {
                        let _ = entity.update(cx, |this, cx| {
                            match this.navigate(&url, window) {
                                Ok(()) => this.page.status = "Navigation requested".into(),
                                Err(error) => this.page.status = error.to_string(),
                            }
                            cx.notify();
                        });
                    });
                }
            }, |_, _, _, _| {}).size_full()))
    }
}
// A window owns one browser view today. Keeping window title/close policy here
// lets a future pane own the same view without closing or renaming its workspace.
struct BrowserWindow {
    browser: Entity<BrowserView>,
    displayed_title: String,
    _metadata: Subscription,
}
impl BrowserWindow {
    fn new(initial: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let browser = cx.new(|cx| BrowserView::new(initial, window, cx));
        let metadata = cx.subscribe(&browser, |_, _, _: &BrowserMetadataChanged, cx| cx.notify());
        Self { browser, displayed_title: String::new(), _metadata: metadata }
    }
}
impl Render for BrowserWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page_title = &self.browser.read(cx).page.title;
        let title = if page_title.is_empty() { "rmux · Browser (Experimental)".to_owned() }
            else { format!("{page_title} — rmux Browser (Experimental)") };
        if self.displayed_title != title {
            window.set_window_title(&title);
            self.displayed_title = title;
        }
        div().size_full()
            .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
            .child(self.browser.clone())
    }
}
fn with_active(cx: &mut App, update: impl FnOnce(&mut BrowserView, &mut Window, &mut Context<BrowserView>)) {
    if let Some(handle) = cx.active_window().and_then(|window| window.downcast::<BrowserWindow>()) {
        let _ = handle.update(cx, |root, window, cx| {
            root.browser.update(cx, |browser, cx| {
                if !browser.input.read(cx).is_composing() && !browser.find_input.read(cx).is_composing() { update(browser, window, cx); }
            });
        });
    }
}
pub fn initialize(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-.", StopLoading, Some("Browser")),
        KeyBinding::new("cmd-alt-i", ToggleInspector, Some("Browser")),
        KeyBinding::new("cmd-f", FindInPage, Some("Browser")),
        KeyBinding::new("cmd-g", FindNext, Some("Browser")),
        KeyBinding::new("cmd-shift-g", FindPrevious, Some("Browser")),
        KeyBinding::new("cmd-l", FocusAddress, Some("Browser")),
        KeyBinding::new("cmd-r", Reload, Some("Browser")),
        KeyBinding::new("cmd-=", ZoomIn, Some("Browser")),
        KeyBinding::new("cmd--", ZoomOut, Some("Browser")),
        KeyBinding::new("cmd-0", ResetZoom, Some("Browser")),
        KeyBinding::new("alt-left", Back, Some("Browser")),
        KeyBinding::new("alt-right", Forward, Some("Browser")),
    ]);
    cx.on_action(|_: &StopLoading, cx| with_active(cx, |this, _, cx| this.stop_loading(cx)));
    cx.on_action(|_: &ToggleInspector, cx| with_active(cx, |this, _, cx| this.toggle_inspector(cx)));
    cx.on_action(|_: &FindInPage, cx| with_active(cx, |this, window, cx| this.open_find(window, cx)));
    cx.on_action(|_: &FindNext, cx| with_active(cx, |this, window, cx| {
        if !this.find.visible { this.open_find(window, cx); }
        this.find_page(false, cx);
    }));
    cx.on_action(|_: &FindPrevious, cx| with_active(cx, |this, window, cx| {
        if !this.find.visible { this.open_find(window, cx); }
        this.find_page(true, cx);
    }));
    cx.on_action(|_: &ZoomIn, cx| with_active(cx, |this, _, cx| this.zoom(1, cx)));
    cx.on_action(|_: &ZoomOut, cx| with_active(cx, |this, _, cx| this.zoom(-1, cx)));
    cx.on_action(|_: &ResetZoom, cx| with_active(cx, |this, _, cx| this.zoom(0, cx)));
    cx.on_action(|_: &FocusAddress, cx| with_active(cx, |this, window, cx| this.focus_address(window, cx)));
    cx.on_action(|_: &Back, cx| with_active(cx, |this, _, cx| { this.navigation(Navigation::Back); cx.notify(); }));
    cx.on_action(|_: &Forward, cx| with_active(cx, |this, _, cx| { this.navigation(Navigation::Forward); cx.notify(); }));
    cx.on_action(|_: &Reload, cx| with_active(cx, |this, _, cx| { this.navigation(Navigation::Reload); cx.notify(); }));
}
pub fn menu() -> Menu {
    Menu { name: "Browser".into(), items: vec![
        MenuItem::action("Toggle Web Inspector", ToggleInspector),
        MenuItem::action("Find in Page", FindInPage),
        MenuItem::action("Find Next", FindNext),
        MenuItem::action("Find Previous", FindPrevious),
        MenuItem::action("Focus Address", FocusAddress),
        MenuItem::action("Back", Back),
        MenuItem::action("Forward", Forward),
        MenuItem::action("Reload", Reload),
        MenuItem::action("Stop Loading", StopLoading),
        MenuItem::action("Zoom In", ZoomIn),
        MenuItem::action("Zoom Out", ZoomOut),
        MenuItem::action("Actual Size", ResetZoom),
    ] }
}

pub fn open(cx: &mut App) -> anyhow::Result<()> { open_initial(None, cx) }
pub fn can_open_link(uri: &str) -> bool {
    uri.len() <= 4096 && !uri.chars().any(char::is_control) && allowed(uri)
}
pub fn open_link(uri: &str, cx: &mut App) -> anyhow::Result<()> {
    anyhow::ensure!(can_open_link(uri), "This link cannot be opened in the rmux browser");
    open_initial(Some(address(uri)?), cx)
}
fn open_initial(initial: Option<String>, cx: &mut App) -> anyhow::Result<()> {
    anyhow::ensure!(!cx.global::<Shutdowns>().quitting, "application is quitting");
    cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::centered(size(px(1000.), px(680.)), cx)), ..Default::default() }, move |window, cx| {
        window.set_window_title("rmux · Browser (Experimental)");
        cx.new(|cx| BrowserWindow::new(initial, window, cx))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{should_sync_address, FindState, address, allowed, can_open_link, PageUpdates, PageState, zoom_level, navigation_error};
    #[test]
    fn browser_address_sync_preserves_edits_and_accepts_redirects() {
        let old = "https://example.com/old";
        let next = "https://example.com/new";
        assert!(should_sync_address(old, old, next, false, false));
        assert!(!should_sync_address(old, old, next, true, false));
        assert!(!should_sync_address(old, old, next, false, true));
        assert!(!should_sync_address("unsent 世界 draft", old, next, false, false));
        assert!(!should_sync_address(old, old, "", false, false));
        assert!(!should_sync_address(old, old, old, false, false));
        // Successful submission records the user's draft as a new baseline;
        // normalized/redirected callback URLs can then update it.
        assert!(should_sync_address("localhost:3000", "localhost:3000", "http://localhost:3000/login", false, false));
    }
    #[test]
    fn browser_stop_ignores_late_finish_and_resets_on_new_navigation() {
        let mut state = PageState::default();
        let mut started = PageUpdates::default();
        started.loaded(false, "https://example.com/".into());
        state.apply(started);
        assert!(state.loading);
        state.stop();
        assert!(!state.loading);
        let mut late = PageUpdates::default();
        late.loaded(true, "https://example.com/".into());
        assert!(!state.apply(late));
        assert_eq!(state.status, "Loading stopped");
        let mut next = PageUpdates::default();
        next.loaded(false, "https://example.com/next".into());
        state.apply(next);
        assert!(state.loading && !state.stopped);
        let mut finished = PageUpdates::default();
        finished.loaded(true, "https://example.com/next".into());
        state.apply(finished);
        assert!(!state.loading);
        assert_eq!(state.status, "Loaded");
        let mut failed = PageUpdates::default();
        failed.terminated(); state.apply(failed); state.stop();
        assert!(state.status.contains("process stopped"));
        let mut coalesced = PageUpdates::default();
        coalesced.loaded(false, "https://example.com/new".into());
        coalesced.loaded(true, "https://example.com/new".into());
        state.apply(coalesced);
        assert!(!state.loading && !state.stopped && !state.failed);
        assert_eq!(state.status, "Loaded");
    }
    #[test]
    fn browser_blocked_navigation_has_bounded_nonsecret_feedback() {
        assert_eq!(navigation_error("https://user:secret@example.com"), Some("Use an address without embedded credentials."));
        assert!(navigation_error(&format!("https://example.com/{}", "x".repeat(4096))).is_some());
        assert!(navigation_error("https://example.com/a\nb").is_some());
        assert!(navigation_error("mailto:user@example.com").is_some());
        assert_eq!(navigation_error("https://example.com/path"), None);
        let mut state = PageState::default();
        let notice = || PageUpdates { notice: Some("Downloads are not supported yet."), ..Default::default() };
        assert!(state.apply(notice()));
        assert!(!state.apply(notice()));
        let mut next = notice();
        next.loaded(false, "https://example.com/".into());
        assert!(state.apply(next));
        assert_eq!(state.status, "Loading…");
        let mut failed = notice(); failed.terminated(); state.apply(failed);
        assert!(!state.apply(notice()));
        assert!(state.status.contains("process stopped"));
    }
    #[test]
    fn browser_metadata_repaints_only_on_changes_and_keeps_failure_sticky() {
        let mut state = PageState::default();
        let loaded = || PageUpdates { load: Some((true, "https://example.com/".into())), title: Some("Example".into()), ..Default::default() };
        assert!(state.apply(loaded()));
        assert!(!state.apply(loaded()));
        assert!(!state.apply(PageUpdates::default()));
        let mut failed = PageUpdates::default(); failed.terminated();
        assert!(state.apply(failed));
        assert!(state.failed);
        assert!(!state.apply(loaded()));
        assert!(state.title.is_empty());
        assert_eq!(state.committed, "https://example.com/");
        let mut restarted = PageUpdates::default(); restarted.loaded(false, "https://example.com/retry".into());
        assert!(state.apply(restarted));
        assert!(!state.failed);
        assert_eq!(state.status, "Loading…");
    }
    #[test]
    fn browser_zoom_walk_is_bounded_and_returns_to_actual_size() {
        let mut level = 100;
        for _ in 0..100 { level = zoom_level(level, 1); }
        assert_eq!(level, 500);
        for _ in 0..100 { level = zoom_level(level, -1); }
        assert_eq!(level, 25);
        assert_eq!(zoom_level(level, 0), 100);
        assert_eq!(zoom_level(100, 1), 110);
        assert_eq!(zoom_level(100, -1), 90);
    }
    #[test]
    fn browser_termination_survives_late_finish_and_recovers_on_new_load() {
        let mut updates = PageUpdates::default();
        updates.loaded(false, "https://example.com".into());
        updates.titled("old title");
        updates.terminated();
        assert_eq!(updates.process_alive, Some(false));
        assert!(updates.load.is_none() && updates.title.is_none());
        // A queued completion must not declare a terminated process alive.
        updates.loaded(true, "https://example.com".into());
        assert_eq!(updates.process_alive, Some(false));
        let _ = std::mem::take(&mut updates);
        updates.loaded(true, "https://example.com".into());
        assert_eq!(updates.process_alive, None);
        updates.loaded(false, "https://example.com/retry".into());
        assert_eq!(updates.process_alive, Some(true));
        assert_eq!(updates.title.as_deref(), Some(""));
    }
    #[test]
    fn browser_title_updates_coalesce_without_losing_load_state() {
        let mut updates = PageUpdates::default();
        updates.loaded(false, "https://example.com".into());
        updates.titled(&format!("A\nB{}", "界".repeat(1000)));
        updates.loaded(true, "https://example.com/final".into());
        let batch = std::mem::take(&mut updates);
        assert_eq!(batch.load, Some((true, "https://example.com/final".into())));
        let title = batch.title.unwrap();
        assert_eq!(title.chars().count(), 256);
        assert!(title.starts_with("A B"));
        assert!(updates.load.is_none() && updates.title.is_none());
        updates.titled("old page");
        updates.loaded(false, "https://example.org".into());
        assert_eq!(updates.title.as_deref(), Some(""));
        updates.titled("");
        assert_eq!(updates.title.as_deref(), Some(""));
    }
    #[test]
    fn terminal_browser_link_policy_is_bounded_and_explicit() {
        assert!(can_open_link("http://localhost:3000/path?q=hello"));
        assert!(!can_open_link("localhost:3000"));
        assert!(!can_open_link("https://user:secret@example.com"));
        assert!(!can_open_link("https://example.com/a\nb"));
        assert!(!can_open_link(&format!("https://example.com/{}", "a".repeat(4096))));
        assert!(!can_open_link("file:///tmp/test"));
    }
    #[test]
    fn browser_addresses_choose_http_only_for_implicit_loopback() {
        for (input, expected) in [
            ("localhost:3000", "http://localhost:3000/"),
            ("LOCALHOST.:443/path", "http://localhost.:443/path"),
            ("app.localhost:8080", "http://app.localhost:8080/"),
            ("127.0.0.1:8000/?next=https://example.com", "http://127.0.0.1:8000/?next=https://example.com"),
            ("127.1:443", "http://127.0.0.1:443/"),
            ("[::1]:3000", "http://[::1]:3000/"),
            ("https://localhost:3000", "https://localhost:3000/"),
            ("HTTPS://localhost:3000", "https://localhost:3000/"),
            ("example.com:3000", "https://example.com:3000/"),
            ("192.168.1.2:3000", "https://192.168.1.2:3000/"),
            ("localhost.example.com", "https://localhost.example.com/"),
            ("example.com/path?next=http://localhost", "https://example.com/path?next=http://localhost"),
        ] { assert_eq!(address(input).unwrap(), expected, "{input}"); }
        for input in ["localhost:65536", "[::1", "https://example.com/a\nb", "localhost:3000@evil.example"] {
            assert!(address(input).is_err(), "{input}");
        }
    }
    #[test]
    fn browser_addresses_normalize_and_restrict_navigation() {
        assert_eq!(address(" example.com ").unwrap(), "https://example.com/");
        assert_eq!(address("http://localhost:3000/path?q=a").unwrap(), "http://localhost:3000/path?q=a");
        assert_eq!(address("about:blank").unwrap(), "about:blank");
        for url in ["", "file:///tmp/private", "javascript:alert(1)", "data:text/html,test", "https://user:password@example.com"] {
            assert!(address(url).is_err(), "{url}");
        }
        assert!(!allowed("mailto:test@example.com"));
        assert!(!allowed("https://user@example.com"));
        assert!(allowed("https://example.com/path"));
    }

#[test]
fn browser_find_discards_stale_results_and_bounds_native_requests() {
    let mut find = FindState::default();
    let first = find.begin().unwrap();
    assert!(find.begin().is_none());
    find.invalidate(); // query edit/close while the native search is outstanding
    assert!(!find.complete(first, true));
    assert!(find.status.is_empty());
    let second = find.begin().unwrap();
    assert!(find.complete(second, false));
    assert_eq!(find.status, "No matches");
    let old_page = find.begin().unwrap();
    find.navigated();
    let new_page = find.begin().unwrap();
    assert!(!find.complete(old_page, true));
    assert_eq!(find.pending, Some(new_page));
    assert!(find.complete(new_page, true));
    assert_eq!(find.status, "Match found");
}

}
