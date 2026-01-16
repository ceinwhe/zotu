use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use gpui::SharedString;
use lofty::prelude::{Accessor, TaggedFileExt};
use lofty::{error::LoftyError, file::AudioFile, read_from_path};
use uuid::Uuid;

#[derive(Clone)]
pub struct AlbumInfo {
    pub id: Uuid,
    title: SharedString,
    artist: SharedString,
    album: SharedString,
    duration: u64,
    path: Arc<PathBuf>,
    cover: Option<Arc<Vec<u8>>>,
}

impl AlbumInfo {
    pub fn new(
        id: Uuid,
        title: SharedString,
        artist: SharedString,
        album: SharedString,
        duration: u64,
        path: Arc<PathBuf>,
        cover: Option<Arc<Vec<u8>>>,
    ) -> Self {
        AlbumInfo {
            id,
            title,
            artist,
            album,
            duration,
            path,
            cover,
        }
    }

    /// todo uuid生成策略
    pub fn new_from_file(path: impl AsRef<Path>) -> Result<Self, LoftyError> {
        let uuid=Uuid::new_v4();

        let path = path.as_ref();

        // 读取 tags + properties（时长/比特率等）
        let tagged_file = read_from_path(path)?;
        let properties = tagged_file.properties();

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        let title_from_filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();

        match tag {
            Some(tag) => {
                let title = tag.title().unwrap_or(Cow::Borrowed(title_from_filename));
                let artist = tag.artist().unwrap_or(Cow::Borrowed("未知艺术家"));
                let album = tag.album().unwrap_or(Cow::Borrowed("未知专辑"));

                Ok(AlbumInfo {
                    id: uuid,
                    title: SharedString::new(title),
                    artist: SharedString::new(artist),
                    album: SharedString::new(album),
                    duration: properties.duration().as_secs(),
                    path: Arc::new(path.to_path_buf()),
                    cover: None, // 从文件创建时不加载封面，由数据库加载
                })
            }
            None => {
                let title = title_from_filename;
                let artist = "Unknown Artist";
                let album = "Unknown Album";
                let duration = properties.duration().as_secs();
                Ok(AlbumInfo {
                    id: uuid,
                    title: SharedString::new(title),
                    artist: SharedString::new(artist),
                    album: SharedString::new(album),
                    duration,
                    path: Arc::new(path.to_path_buf()),
                    cover: None, // 从文件创建时不加载封面，由数据库加载
                })
            }
        }
    }

    pub fn title(&self) -> SharedString {
        SharedString::clone(&self.title)
    }

    pub fn artist(&self) -> SharedString {
        SharedString::clone(&self.artist)
    }

    pub fn album(&self) -> SharedString {
        SharedString::clone(&self.album)
    }

    /// 单位：秒
    pub fn duration(&self) -> u64 {
        self.duration
    }

    pub fn path(&self) -> Arc<PathBuf> {
        Arc::clone(&self.path)
    }

    pub fn cover(&self) -> Option<Arc<Vec<u8>>> {
        self.cover.as_ref().map(Arc::clone)
    }
}
