use gpui::{prelude::FluentBuilder, *};
use uuid::Uuid;

use crate::{
    application::{AppAction, AppController, LibraryAction, PlaybackAction},
    theme::*,
};

#[derive(Clone)]
enum MenuAction {
    AddFavorite(Uuid),
    RemoveFavorite(Uuid),
    Play(Uuid),
}

struct MenuItem {
    label: SharedString,
    action: MenuAction,
    danger: bool,
}

impl MenuItem {
    fn new(label: impl Into<SharedString>, action: MenuAction) -> Self {
        Self {
            label: label.into(),
            action,
            danger: false,
        }
    }
}

pub struct ContextMenu {
    controller: Entity<AppController>,
    uuid: Option<Uuid>,
    visible: bool,
    position: Point<Pixels>,
    is_favorite: bool,
}

impl ContextMenu {
    pub fn new(controller: Entity<AppController>) -> Self {
        Self {
            controller,
            uuid: None,
            visible: false,
            position: Point::default(),
            is_favorite: false,
        }
    }

    pub fn show(
        &mut self,
        uuid: Uuid,
        position: Point<Pixels>,
        is_favorite: bool,
        cx: &mut Context<Self>,
    ) {
        self.uuid = Some(uuid);
        self.position = position;
        self.is_favorite = is_favorite;
        self.visible = true;
        cx.notify();
    }

    fn hide(&mut self, cx: &mut Context<Self>) {
        self.visible = false;
        cx.notify();
    }

    fn menu_items(&self) -> Vec<MenuItem> {
        let Some(id) = self.uuid else {
            return Vec::new();
        };
        let favorite_action = if self.is_favorite {
            MenuItem::new("从收藏中移除", MenuAction::RemoveFavorite(id))
        } else {
            MenuItem::new("添加到收藏", MenuAction::AddFavorite(id))
        };
        vec![favorite_action, MenuItem::new("播放", MenuAction::Play(id))]
    }

    fn execute(&mut self, action: MenuAction, cx: &mut Context<Self>) {
        let app_action = match action {
            MenuAction::AddFavorite(id) => AppAction::Library(LibraryAction::AddFavorite(id)),
            MenuAction::RemoveFavorite(id) => AppAction::Library(LibraryAction::RemoveFavorite(id)),
            MenuAction::Play(id) => AppAction::Playback(PlaybackAction::PlayTrack(id)),
        };
        self.controller.update(cx, |controller, cx| {
            controller.dispatch(app_action, cx);
        });
        self.hide(cx);
    }
}

impl Render for ContextMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_items = self.menu_items();
        let position = self.position;

        div().id("menu-backdrop").when(self.visible, |this| {
            this.absolute()
                .size_full()
                .top_0()
                .left_0()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _evt, _window, cx| this.hide(cx)),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _evt, _window, cx| this.hide(cx)),
                )
                .child(
                    div()
                        .id("context-menu")
                        .absolute()
                        .left(position.x)
                        .top(position.y)
                        .min_w(px(180.0))
                        .bg(bg_content())
                        .rounded_lg()
                        .shadow_md()
                        .border_1()
                        .border_color(border_default())
                        .py_1()
                        .children(menu_items.into_iter().map(|item| {
                            let action = item.action;
                            div()
                                .px_3()
                                .py_2()
                                .text_sm()
                                .cursor_pointer()
                                .when(item.danger, |this| this.text_color(accent_red()))
                                .when(!item.danger, |this| this.text_color(text_secondary()))
                                .hover(|style| style.bg(bg_active()))
                                .child(item.label)
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |menu, _evt, _window, cx| {
                                        menu.execute(action.clone(), cx);
                                    }),
                                )
                        })),
                )
        })
    }
}
