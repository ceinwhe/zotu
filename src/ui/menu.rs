use gpui::*;
use uuid::Uuid;

/// 右键菜单项的动作类型
#[derive(Clone)]
pub enum MenuAction {
    /// 添加到收藏
    AddToFavorite(Uuid),
    /// 从收藏中移除
    RemoveFromFavorite(Uuid),
    // === 未来可扩展的动作 ===
    // AddToPlaylist(String),      // 添加到播放列表
    // RemoveFromPlaylist(String), // 从播放列表移除
    // PlayNext,                   // 下一首播放
    // AddToQueue,                 // 添加到播放队列
    // ShowInfo,                   // 显示歌曲信息
    // EditMetadata,               // 编辑元数据
    // DeleteFromLibrary,          // 从曲库删除
    // OpenInExplorer,             // 在文件管理器中打开
}

/// 菜单项配置
#[derive(Clone)]
pub struct MenuItem {
    /// 显示的标签
    pub label: SharedString,
    /// 图标路径（可选）
    pub icon: Option<SharedString>,
    /// 关联的动作
    pub action: MenuAction,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>, action: MenuAction) -> Self {
        Self {
            label: label.into(),
            icon: None,
            action,
        }
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

pub struct MenuContext {
    items: Vec<MenuItem>,
    uuid: Option<Uuid>,
    /// 菜单是否显示
    visible: bool,
    /// 菜单位置
    position: Point<Pixels>,
}

impl EventEmitter<MenuAction> for MenuContext {}

impl MenuContext {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            uuid: None,
            visible: false,
            position: Point::default(),
        }
    }

    /// 显示菜单
    pub fn show(&mut self, uuid: &Uuid, cx: &mut Context<Self>, pos: Point<Pixels>) {
        self.uuid = Some(*uuid);
        self.position = pos;
        self.visible = true;
        cx.notify();
    }

    /// 隐藏菜单
    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.visible = false;
        cx.notify();
    }

    /// 构建菜单项列表
    fn build_menu_items(&mut self, item: MenuItem) {
        self.items.push(item);
    }

    /// 执行菜单动作
    fn execute_action(&mut self, evt: &MenuAction, cx: &mut Context<Self>) {

        cx.emit(evt.clone());
        // 执行完动作后隐藏菜单
        self.hide(cx);
    }

}

impl Render for MenuContext {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let item = MenuItem::new("添加到收藏", MenuAction::AddToFavorite(self.uuid.unwrap_or_default()));

        self.build_menu_items(item);

        // 菜单位置：在鼠标点击位置显示
        let menu_x = self.position.x;
        let menu_y = self.position.y;

        div()
            .id("menu")
            .absolute()
            .left(menu_x)
            .top(menu_y)
            .min_w(px(160.0))
            .children(self.items.iter().map(|this| {
                let action = this.action.clone();
                div()
                    .flex()
                    .flex_row()
                    .child(this.label.clone())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |menu, _evt, _window, cx| {
                            menu.execute_action(&action, cx);
                        }),
                    )
            }))
            .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                if !*hovered {
                    this.hide(cx);
                }
            }))
            // 阻止所有鼠标事件穿透
            .on_mouse_down(MouseButton::Left, |_evt, _window, cx| {
                cx.stop_propagation();
            })
            .on_mouse_down(MouseButton::Right, |_evt, _window, cx| {
                cx.stop_propagation();
            })
    }
}
