use gpui::*;
use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::metadata::AlbumInfo;

/// 曲库状态事件
#[derive(Clone)]
pub enum LibraryEvent {
    /// 歌曲被添加到收藏
    FavoriteAdded(Arc<Uuid>),
    /// 歌曲从收藏中移除
    FavoriteRemoved(Arc<Uuid>),
    /// 歌曲被添加到历史
    HistoryAdded(Arc<Uuid>),
    /// 曲库更新
    LibraryUpdated,
}

/// 全局曲库状态管理
/// 在内存中维护曲库、收藏和历史列表，提供高效的增删查操作
pub struct LibraryState {
    /// 完整曲库（只读，作为数据源）
    library: Option<Arc<Vec<AlbumInfo>>>,
    /// 曲库索引：UUID -> 索引位置，用于 O(1) 查找
    library_index: HashMap<Arc<Uuid>, usize>,
    /// 收藏列表（存储索引）
    favorites: Option<Arc<Vec<AlbumInfo>>>,
    /// 收藏 UUID 集合，用于 O(1) 判断是否已收藏
    favorite_ids: HashSet<Arc<Uuid>>,
    /// 历史记录列表（按添加顺序，最新在前）
    history: Option<Arc<Vec<AlbumInfo>>>,
    /// 历史 UUID 集合，用于去重
    history_ids: HashSet<Arc<Uuid>>,
}

impl EventEmitter<LibraryEvent> for LibraryState {}

impl LibraryState {
    pub fn new(
        library: Option<Vec<AlbumInfo>>,
        favorite_uuids: Option<Vec<Uuid>>,
        history_uuids: Option<Vec<Uuid>>,
    ) -> Self {
        let library = library.map(Arc::new);

        // 构建曲库索引
        let library_index: std::collections::HashMap<Arc<Uuid>, usize> = library
            .as_ref()
            .map(|lib| {
                lib.iter()
                    .enumerate()
                    .map(|(i, item)| (item.id(), i))
                    .collect()
            })
            .unwrap_or_default();

        // 从 UUID 列表构建收藏列表
        let (favorites, favorite_ids) = match (&library, favorite_uuids) {
            (Some(lib), Some(uuids)) => {
                let mut favorites = Vec::with_capacity(uuids.len());
                let mut favorite_ids = HashSet::with_capacity(uuids.len());
                for uuid in uuids {
                    if let Some(&idx) = library_index.get(&uuid) {
                        favorites.push(lib[idx].clone());
                        favorite_ids.insert(Arc::new(uuid));
                    }
                }
                let favorites = if favorites.is_empty() {
                    None
                } else {
                    Some(Arc::new(favorites))
                };
                (favorites, favorite_ids)
            }
            _ => (None, HashSet::new()),
        };

        // 从 UUID 列表构建历史列表
        let (history, history_ids) = match (&library, history_uuids) {
            (Some(lib), Some(uuids)) => {
                let mut history = Vec::with_capacity(uuids.len());
                let mut history_ids = HashSet::with_capacity(uuids.len());
                for uuid in uuids {
                    if let Some(&idx) = library_index.get(&uuid) {
                        history.push(lib[idx].clone());
                        history_ids.insert(Arc::new(uuid));
                    }
                }
                let history = if history.is_empty() {
                    None
                } else {
                    Some(Arc::new(history))
                };
                (history, history_ids)
            }
            _ => (None, HashSet::new()),
        };

        Self {
            library,
            library_index,
            favorites,
            favorite_ids,
            history,
            history_ids,
        }
    }

    // ========== 曲库访问 ==========

    /// 获取完整曲库
    pub fn library(&self) -> Option<Arc<Vec<AlbumInfo>>> {
        self.library.as_ref().map(Arc::clone)
    }

    /// 获取收藏列表
    pub fn favorites(&self) -> Option<Arc<Vec<AlbumInfo>>> {
        self.favorites.as_ref().map(Arc::clone)
    }

    /// 获取历史列表
    pub fn history(&self) -> Option<Arc<Vec<AlbumInfo>>> {
        self.history.as_ref().map(Arc::clone)
    }

