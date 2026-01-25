use gpui::prelude::*;
use gpui::*;

const SIDEBAR_WIDTH: f32 = 180.0;
const TITLEBAR_HEIGHT: f32 = 40.0;
const NAVIGATE_ITEM_HEIGHT: f32 = 50.0;

pub struct Sidebar;

pub enum SidebarMessage {
    Library,
    Favorite,
    History,
    Settings,
}

impl EventEmitter<SidebarMessage> for Sidebar {}

/// 创建导航项的辅助函数
fn nav_item(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    message: SidebarMessage,
    cx: &mut Context<Sidebar>,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_row()
        .h(px(NAVIGATE_ITEM_HEIGHT))
        .gap_2()
        .bg(rgb(0xF9FAFB))
        .items_center()
        .px_1()
        .mx_3()
        .rounded_lg()
        .cursor_pointer()
        .child(svg().path(icon).size_6().text_color(black()))
        .on_click(cx.listener(move |_this, _event, _window, cx| {
            cx.emit(match message {
                SidebarMessage::Library => SidebarMessage::Library,
                SidebarMessage::Favorite => SidebarMessage::Favorite,
                SidebarMessage::History => SidebarMessage::History,
                SidebarMessage::Settings => SidebarMessage::Settings,
            });
        }))
        .hover(|style| style.bg(rgb(0xE5E5E5)))
        .active(|style| style.bg(rgb(0xF1F5F9)))
        .child(label)
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(0xFAFAFA))
            .child(
                div()
                    .h(px(TITLEBAR_HEIGHT))
                    .w_full()
                    .flex()
                    .justify_center()
                    .items_center()
                    .child("Zotu"),
            )
            .child(nav_item(
                "library",
                "library.svg",
                "曲库",
                SidebarMessage::Library,
                cx,
            ))
            .child(nav_item(
                "favorites",
                "heart.svg",
                "收藏",
                SidebarMessage::Favorite,
                cx,
            ))
            .child(nav_item(
                "history",
                "history.svg",
                "历史",
                SidebarMessage::History,
                cx,
            ))
            .child(
                svg()
                    .path("setting.svg")
                    .flex()
                    .size_6()
                    .text_color(black())
                    .mt_auto()
                    .justify_start()
                    .cursor_pointer()
                    .ml_3()
                    .mb_3()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _evt, _window, cx| {
                            cx.emit(SidebarMessage::Settings);
                        }),
                    ),
            )
    }
}
