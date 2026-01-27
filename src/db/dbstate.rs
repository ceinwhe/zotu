use gpui::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use uuid::Uuid;

use crate::db::metadata::AlbumInfo;

/// 曲库状态事件
#[derive(Clone, Copy)]
pub enum LibraryEvent {
    /// 歌曲被添加到收藏
    FavoriteAdded(Uuid),
    /// 歌曲从收藏中移除
    FavoriteRemoved(Uuid),
    /// 歌曲被添加到历史
    HistoryAdded(Uuid),
    /// 曲库更新
    LibraryUpdated,
}

/// 全局曲库状态管理
/// 在内存中维护曲库、收藏和历史列表，提供高效的增删查操作
pub struct LibraryState {
    /// 完整曲库（只读，作为数据源）
    library: Arc<Vec<AlbumInfo>>,
    /// 曲库索引：UUID -> 索引位置，用于 O(1) 查找
    library_index: HashMap<Uuid, usize>,
    /// 收藏列表（存储索引）
    favorites: Arc<Vec<AlbumInfo>>,
    /// 收藏 UUID 集合，用于 O(1) 判断是否已收藏
    favorite_ids: HashSet<Uuid>,
    /// 历史记录列表（按添加顺序，最新在前）
    history: Arc<Vec<AlbumInfo>>,
    /// 历史 UUID 集合，用于去重
    history_ids: HashSet<Uuid>,
}

impl EventEmitter<LibraryEvent> for LibraryState {}

impl LibraryState {
    pub fn new(
        library: Vec<AlbumInfo>,
        favorite_uuids: Vec<Uuid>,
        history_uuids: Vec<Uuid>,
    ) -> Self {
        // 构建曲库索引
        let library_index: HashMap<Uuid, usize> = library
            .iter()
            .enumerate()
            .map(|(i, item)| (item.id(), i))
            .collect();

        // 从 UUID 列表构建收藏列表
        let mut favorites = Vec::with_capacity(favorite_uuids.len());
        let mut favorite_ids = HashSet::with_capacity(favorite_uuids.len());
        for uuid in favorite_uuids {
            if let Some(&idx) = library_index.get(&uuid) {
                favorites.push(library[idx].clone());
                favorite_ids.insert(uuid);
            }
        }

        // 从 UUID 列表构建历史列表
        let mut history = Vec::with_capacity(history_uuids.len());
        let mut history_ids = HashSet::with_capacity(history_uuids.len());
        for uuid in history_uuids {
            if let Some(&idx) = library_index.get(&uuid) {
                history.push(library[idx].clone());
                history_ids.insert(uuid);
            }
        }

        Self {
            library: Arc::new(library),
            library_index,
            favorites: Arc::new(favorites),
            favorite_ids,
            history: Arc::new(history),
            history_ids,
        }
    }

    // ========== 曲库访问 ==========

    /// 获取完整曲库
    pub fn library(&self) -> Arc<Vec<AlbumInfo>> {
        Arc::clone(&self.library)
    }

    /// 获取收藏列表
    pub fn favorites(&self) -> Arc<Vec<AlbumInfo>> {
        Arc::clone(&self.favorites)
    }

    /// 获取历史列表
    pub fn history(&self) -> Arc<Vec<AlbumInfo>> {
        Arc::clone(&self.history)
    }

    /// 通过 UUID 查找歌曲
    pub fn get_by_id(&self, id: &Uuid) -> Option<&AlbumInfo> {
        self.library_index
            .get(id)
            .and_then(|&idx| self.library.get(idx))
    }

    // ========== 收藏操作 ==========

    /// 检查歌曲是否已收藏
    pub fn is_favorite(&self, id: &Uuid) -> bool {
        self.favorite_ids.contains(id)
    }

    /// 添加歌曲到收藏（如果已存在则不重复添加）
    /// 返回 true 表示成功添加，false 表示已存在
    pub fn add_to_favorites(&mut self, id: &Uuid, cx: &mut Context<Self>) -> bool {
        if self.favorite_ids.contains(id) {
            return false;
        }

        if let Some(item) = self.get_by_id(id).cloned() {
            let mut new_favorites = (*self.favorites).clone();
            new_favorites.push(item);
            self.favorites = Arc::new(new_favorites);
            self.favorite_ids.insert(*id);
            cx.emit(LibraryEvent::FavoriteAdded(*id));
            cx.notify();
            true
        } else {
            false
        }
    }

    /// 从收藏中移除歌曲
    pub fn remove_from_favorites(&mut self, id: &Uuid, cx: &mut Context<Self>) -> bool {
        if !self.favorite_ids.remove(id) {
            return false;
        }

        let new_favorites: Vec<AlbumInfo> = self
            .favorites
            .iter()
            .filter(|item| &item.id() != id)
            .cloned()
            .collect();
        self.favorites = Arc::new(new_favorites);

        cx.emit(LibraryEvent::FavoriteRemoved(*id));
        cx.notify();
        true
    }

    /// 切换收藏状态
    pub fn toggle_favorite(&mut self, id: &Uuid, cx: &mut Context<Self>) -> bool {
        if self.is_favorite(id) {
            self.remove_from_favorites(id, cx)
        } else {
            self.add_to_favorites(id, cx)
        }
    }

    // ========== 历史记录操作 ==========

    /// 添加歌曲到历史记录
    /// 如果已存在，会将其移动到最前面
    pub fn add_to_history(&mut self, id: &Uuid, cx: &mut Context<Self>) -> bool {
        if let Some(item) = self.get_by_id(id).cloned() {
            let mut new_history: Vec<AlbumInfo> = if self.history_ids.contains(id) {
                // 如果已存在，先移除旧记录
                self.history
                    .iter()
                    .filter(|i| &i.id() != id)
                    .cloned()
                    .collect()
            } else {
                self.history_ids.insert(*id);
                (*self.history).clone()
            };

            // 插入到最前面
            new_history.insert(0, item);

            // 限制历史记录数量（可选，保留最近 100 首）
            const MAX_HISTORY: usize = 100;
            if new_history.len() > MAX_HISTORY {
                if let Some(removed) = new_history.pop() {
                    self.history_ids.remove(&removed.id());
                }
            }

            self.history = Arc::new(new_history);
            cx.emit(LibraryEvent::HistoryAdded(*id));
            cx.notify();
            true
        } else {
            false
        }
    }

    /// 清空历史记录
    pub fn clear_history(&mut self, cx: &mut Context<Self>) {
        self.history = Arc::new(Vec::new());
        self.history_ids.clear();
        cx.notify();
    }

    // ========== 曲库更新 ==========

    /// 更新曲库（添加新歌曲后调用）
    pub fn update_library(&mut self, new_library: Vec<AlbumInfo>, cx: &mut Context<Self>) {
        // 重建索引
        self.library_index = new_library
            .iter()
            .enumerate()
            .map(|(i, item)| (item.id(), i))
            .collect();

        self.library = Arc::new(new_library);

        cx.emit(LibraryEvent::LibraryUpdated);
        cx.notify();
    }
}
