mod decoder;
mod ffi;
mod resampler;

use super::engine::{AudioEngine, EngineEvent};
use super::output::{CpalOutput, SharedAudioBuffer};
use decoder::FfmpegDecoder;
use resampler::FfmpegResampler;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

const TARGET_BUFFER_MILLIS: usize = 200;

pub struct FfmpegEngine {
    command_tx: std::sync::mpsc::Sender<Command>,
    event_rx: std::sync::mpsc::Receiver<EngineEvent>,
    state: Arc<Mutex<EngineState>>,
    worker: Option<JoinHandle<()>>,
}

struct EngineState {
    is_playing: bool,
    is_paused: bool,
    current_position_secs: u64,
    duration_secs: Option<u64>,
}

enum Command {
    Play(PathBuf),
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
        let worker = thread::Builder::new()
            .name("zotu-ffmpeg".to_string())
            .spawn(move || engine_thread(cmd_rx, evt_tx, state_clone))?;

        Ok(Self {
            command_tx: cmd_tx,
            event_rx: evt_rx,
            state,
            worker: Some(worker),
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
    let audio_buffer: SharedAudioBuffer = Arc::new(Mutex::new(VecDeque::new()));
    let mut paused = false;
    let mut decode_finished = false;
    let mut position_base_secs = 0u64;
    let mut written_frames = 0u64;

    'engine: loop {
        while let Ok(command) = cmd_rx.try_recv() {
            match command {
                Command::Play(path) => {
                    decoder = None;
                    resampler = None;
                    output = None;
                    clear_audio_buffer(&audio_buffer);
                    decode_finished = false;
                    position_base_secs = 0;
                    written_frames = 0;

                    match create_playback(&path, Arc::clone(&audio_buffer)) {
                        Ok((dec, res, out)) => {
                            let duration = dec.duration_secs();
                            decoder = Some(dec);
                            resampler = Some(res);
                            output = Some(out);
                            paused = false;

                            let mut s = state.lock().unwrap();
                            s.is_playing = true;
                            s.is_paused = false;
                            s.duration_secs = Some(duration);
                            s.current_position_secs = 0;
                        }
                        Err(error) => {
                            set_stopped(&state, true);
                            let _ = evt_tx.send(EngineEvent::Error(error.to_string()));
                        }
                    }
                }
                Command::Pause => {
                    paused = true;
                    if let Some(out) = &output
                        && let Err(error) = out.pause()
                    {
                        let _ = evt_tx.send(EngineEvent::Error(error.to_string()));
                    }
                    let mut s = state.lock().unwrap();
                    s.is_paused = true;
                    s.is_playing = false;
                }
                Command::Resume => {
                    if decoder.is_some() || decode_finished {
                        paused = false;
                        if let Some(out) = &output
                            && let Err(error) = out.resume()
                        {
                            let _ = evt_tx.send(EngineEvent::Error(error.to_string()));
                        }
                        let mut s = state.lock().unwrap();
                        s.is_paused = false;
                        s.is_playing = true;
                    }
                }
                Command::Stop => {
                    decoder = None;
                    resampler = None;
                    output = None;
                    clear_audio_buffer(&audio_buffer);
                    paused = false;
                    decode_finished = false;
                    position_base_secs = 0;
                    written_frames = 0;
                    set_stopped(&state, false);
                }
                Command::Seek(position_secs) => {
                    if let (Some(dec), Some(out)) = (&mut decoder, &output) {
                        let seek_result = dec.seek(position_secs).and_then(|()| {
                            FfmpegResampler::new(
                                dec.format(),
                                out.sample_rate() as i32,
                                out.channels() as i32,
                            )
                        });

                        match seek_result {
                            Ok(new_resampler) => {
                                clear_audio_buffer(&audio_buffer);
                                resampler = Some(new_resampler);
                                decode_finished = false;
                                position_base_secs = position_secs;
                                written_frames = 0;
                                state.lock().unwrap().current_position_secs = position_secs;
                            }
                            Err(error) => {
                                let _ = evt_tx.send(EngineEvent::Error(error.to_string()));
                            }
                        }
                    }
                }
                Command::Shutdown => break 'engine,
            }
        }

        if paused {
            thread::sleep(std::time::Duration::from_millis(5));
            continue;
        }

        if decode_finished {
            if audio_buffer.lock().unwrap().is_empty() {
                decode_finished = false;
                output = None;
                set_stopped(&state, false);
                let _ = evt_tx.send(EngineEvent::TrackFinished);
            } else {
                thread::sleep(std::time::Duration::from_millis(2));
            }
            continue;
        }

        let Some(out) = &output else {
            thread::sleep(std::time::Duration::from_millis(10));
            continue;
        };

        let target_buffer_samples =
            out.sample_rate() as usize * out.channels() as usize * TARGET_BUFFER_MILLIS / 1000;
        if audio_buffer.lock().unwrap().len() >= target_buffer_samples {
            thread::sleep(std::time::Duration::from_millis(2));
            continue;
        }

        let mut reached_eof = false;
        let mut decode_error = None;
        if let (Some(dec), Some(res)) = (&mut decoder, &mut resampler) {
            match dec.read_frame() {
                Ok(Some(frame)) => {
                    let result = (|| -> anyhow::Result<()> {
                        let converted = res.convert(&frame)?;
                        audio_buffer.lock().unwrap().extend(converted.samples);

                        written_frames += converted.frame_count as u64;
                        let sample_rate = res.output_sample_rate() as u64;
                        if let Some(elapsed_secs) = written_frames.checked_div(sample_rate) {
                            state.lock().unwrap().current_position_secs =
                                position_base_secs + elapsed_secs;
                        }
                        Ok(())
                    })();

                    if let Err(error) = result {
                        decode_error = Some(error.to_string());
                    }
                }
                Ok(None) => reached_eof = true,
                Err(error) => decode_error = Some(error.to_string()),
            }
        }

        if reached_eof {
            decoder = None;
            resampler = None;
            decode_finished = true;
        }

        if let Some(error) = decode_error {
            decoder = None;
            resampler = None;
            output = None;
            clear_audio_buffer(&audio_buffer);
            set_stopped(&state, false);
            let _ = evt_tx.send(EngineEvent::Error(error));
        }
    }
}

fn create_playback(
    path: &Path,
    audio_buffer: SharedAudioBuffer,
) -> anyhow::Result<(FfmpegDecoder, FfmpegResampler, CpalOutput)> {
    let decoder = FfmpegDecoder::open(path)?;
    let preferred_rate = decoder.format().sample_rate().max(1) as u32;
    let mut output = CpalOutput::new(preferred_rate, 2)?;
    let resampler = FfmpegResampler::new(
        decoder.format(),
        output.sample_rate() as i32,
        output.channels() as i32,
    )?;
    output.start(audio_buffer)?;
    Ok((decoder, resampler, output))
}

fn clear_audio_buffer(audio_buffer: &SharedAudioBuffer) {
    audio_buffer.lock().unwrap().clear();
}

fn set_stopped(state: &Arc<Mutex<EngineState>>, clear_duration: bool) {
    let mut state = state.lock().unwrap();
    state.is_playing = false;
    state.is_paused = false;
    state.current_position_secs = 0;
    if clear_duration {
        state.duration_secs = None;
    }
}

impl AudioEngine for FfmpegEngine {
    fn play(&mut self, path: &Path) -> anyhow::Result<()> {
        self.command_tx
            .send(Command::Play(path.to_path_buf()))
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
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
