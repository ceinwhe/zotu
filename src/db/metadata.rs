use gpui::SharedString;
use image::{ExtendedColorType, codecs::jpeg::JpegEncoder, imageops::FilterType, load_from_memory};
use lofty::{
    file::AudioFile,
    picture::MimeType,
    prelude::{Accessor, TaggedFileExt},
    read_from_path,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
/// 音乐专辑元信息,包含标题、艺术家、专辑名、时长、文件路径及封面等
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AlbumInfo {
    id: Uuid,
    title: SharedString,
    artist: SharedString,
    album: SharedString,
    duration: u64,
    path: Arc<PathBuf>,
    cover_path: Option<SharedString>,
    cover_64: Option<Arc<Vec<u8>>>,
}

impl AlbumInfo {
    /// 创建一个新的 AlbumInfo 实例
    pub fn new(
        id: Uuid,
        title: SharedString,
        artist: SharedString,
        album: SharedString,
        duration: u64,
        path: Arc<PathBuf>,
        cover_path: Option<SharedString>,
        cover_64: Option<Arc<Vec<u8>>>,
    ) -> Self {
        AlbumInfo {
            id,
            title,
            artist,
            album,
            duration,
            path,
            cover_path,
            cover_64,
        }
    }

    /// 从音频文件中读取元信息并创建 AlbumInfo 实例
    pub fn new_from_file(
        source_path: impl AsRef<Path>,
        cover_dir: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();

        let path = source_path.as_ref();
        let cover_dir = cover_dir.as_ref();
        let tagged_file = read_from_path(path)?;
        let props = tagged_file.properties();

        let title_from_filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        // 统一把 title/artist/album 都算出来（无 tag 时走默认值）
        let (title, artist, album) = match tag {
            Some(t) => (
                t.title().unwrap_or(Cow::Borrowed(title_from_filename)),
                t.artist().unwrap_or(Cow::Borrowed("未知艺术家")),
                t.album().unwrap_or(Cow::Borrowed("未知专辑")),
            ),
            None => (
                Cow::Borrowed(title_from_filename),
                Cow::Borrowed("未知艺术家"),
                Cow::Borrowed("未知专辑"),
            ),
        };

        // cover 处理：最终变成 Option<(cover_path, cover_64)>
        let cover_pack: Option<(SharedString, Arc<Vec<u8>>)> = tag
            .and_then(|t| t.pictures().first())
            .map(|cover| -> Result<_, Box<dyn std::error::Error>> {
                let ext = cover
                    .mime_type()
                    .cloned()
                    .unwrap_or(MimeType::Png)
                    .ext()
                    .unwrap_or("png")
                    .to_string();

                // 生成 64x64 jpeg bytes
                let img = load_from_memory(cover.data())?;
                let resized = img.resize_exact(64, 64, FilterType::Lanczos3).to_rgb8();
                let mut cover_64 = Vec::new();
                JpegEncoder::new_with_quality(&mut cover_64, 100).encode(
                    &resized,
                    64,
                    64,
                    ExtendedColorType::Rgb8,
                )?;

                // 落盘原图
                fs::create_dir_all(cover_dir)?;
                let cover_path = cover_dir.join(format!("{id}.{ext}"));
                fs::write(&cover_path, cover.data())?;

                Ok((
                    SharedString::new(cover_path.to_string_lossy()),
                    Arc::new(cover_64),
                ))
            })
            .transpose()?; // Option<Result<T>> -> Result<Option<T>>

        let (cover_path, cover_64) = match cover_pack {
            Some((p, b)) => (Some(p), Some(b)),
            None => (None, None),
        };

        Ok(AlbumInfo {
            id,
            title: SharedString::new(title),
            artist: SharedString::new(artist),
            album: SharedString::new(album),
            duration: props.duration().as_secs(),
            path: Arc::new(path.to_path_buf()),
            cover_path,
            cover_64,
        })
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

    pub fn cover_path(&self) -> Option<SharedString> {
        self.cover_path.clone()
    }

    pub fn cover_64(&self) -> Option<Arc<Vec<u8>>> {
        self.cover_64.as_ref().map(Arc::clone)
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}
