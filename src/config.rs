use gpui::{Global, SharedString};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

use crate::{audio::playlist::LoopMode, db::AlbumInfo};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub media_file: MediaFile,
    pub play_info: PlayInfo,
}

impl Global for Config {}

impl Default for Config {
    fn default() -> Self {
        Config {
            media_file: MediaFile::default(),
            play_info: PlayInfo::default(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct MediaFile {
    pub music_directory: SharedString,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayInfo {
    pub loop_mode: LoopMode,
    pub volume: f32,
    pub album: Option<AlbumInfo>,
}

impl Default for MediaFile {
    fn default() -> Self {
        MediaFile {
            music_directory: SharedString::from("C:/Users/ceinw/OneDrive/Desktop/Music"),
        }
    }
}

impl Default for PlayInfo {
    fn default() -> Self {
        PlayInfo {
            loop_mode: LoopMode::List,
            volume: 0.5,
            album: None,
        }
    }
}

impl Config {
    pub const PATH: &'static str = "config.toml";

    /// 读取配置：文件存在就读；不存在就用默认并写入一份
    pub fn load_or_create(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        match fs::read_to_string(path) {
            Ok(text) => {
                let cfg = toml::from_str(&text)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(cfg)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let cfg = Self::default();
                cfg.save(path)?;
                Ok(cfg)
            }
            Err(e) => Err(e),
        }
    }

    /// 写入配置（覆盖写）
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }

        let text = toml::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, text)
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use uuid::Uuid;

    use super::*;

    #[test]
    fn config_round_trips_as_toml() {
        let mut config = Config::default();
        config.play_info.album = Some(AlbumInfo::new(
            Uuid::new_v4(),
            SharedString::from("Track"),
            SharedString::from("Artist"),
            SharedString::from("Album"),
            180,
            Arc::new(PathBuf::from("music/track.mp3")),
            None,
            None,
        ));
        let text = toml::to_string_pretty(&config).expect("serialize config as TOML");
        let decoded: Config = toml::from_str(&text).expect("deserialize config from TOML");

        assert!(text.contains("[media_file]"));
        assert!(text.contains("[play_info]"));
        assert_eq!(
            decoded.media_file.music_directory,
            config.media_file.music_directory
        );
        assert!((decoded.play_info.volume - config.play_info.volume).abs() < f32::EPSILON);
        assert_eq!(
            decoded
                .play_info
                .album
                .as_ref()
                .expect("album is preserved")
                .title(),
            "Track"
        );
    }
}
