use crate::db::metadata::AlbumInfo;
use gpui::*;
use std::sync::Arc;

pub struct Cover {
    pub album: Option<Arc<AlbumInfo>>,
}

impl Render for Cover {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}
