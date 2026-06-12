use std::path::Path;

pub enum EngineEvent {
    TrackFinished,
    Error(String),
}

pub trait AudioEngine: Send {
    fn play(&mut self, path: &Path) -> anyhow::Result<()>;
    fn pause(&mut self);
    fn resume(&mut self);
    fn stop(&mut self);
    fn seek(&mut self, position_secs: u64) -> anyhow::Result<()>;
    fn is_playing(&self) -> bool;
    fn is_paused(&self) -> bool;
    fn current_position_secs(&self) -> u64;
    fn duration_secs(&self) -> Option<u64>;
    fn poll_events(&mut self) -> Vec<EngineEvent>;
}
