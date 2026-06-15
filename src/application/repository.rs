use crate::db::AlbumInfo;
use uuid::Uuid;

pub trait LibraryRepository {
    fn load_library(&self) -> Vec<AlbumInfo>;
    fn load_favorite_ids(&self) -> Vec<Uuid>;
    fn load_history_ids(&self) -> Vec<Uuid>;
    fn add_favorite(&self, id: &Uuid) -> anyhow::Result<()>;
    fn remove_favorite(&self, id: &Uuid) -> anyhow::Result<()>;
    fn add_history(&self, id: &Uuid) -> anyhow::Result<()>;
    fn scan_directory(&self, path: &str) -> anyhow::Result<()>;
}
