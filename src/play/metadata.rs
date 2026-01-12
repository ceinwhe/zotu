use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::*;
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
}

impl AlbumInfo {
    pub fn new(path: impl AsRef<Path>, id: Uuid) -> Result<Self, LoftyError> {
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
                    id,
                    title: SharedString::new(title),
                    artist: SharedString::new(artist),
                    album: SharedString::new(album),
                    duration: properties.duration().as_secs(),
                    path: Arc::new(path.to_path_buf()),
                })
            }
            None => {
                let title = title_from_filename;
                let artist = "Unknown Artist";
                let album = "Unknown Album";
                let duration = properties.duration().as_secs();
                Ok(AlbumInfo {
                    id,
                    title: SharedString::new(title),
                    artist: SharedString::new(artist),
                    album: SharedString::new(album),
                    duration,
                    path: Arc::new(path.to_path_buf()),
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

}
