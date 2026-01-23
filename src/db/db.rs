use gpui::{Global, SharedString};
use rusqlite::{Connection, params};
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;
use walkdir::WalkDir;

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
            .prepare_cached("SELECT uuid, title, artist, album, duration, path, cover_path, cover_64 FROM library")
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
                let cover_path: Option<String> = row.get(6)?;
                let cover_64: Option<Vec<u8>> = row.get(7)?;

                let title = SharedString::new(row.get::<_, String>(1)?);
                let artist = SharedString::new(artist.unwrap_or_else(|| "未知艺术家".to_string()));
                let album = SharedString::new(album.unwrap_or_else(|| "未知专辑".to_string()));
                let duration = row.get::<_, i64>(4)? as u64;
                let path = Arc::new(PathBuf::from(row.get::<_, String>(5)?));
                let cover_path = cover_path.map(SharedString::new);
                let cover_64 = cover_64.map(Arc::new);

                Ok(AlbumInfo::new(
                    id, title, artist, album, duration, path, cover_path, cover_64,
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
            "SELECT uuid, title, artist, album, duration, path, cover_path, cover_64 FROM library WHERE uuid = ?",
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
            let cover_path: Option<String> = row.get(6)?;
            let cover_64: Option<Vec<u8>> = row.get(7)?;

            let title = SharedString::new(row.get::<_, String>(1)?);
            let artist = SharedString::new(artist.unwrap_or_else(|| "未知艺术家".to_string()));
            let album = SharedString::new(album.unwrap_or_else(|| "未知专辑".to_string()));
            let duration = row.get::<_, i64>(4)? as u64;
            let path = Arc::new(PathBuf::from(row.get::<_, String>(5)?));
            let cover_path = cover_path.map(SharedString::new);
            let cover_64 = cover_64.map(Arc::new);
            Ok(Some(AlbumInfo::new(
                id, title, artist, album, duration, path, cover_path, cover_64,
            )))
        } else {
            Ok(None)
        }
    }

    pub fn add_to_table(&self, table: &str, id: Uuid) -> rusqlite::Result<()> {
        let mut stmt = self
            .conn
            .prepare_cached(&format!("INSERT INTO {} (uuid) VALUES (?)", table))?;
        stmt.execute(params![id.as_bytes().as_slice()])?;
        Ok(())
    }

    /// 分页加载专辑（适用于大数据量场景）
    pub fn load_albums_paginated(
        &self,
        offset: i64,
        limit: i64,
    ) -> rusqlite::Result<Vec<AlbumInfo>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT uuid, title, artist, album, duration, path, cover_path, cover_64 FROM library LIMIT ? OFFSET ?"
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
            let cover_path: Option<String> = row.get(6)?;
            let cover_64: Option<Vec<u8>> = row.get(7)?;

            let title = SharedString::new(row.get::<_, String>(1)?);
            let artist = SharedString::new(artist.unwrap_or_else(|| "未知艺术家".to_string()));
            let album = SharedString::new(album.unwrap_or_else(|| "未知专辑".to_string()));
            let duration = row.get::<_, i64>(4)? as u64;
            let path = Arc::new(PathBuf::from(row.get::<_, String>(5)?));
            let cover_path = cover_path.map(SharedString::new);
            let cover_64 = cover_64.map(Arc::new);

            Ok(AlbumInfo::new(
                id, title, artist, album, duration, path, cover_path, cover_64,
            ))
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

    pub fn add_metadata_to_library(
        &self,
        folder_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 定义支持的音频文件扩展名
        let audio_extensions = ["mp3", "flac", "wav", "m4a", "ogg", "aac", "vorbis"];

        // 遍历文件夹获取所有音频文件
        let audio_files = self.get_audio_files(folder_path, &audio_extensions)?;

        // 批量处理音频文件
        for file_path in audio_files {
            if let Err(e) = self.process_single_audio_file(&file_path) {
                eprintln!("处理文件 {:?} 时出错: {}", file_path, e);
                // 继续处理其他文件，不中断整个流程
            }
        }

        Ok(())
    }

    /// 获取指定文件夹下的所有音频文件
    fn get_audio_files(
        &self,
        folder_path: &str,
        extensions: &[&str],
    ) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut audio_files = Vec::new();

        for entry in WalkDir::new(folder_path).into_iter() {
            let entry = entry?;

            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if extensions.contains(&ext_str.to_lowercase().as_str()) {
                            audio_files.push(entry.path().to_path_buf());
                        }
                    }
                }
            }
        }

        Ok(audio_files)
    }

    /// 处理单个音频文件的元数据
    fn process_single_audio_file(
        &self,
        file_path: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 从文件创建 AlbumInfo 结构体
        let album_info = AlbumInfo::new_from_file(file_path, "./assets/covers")?;

        // 从 AlbumInfo 中提取数据
        let cover_path = album_info.cover_path().map(|s| s.to_string());
        let cover_64 = album_info.cover_64().map(|arc| arc.as_ref().clone());

        // 插入数据库
        self.conn.execute(
            "INSERT INTO library (uuid, title, artist, album, duration, path, cover_path, cover_64) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                album_info.id().as_bytes().as_slice(),
                album_info.title().to_string(),
                album_info.artist().to_string(),
                album_info.album().to_string(),
                album_info.duration() as i64,
                album_info.path().to_string_lossy().to_string(),
                cover_path,
                cover_64
            ],
        )?;

        Ok(())
    }
}
