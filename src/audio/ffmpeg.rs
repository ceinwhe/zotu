pub mod decoder;
pub mod ffi;
pub mod resampler;

use super::engine::{AudioEngine, EngineEvent};
use super::output::CpalOutput;
use decoder::FfmpegDecoder;
use resampler::FfmpegResampler;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct FfmpegEngine {
    command_tx: std::sync::mpsc::Sender<Command>,
    event_rx: std::sync::mpsc::Receiver<EngineEvent>,
    state: Arc<Mutex<EngineState>>,
}

struct EngineState {
    is_playing: bool,
    is_paused: bool,
    current_position_secs: u64,
    duration_secs: Option<u64>,
}

enum Command {
    Play(String),
    Pause,
    Resume,
    Stop,
    Seek(u64),
    Shutdown,
}

impl FfmpegEngine {
    pub fn new() -> anyhow::Result<Self> {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let (evt_tx, evt_rx) = std::sync::mpsc::channel();
        let state = Arc::new(Mutex::new(EngineState {
            is_playing: false,
            is_paused: false,
            current_position_secs: 0,
            duration_secs: None,
        }));

        let state_clone = Arc::clone(&state);
        thread::spawn(move || {
            engine_thread(cmd_rx, evt_tx, state_clone);
        });

        Ok(Self {
            command_tx: cmd_tx,
            event_rx: evt_rx,
            state,
        })
    }
}

fn engine_thread(
    cmd_rx: std::sync::mpsc::Receiver<Command>,
    evt_tx: std::sync::mpsc::Sender<EngineEvent>,
    state: Arc<Mutex<EngineState>>,
) {
    let mut decoder: Option<FfmpegDecoder> = None;
    let mut resampler: Option<FfmpegResampler> = None;
    let mut output: Option<CpalOutput> = None;
    let audio_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let mut paused = false;

    loop {
        match cmd_rx.try_recv() {
            Ok(Command::Play(path)) => {
                match FfmpegDecoder::open(Path::new(&path)) {
                    Ok(dec) => {
                        let out_rate = 44100;
                        let out_channels = 2;

                        let res = FfmpegResampler::new(
                            dec.format.sample_rate,
                            dec.format.channels,
                            dec.format.sample_fmt,
                            out_rate,
                            out_channels,
                        );

                        match res {
                            Ok(r) => {
                                let mut out =
                                    CpalOutput::new(out_rate as u32, out_channels as u16).ok();
                                if let Some(ref mut o) = out {
                                    let _ = o.start(Arc::clone(&audio_buffer));
                                }

                                {
                                    let mut s = state.lock().unwrap();
                                    s.is_playing = true;
                                    s.is_paused = false;
                                    s.duration_secs = Some(dec.duration_secs());
                                    s.current_position_secs = 0;
                                }

                                decoder = Some(dec);
                                resampler = Some(r);
                                output = out;
                                paused = false;
                            }
                            Err(e) => {
                                let _ = evt_tx.send(EngineEvent::Error(e.to_string()));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = evt_tx.send(EngineEvent::Error(e.to_string()));
                    }
                }
            }
            Ok(Command::Pause) => {
                paused = true;
                let mut s = state.lock().unwrap();
                s.is_paused = true;
                s.is_playing = false;
            }
            Ok(Command::Resume) => {
                paused = false;
                let mut s = state.lock().unwrap();
                s.is_paused = false;
                s.is_playing = true;
            }
            Ok(Command::Stop) => {
                decoder = None;
                resampler = None;
                output = None;
                let mut s = state.lock().unwrap();
                s.is_playing = false;
                s.is_paused = false;
                s.current_position_secs = 0;
            }
            Ok(Command::Seek(pos)) => {
                if let Some(ref mut dec) = decoder {
                    let _ = dec.seek(pos);
                    let mut s = state.lock().unwrap();
                    s.current_position_secs = pos;
                }
            }
            Ok(Command::Shutdown) => {
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
        }

        if !paused {
            if let (Some(ref mut dec), Some(ref mut res)) = (&mut decoder, &mut resampler) {
                if !dec.is_finished() {
                    match dec.read_frame() {
                        Ok(Some((data, nb_samples))) => {
                            let out_samples = nb_samples * 2;
                            let mut out_buf = vec![0f32; out_samples * 2];
                            let out_ptr = out_buf.as_mut_ptr() as *mut u8;
                            let in_ptr = data as *const u8;

                            match res.resample(
                                &in_ptr,
                                nb_samples as i32,
                                &mut (out_ptr as *mut u8),
                                out_samples as i32,
                            ) {
                                Ok(written) => {
                                    let samples = written as usize * 2;
                                    let mut buf = audio_buffer.lock().unwrap();
                                    buf.extend_from_slice(&out_buf[..samples]);

                                    let sample_rate = res.out_sample_rate() as u64;
                                    if sample_rate > 0 {
                                        let mut s = state.lock().unwrap();
                                        s.current_position_secs += written as u64 / sample_rate;
                                    }
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(EngineEvent::Error(e.to_string()));
                                }
                            }
                        }
                        Ok(None) => {
                            let _ = evt_tx.send(EngineEvent::TrackFinished);
                            decoder = None;
                            resampler = None;
                            let mut s = state.lock().unwrap();
                            s.is_playing = false;
                        }
                        Err(e) => {
                            let _ = evt_tx.send(EngineEvent::Error(e.to_string()));
                        }
                    }
                }
            }
        }

        thread::sleep(std::time::Duration::from_millis(10));
    }
}

impl AudioEngine for FfmpegEngine {
    fn play(&mut self, path: &Path) -> anyhow::Result<()> {
        self.command_tx
            .send(Command::Play(path.to_string_lossy().to_string()))
            .map_err(|_| anyhow::anyhow!("Engine thread disconnected"))?;
        Ok(())
    }
    fn pause(&mut self) {
        let _ = self.command_tx.send(Command::Pause);
    }
    fn resume(&mut self) {
        let _ = self.command_tx.send(Command::Resume);
    }
    fn stop(&mut self) {
        let _ = self.command_tx.send(Command::Stop);
    }
    fn seek(&mut self, position_secs: u64) -> anyhow::Result<()> {
        self.command_tx
            .send(Command::Seek(position_secs))
            .map_err(|_| anyhow::anyhow!("Engine thread disconnected"))?;
        Ok(())
    }
    fn is_playing(&self) -> bool {
        self.state.lock().unwrap().is_playing
    }
    fn is_paused(&self) -> bool {
        self.state.lock().unwrap().is_paused
    }
    fn current_position_secs(&self) -> u64 {
        self.state.lock().unwrap().current_position_secs
    }
    fn duration_secs(&self) -> Option<u64> {
        self.state.lock().unwrap().duration_secs
    }
    fn poll_events(&mut self) -> Vec<EngineEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.event_rx.try_recv() {
            events.push(evt);
        }
        events
    }
}

impl Drop for FfmpegEngine {
    fn drop(&mut self) {
        let _ = self.command_tx.send(Command::Shutdown);
    }
}
