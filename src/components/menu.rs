use gpui::prelude::FluentBuilder;
use gpui::*;

pub struct Menu {
    items: Vec<SharedString>,
    selected: Option<usize>,
}

impl Render for Menu {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("menu")
            .absolute()
            .bg(rgb(0xFFFFFF))
            .shadow_md()
            .rounded_lg()
            .children(self.items.iter().enumerate().map(|(idx, item)| {
                let is_selected = self.selected == Some(idx);
                div()
                    .id(ElementId::Name(format!("menu-item-{}", idx).into()))
                    .child(item.clone())
                    .when(is_selected, |this| this.bg(rgb(0xE0E0E0)))
                    .hover(|style| style.bg(rgb(0xF0F0F0)))
                    .cursor_pointer()
            }))
    }
}

impl Menu {
    pub fn new(items: Vec<SharedString>) -> Self {
        Self {
            items,
            selected: None,
        }
    }

    #[allow(dead_code)]
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }
}
