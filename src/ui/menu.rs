use gpui::{prelude::FluentBuilder, *};
use uuid::Uuid;

use crate::theme::*;

/// 右键菜单项的动作类型
#[derive(Clone)]
pub enum MenuAction {
    /// 添加到收藏
    AddToFavorite(Uuid),
    /// 从收藏中移除
    RemoveFromFavorite(Uuid),
    /// 下一首播放
    PlayNext(Uuid),
}

/// 菜单项配置
#[derive(Clone)]
pub struct MenuItem {
    /// 显示的标签
    pub label: SharedString,
    /// 关联的动作
    pub action: MenuAction,
    /// 是否危险操作（红色文字）
    pub danger: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>, action: MenuAction) -> Self {
        Self {
            label: label.into(),
            action,
            danger: false,
        }
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
}

pub struct MenuContext {
    /// 当前上下文的目标 UUID
    uuid: Option<Uuid>,
    /// 菜单是否显示
    visible: bool,
    /// 菜单位置
    position: Point<Pixels>,
    /// 目标是否已收藏（用于动态显示菜单项）
    is_favorite: bool,
}

impl EventEmitter<MenuAction> for MenuContext {}

impl MenuContext {
    pub fn new() -> Self {
        Self {
            uuid: None,
            visible: false,
            position: Point::default(),
            is_favorite: false,
        }
    }

    /// 显示菜单
    pub fn show(
        &mut self,
        uuid: &Uuid,
        cx: &mut Context<Self>,
        pos: Point<Pixels>,
        is_favorite: bool,
    ) {
        self.uuid = Some(*uuid);
        self.position = pos;
        self.visible = true;
        self.is_favorite = is_favorite;
        cx.notify();
    }

    /// 隐藏菜单
    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.visible = false;
        cx.notify();
    }

    /// 构建菜单项列表（根据当前状态动态生成）
    fn build_menu_items(&self) -> Vec<MenuItem> {
        let uuid = self.uuid.unwrap_or_default();
        let mut items = Vec::new();

        if self.is_favorite {
            items.push(MenuItem::new(
                "从收藏中移除",
                MenuAction::RemoveFromFavorite(uuid),
            ));
        } else {
            items.push(MenuItem::new("添加到收藏", MenuAction::AddToFavorite(uuid)));
        }

        items.push(MenuItem::new("下一首播放", MenuAction::PlayNext(uuid)));

        items
    }

    /// 执行菜单动作
    fn execute_action(&mut self, action: &MenuAction, cx: &mut Context<Self>) {
        cx.emit(action.clone());
        self.hide(cx);
    }
}

impl Render for MenuContext {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_items = self.build_menu_items();
        let menu_x = self.position.x;
        let menu_y = self.position.y;

        // 点击菜单外部时关闭（id() 先将 Div 转为 Stateful<Div>，使 when 闭包类型匹配）
        div().id("menu-backdrop").when(self.visible, |this| {
            this.absolute()
                .size_full()
                .top_0()
                .left_0()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _evt, _window, cx| {
                        this.hide(cx);
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _evt, _window, cx| {
                        this.hide(cx);
                    }),
                )
                .child(
                    div()
                        .id("context-menu")
                        .absolute()
                        .left(menu_x)
                        .top(menu_y)
                        .min_w(px(180.0))
                        .bg(bg_content())
                        .rounded_lg()
                        .shadow_md()
                        .border_1()
                        .border_color(border_default())
                        .py_1()
                        .children(menu_items.iter().map(|item| {
                            let action = item.action.clone();
                            let is_danger = item.danger;
                            div()
                                .px_3()
                                .py_2()
                                .text_sm()
                                .cursor_pointer()
                                .when(is_danger, |this| this.text_color(accent_red()))
                                .when(!is_danger, |this| this.text_color(text_secondary()))
                                .hover(|s| s.bg(bg_active()))
                                .child(item.label.clone())
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |menu, _evt, _window, cx| {
                                        menu.execute_action(&action, cx);
                                    }),
                                )
                        })),
                )
        })
    }
}
