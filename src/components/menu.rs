use gpui::*;

pub struct Menu{
    items: Vec<SharedString>,
    _select: Option<usize>,
}

impl Render for Menu {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("menu")
            .absolute()
            .bg(rgb(0xFFFFFF))
            .shadow_md()
            .rounded_lg()
            .children(
                self.items.iter().map(|item:&SharedString| {
                    div()
                        .child(item.clone())
                        .hover(|style| style.bg(rgb(0xF0F0F0)))
                        .cursor_pointer()
                })
            )
    }
}

impl Menu {
    pub fn new(items:Vec<SharedString>)->Self{
        Self{
            items,
            _select: None,
        }
    }
}