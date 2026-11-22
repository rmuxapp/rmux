use gpui::{prelude::*, *};

pub(crate) struct ControlHint(pub(crate) SharedString);
impl Render for ControlHint {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        div()
            .font_family(crate::FONT_FAMILY)
            .px_2()
            .py_1()
            .rounded_md()
            .bg(crate::chrome_style::border())
            .border_1().border_color(crate::chrome_style::border())
            .text_color(crate::chrome_style::foreground())
            .text_sm().line_height(px(18.))
            .max_w(px(380.).min((viewport.width - px(24.)).max(px(1.))))
            .max_h(px(120.).min((viewport.height - px(24.)).max(px(1.))))
            .overflow_hidden()
            .child(div().min_w_0().whitespace_normal().line_clamp(5).text_ellipsis().child(self.0.clone()))
    }
}