    /// 通过 UUID 查找歌曲
    pub fn get_by_id(&self, id: &Arc<Uuid>) -> Option<&AlbumInfo> {
        self.library_index
            .get(id)
            .and_then(|&idx| self.library.as_ref().and_then(|lib| lib.get(idx)))
    }

    // ========== 收藏操作 ==========

    /// 检查歌曲是否已收藏
    pub fn is_favorite(&self, id: &Arc<Uuid>) -> bool {
        self.favorite_ids.contains(id)
    }

    /// 添加歌曲到收藏（如果已存在则不重复添加）
    /// 返回 true 表示成功添加，false 表示已存在
    pub fn add_to_favorites(&mut self, id: &Arc<Uuid>, cx: &mut Context<Self>) -> bool {
        if self.favorite_ids.contains(id) {
            return false;
        }

        if let Some(item) = self.get_by_id(id).cloned() {
            let mut new_favorites = self
                .favorites
                .as_ref()
                .map(|f| (**f).clone())
                .unwrap_or_default();
            new_favorites.push(item);
            self.favorites = Some(Arc::new(new_favorites));
            self.favorite_ids.insert(Arc::clone(id));
            cx.emit(LibraryEvent::FavoriteAdded(Arc::clone(id)));
            cx.notify();
            true
        } else {
            false
        }
    }

    /// 从收藏中移除歌曲
    pub fn remove_from_favorites(&mut self, id: &Arc<Uuid>, cx: &mut Context<Self>) -> bool {
        if !self.favorite_ids.remove(id) {
            return false;
        }

        if let Some(favorites) = &self.favorites {
            let new_favorites: Vec<AlbumInfo> = favorites
                .iter()
                .filter(|item| &item.id() != id)
                .cloned()
                .collect();
            self.favorites = if new_favorites.is_empty() {
                None
            } else {
                Some(Arc::new(new_favorites))
            };
        }

        cx.emit(LibraryEvent::FavoriteRemoved(Arc::clone(id)));
        cx.notify();
        true
    }

    /// 切换收藏状态
    pub fn toggle_favorite(&mut self, id: &Arc<Uuid>, cx: &mut Context<Self>) -> bool {
        if self.is_favorite(id) {
            self.remove_from_favorites(id, cx)
        } else {
            self.add_to_favorites(id, cx)
        }
    }

    // ========== 历史记录操作 ==========

    /// 添加歌曲到历史记录
    /// 如果已存在，会将其移动到最前面
    pub fn add_to_history(&mut self, id: &Arc<Uuid>, cx: &mut Context<Self>) -> bool {
        if let Some(item) = self.get_by_id(id).cloned() {
            let mut new_history: Vec<AlbumInfo> = if self.history_ids.contains(id) {
                // 如果已存在，先移除旧记录
                self.history
                    .as_ref()
                    .map(|h| h.iter().filter(|i| &i.id() != id).cloned().collect())
                    .unwrap_or_default()
            } else {
                self.history_ids.insert(Arc::clone(id));
                self.history
                    .as_ref()
                    .map(|h| (**h).clone())
                    .unwrap_or_default()
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

            self.history = Some(Arc::new(new_history));
            cx.emit(LibraryEvent::HistoryAdded(Arc::clone(id)));
            cx.notify();
            true
        } else {
            false
        }
    }

    /// 清空历史记录
    pub fn clear_history(&mut self, cx: &mut Context<Self>) {
        self.history = None;
        self.history_ids.clear();
        cx.notify();
    }

    // ========== 曲库更新 ==========

    /// 更新曲库（添加新歌曲后调用）
    pub fn update_library(&mut self, new_library: Vec<AlbumInfo>, cx: &mut Context<Self>) {
        if new_library.is_empty() {
            self.library = None;
            self.library_index.clear();
        } else {
            // 重建索引
            let library_index: std::collections::HashMap<Arc<Uuid>, usize> = new_library
                .iter()
                .enumerate()
                .map(|(i, item)| (item.id(), i))
                .collect();

            self.library = Some(Arc::new(new_library));
            self.library_index = library_index;
        }

        cx.emit(LibraryEvent::LibraryUpdated);
        cx.notify();
    }
}
