use gpui::{prelude::FluentBuilder, *};

const SIDEBAR_WIDTH: f32 = 180.0;
const ITEM_HEIGHT: f32 = 50.0;

#[derive(Clone, Copy, PartialEq)]
pub enum SidebarItem {
    Library,
    Favorite,
    History,
    Settings,
    Custom(usize),
}

/// 原生菜单项配置
struct Menu {
    icon: Option<&'static str>,
    label: &'static str,
    item: SidebarItem,
    selected: bool,
}

/// 侧边栏菜单
pub struct SideBar {
    origin_menu: Vec<Menu>,

    // 未来可扩展的自定义菜单
    #[allow(dead_code)]
    custom_menu: Vec<Menu>,

    select_setting: bool,
}

impl SideBar {
    pub fn new() -> Self {
        Self {
            origin_menu: vec![
                Menu {
                    icon: Some("library.svg"),
                    label: "曲库",
                    item: SidebarItem::Library,
                    selected: true,
                },
                Menu {
                    icon: Some("heart.svg"),
                    label: "收藏",
                    item: SidebarItem::Favorite,
                    selected: false,
                },
                Menu {
                    icon: Some("history.svg"),
                    label: "历史",
                    item: SidebarItem::History,
                    selected: false,
                },
            ],
            custom_menu: Vec::new(),
            select_setting: false,
        }
    }

    fn select_menus(&mut self, item: &SidebarItem) {
        for menu in self.origin_menu.iter_mut() {
            menu.selected = menu.item == *item;
        }
        self.select_setting = false;
    }

    fn select_setting(&mut self) {
        self.select_setting = true;
        for menu in self.origin_menu.iter_mut() {
            menu.selected = false;
        }
    }
}

impl EventEmitter<SidebarItem> for SideBar {}

impl Render for SideBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(Pixels::from(SIDEBAR_WIDTH))
            .h_full()
            .child(
                div()
                    .flex()
                    .justify_center()
                    .items_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Zotu"),
            )
            .children(self.origin_menu.iter().map(|menu| {
                let item = menu.item.clone();
                render_origin_item(menu.label, menu.icon, menu.label)
                    .when(menu.selected, |this| this.bg(rgb(0xE5E7EB)))
                    .on_click(cx.listener(move |this, _evt, _window, cx| {
                        this.select_menus(&item);
                        cx.emit(item);
                        cx.notify();
                    }))
            }))
            .child(
                div()
                    .id("setting")
                    .size(Pixels::from(36.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .mt_auto()
                    .mx_3()
                    .mb_3()
                    .cursor_pointer()
                    .child(svg().path("setting.svg").size_6().text_color(black()))
                    .on_click(cx.listener(|this, _evt, _window, cx| {
                        cx.emit(SidebarItem::Settings);
                        this.select_setting();
                        cx.notify();
                    }))
                    .when(self.select_setting, |this| this.bg(rgb(0xE5E7EB))),
            )
    }
}

/// 渲染单个菜单项
fn render_origin_item(
    id: impl Into<ElementId>,
    icon: Option<&'static str>,
    label: impl IntoElement,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .h(px(ITEM_HEIGHT))
        .px_1()
        .mx_3()
        .mt_1()
        .rounded_lg()
        .cursor_pointer()
        .when_some(icon, |this, icon| {
            this.child(svg().path(icon).size_6().text_color(black()))
        })
        .child(label)
        .hover(|s| s.bg(rgb(0xE5E7EB)))
}
