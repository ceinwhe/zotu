use gpui::{SharedString,Global};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use super::metadata::AlbumInfo;

pub struct DB {
    conn: Connection,
}

impl Global for DB {}

impl DB {
    pub fn new(db_path: &str) -> rusqlite::Result<DB> {
        let conn = Connection::open(db_path)?;
        // 启用性能优化
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = 10000;
             PRAGMA temp_store = MEMORY;",
        )?;
        Ok(DB { conn })
    }

    /// 高性能加载所有专辑信息
    /// 使用预编译语句和批量处理优化性能
    pub fn load_all_albums(&self) -> Option<Vec<AlbumInfo>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT uuid, title, artist, album, duration, path, cover FROM library")
            .ok()?;

        let album_iter = stmt
            .query_map([], |row| {
                // 解析 UUID BLOB (16 bytes)
                let uuid_bytes: Vec<u8> = row.get(0)?;
                let id = Uuid::from_slice(&uuid_bytes).map_err(|erroe| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        Box::new(erroe),
                    )
                })?;

                // 处理可能为 NULL 的字段
                let artist: Option<String> = row.get(2)?;
                let album: Option<String> = row.get(3)?;
                let cover: Option<Vec<u8>> = row.get(6)?;

                let title = SharedString::new(row.get::<_, String>(1)?);
                let artist = SharedString::new(artist.unwrap_or_else(|| "未知艺术家".to_string()));
                let album = SharedString::new(album.unwrap_or_else(|| "未知专辑".to_string()));
                let duration = row.get::<_, i64>(4)? as u64;
                let path = Arc::new(PathBuf::from(row.get::<_, String>(5)?));
                let cover = cover.map(|c| Arc::new(c));

                Ok(AlbumInfo::new(
                    id, title, artist, album, duration, path, cover,
                ))
            })
            .ok()?;

        // 预分配容量以减少重新分配
        let mut albums = Vec::with_capacity(1000);
        for album in album_iter {
            albums.push(album.ok()?);
        }
        albums.shrink_to_fit();

        if albums.is_empty() {
            None
        } else {
            Some(albums)
        }
    }

    /// 通过 UUID 查询单个专辑
    pub fn load_album_by_uuid(&self, uuid: &Uuid) -> rusqlite::Result<Option<AlbumInfo>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT uuid, title, artist, album, duration, path, cover FROM library WHERE uuid = ?",
        )?;

        let mut rows = stmt.query(params![uuid.as_bytes().as_slice()])?;

        if let Some(row) = rows.next()? {
            let uuid_bytes: Vec<u8> = row.get(0)?;
            let id = Uuid::from_slice(&uuid_bytes).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Blob,
                    Box::new(e),
                )
            })?;

            let artist: Option<String> = row.get(2)?;
            let album: Option<String> = row.get(3)?;
            let cover: Option<Vec<u8>> = row.get(6)?;

            let title=SharedString::new(row.get::<_, String>(1)?);
            let artist=SharedString::new(artist.unwrap_or_else(|| "未知艺术家".to_string()));
            let album=SharedString::new(album.unwrap_or_else(|| "未知专辑".to_string()));
            let duration=row.get::<_, i64>(4)? as u64;
            let path=Arc::new(PathBuf::from(row.get::<_, String>(5)?));
            let cover=cover.map(|c| Arc::new(c));
            Ok(Some(AlbumInfo::new(id, title, artist, album, duration, path, cover)))
        } else {
            Ok(None)
        }
    }

    /// 分页加载专辑（适用于大数据量场景）
    pub fn load_albums_paginated(
        &self,
        offset: i64,
        limit: i64,
    ) -> rusqlite::Result<Vec<AlbumInfo>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT uuid, title, artist, album, duration, path, cover FROM library LIMIT ? OFFSET ?"
        )?;

        let album_iter = stmt.query_map(params![limit, offset], |row| {
            let uuid_bytes: Vec<u8> = row.get(0)?;
            let id = Uuid::from_slice(&uuid_bytes).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Blob,
                    Box::new(e),
                )
            })?;

            let artist: Option<String> = row.get(2)?;
            let album: Option<String> = row.get(3)?;
            let cover: Option<Vec<u8>> = row.get(6)?;

            let title=SharedString::new(row.get::<_, String>(1)?);
            let artist=SharedString::new(artist.unwrap_or_else(|| "未知艺术家".to_string()));
            let album=SharedString::new(album.unwrap_or_else(|| "未知专辑".to_string()));
            let duration=row.get::<_, i64>(4)? as u64;
            let path=Arc::new(PathBuf::from(row.get::<_, String>(5)?));
            let cover=cover.map(|c| Arc::new(c));

            Ok(AlbumInfo::new(id, title, artist, album, duration, path, cover))
        })?;

        let mut albums = Vec::with_capacity(limit as usize);
        for album in album_iter {
            albums.push(album?);
        }
        Ok(albums)
    }

    /// 获取专辑总数
    pub fn get_album_count(&self) -> rusqlite::Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM library", [], |row| row.get(0))
    }
}
