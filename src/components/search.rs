use gpui::*;

pub struct Search;

impl Render for Search {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(Pixels::from(60.0))
            .w(Pixels::from(120.0))
            .bg(rgb(0xE2E8F0))
            .child(
                svg()
                    .path("search.svg")
                    .text_color(black())
                    .size(Pixels::from(36.0))
            )
    }
}