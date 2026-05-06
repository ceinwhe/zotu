use gpui::*;

// ============================================================
// 颜色系统（函数形式，因为 rgb() 不是 const fn）
// ============================================================

/// 背景色
pub fn bg_app() -> Rgba {
    rgb(0xFAFAFA)
}
pub fn bg_sidebar() -> Rgba {
    rgb(0xF5F5F5)
}
pub fn bg_content() -> Rgba {
    rgb(0xFCFCFC)
}
pub fn bg_card() -> Rgba {
    rgb(0xFAFAF9)
}
pub fn bg_hover() -> Rgba {
    rgb(0xEEEEEE)
}
pub fn bg_active() -> Rgba {
    rgb(0xE5E7EB)
}
pub fn bg_input() -> Rgba {
    rgb(0xF3F4F6)
}
pub fn bg_playlist() -> Rgba {
    rgb(0xF5F5F5)
}

/// 文字颜色
pub fn text_primary() -> Rgba {
    rgb(0x111827)
}
pub fn text_secondary() -> Rgba {
    rgb(0x374151)
}
pub fn text_tertiary() -> Rgba {
    rgb(0x6B7280)
}
pub fn text_placeholder() -> Rgba {
    rgb(0x9CA3AF)
}
pub fn text_muted() -> Rgba {
    rgb(0xD1D5DB)
}

/// 边框颜色
pub fn border_default() -> Rgba {
    rgb(0xE5E7EB)
}
pub fn border_light() -> Rgba {
    rgb(0xDDDDDD)
}
pub fn border_focus() -> Rgba {
    rgb(0x3B82F6)
}

/// 强调色
pub fn accent_blue() -> Rgba {
    rgb(0xAED6F1)
}
pub fn accent_red() -> Rgba {
    rgb(0xFF6467)
}

// ============================================================
// 尺寸系统
// ============================================================

/// 侧边栏宽度
pub const SIDEBAR_WIDTH: f32 = 180.0;

/// 菜单项高度
pub const MENU_ITEM_HEIGHT: f32 = 50.0;

/// 标题栏高度
pub const TITLEBAR_HEIGHT: f32 = 70.0;

/// 播放栏高度
pub const PLAYBAR_HEIGHT: f32 = 80.0;

/// 设置项高度
pub const SETTING_ITEM_HEIGHT: f32 = 50.0;

/// 封面缩略图尺寸
pub const COVER_THUMB_SIZE: f32 = 48.0;

/// 封面大图尺寸
pub const COVER_LARGE_SIZE: f32 = 200.0;

/// 搜索框宽度
pub const SEARCH_BOX_WIDTH: f32 = 300.0;

/// 搜索框高度
pub const SEARCH_BOX_HEIGHT: f32 = 36.0;

// ============================================================
// 可复用的辅助函数
// ============================================================

/// 标准输入框聚焦边框颜色
pub fn input_focus_ring(is_focused: bool) -> Rgba {
    if is_focused {
        border_focus()
    } else {
        border_default()
    }
}
