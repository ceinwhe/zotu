use gpui::{Global,SharedString};
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::{fs, io, path::Path};

use crate::{db::metadata::AlbumInfo,play::player::LoopMode};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub media_file: MediaFile,
    pub play_info: PlayInfo,
}

impl Global for Config{}

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
pub struct PlayInfo{
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
    /// 读取配置：文件存在就读；不存在就用默认并写入一份
    pub fn load_or_create(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        match fs::read_to_string(path) {
            Ok(text) => {
                let cfg: Self =
                    serde_yaml::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let text =
            serde_yaml::to_string(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, text)
    }

    /// 修改配置：传入一个闭包对 cfg 做修改，然后立刻保存
    pub fn update<F>(path: impl AsRef<Path>, f: F) -> io::Result<Self>
    where
        F: FnOnce(&mut Self),
    {
        let path = path.as_ref();
        let mut cfg = Self::load_or_create(path)?;
        f(&mut cfg);
        cfg.save(path)?;
        Ok(cfg)
    }
}