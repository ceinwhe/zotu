use gpui::{prelude::FluentBuilder, *};

use crate::{
    application::{AppAction, AppController, AppEvent, AppRoute},
    theme::*,
};

struct Menu {
    icon: &'static str,
    label: &'static str,
    route: AppRoute,
}

pub struct Sidebar {
    controller: Entity<AppController>,
    menus: Vec<Menu>,
}

impl Sidebar {
    pub fn new(controller: Entity<AppController>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&controller, |_this, _controller, event, cx| {
            if *event == AppEvent::NavigationChanged {
                cx.notify();
            }
        })
        .detach();

        Self {
            controller,
            menus: vec![
                Menu {
                    icon: "svg/library.svg",
                    label: "曲库",
                    route: AppRoute::Library,
                },
                Menu {
                    icon: "svg/heart.svg",
                    label: "收藏",
                    route: AppRoute::Favorite,
                },
                Menu {
                    icon: "svg/history.svg",
                    label: "历史",
                    route: AppRoute::History,
                },
            ],
        }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_route = self.controller.read(cx).state().route();

        div()
            .flex()
            .flex_col()
            .w(Pixels::from(SIDEBAR_WIDTH))
            .h_full()
            .bg(bg_sidebar())
            .child(
                div()
                    .flex()
                    .justify_center()
                    .items_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .h(Pixels::from(TITLEBAR_HEIGHT))
                    .text_color(text_primary())
                    .child("Zotu"),
            )
            .children(self.menus.iter().map(|menu| {
                let route = menu.route;
                let controller = self.controller.clone();
                render_menu_item(menu.label, menu.icon, menu.label)
                    .when(selected_route == route, |this| this.bg(bg_active()))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_this, _evt, _window, cx| {
                            controller.update(cx, |controller, cx| {
                                controller.dispatch(AppAction::Navigate(route), cx);
                            });
                        }),
                    )
            }))
            .child({
                let controller = self.controller.clone();
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
                    .child(
                        svg()
                            .path("svg/setting.svg")
                            .size_6()
                            .text_color(text_secondary()),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_this, _evt, _window, cx| {
                            controller.update(cx, |controller, cx| {
                                controller.dispatch(AppAction::Navigate(AppRoute::Settings), cx);
                            });
                        }),
                    )
                    .when(selected_route == AppRoute::Settings, |this| {
                        this.bg(bg_active())
                    })
            })
    }
}

fn render_menu_item(
    id: impl Into<ElementId>,
    icon: &'static str,
    label: impl IntoElement,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .h(px(MENU_ITEM_HEIGHT))
        .px_1()
        .mx_3()
        .mt_1()
        .rounded_lg()
        .cursor_pointer()
        .child(
            svg()
                .path(icon)
                .size_6()
                .text_color(text_secondary())
                .mr_2(),
        )
        .child(label)
        .hover(|style| style.bg(bg_active()))
}
