use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, TryRecvError, sync_channel},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{errors::IpcError, recording::AudioSource};

use super::{com::ComApartment, stream::AudioStream};

// Leave room for short Media Foundation and UI startup stalls without blocking WASAPI reads.
const AUDIO_QUEUE_CAPACITY: usize = 512;

#[derive(Debug)]
pub enum AudioMessage {
    Samples(AudioSample),
    Failed(IpcError),
}

#[derive(Debug)]
pub struct AudioSample {
    pub source: AudioSource,
    pub timestamp_100ns: i64,
    pub discontinuity: bool,
    pub frame_count: u32,
    pub pcm_s16le: Vec<u8>,
}

enum AudioWorkerCommand {
    SetEnabled {
        enabled: bool,
        response: SyncSender<Result<(), IpcError>>,
    },
    Shutdown,
}

struct AudioWorker {
    source: AudioSource,
    commands: SyncSender<AudioWorkerCommand>,
    thread: JoinHandle<Result<(), IpcError>>,
}

pub struct AudioCapture {
    packets: Receiver<AudioMessage>,
    shutdown: Arc<AtomicBool>,
    workers: Vec<AudioWorker>,
}

impl AudioCapture {
    pub fn start(
        microphone_device_id: Option<&str>,
        microphone: bool,
        system: bool,
    ) -> Result<Self, IpcError> {
        let (packet_tx, packets) = sync_channel(AUDIO_QUEUE_CAPACITY);
        let shutdown = Arc::new(AtomicBool::new(false));
        let mut capture = Self {
            packets,
            shutdown,
            workers: Vec::new(),
        };

        if microphone {
            capture.start_worker(
                AudioSource::Microphone,
                microphone_device_id.map(str::to_owned),
                packet_tx.clone(),
            )?;
        }
        if system {
            capture.start_worker(AudioSource::System, None, packet_tx)?;
        }
        Ok(capture)
    }

    pub fn drain(&self) -> Result<Vec<AudioSample>, IpcError> {
        let mut samples = Vec::with_capacity(AUDIO_QUEUE_CAPACITY);
        loop {
            match self.packets.try_recv() {
                Ok(AudioMessage::Samples(sample)) => samples.push(sample),
                Ok(AudioMessage::Failed(error)) => return Err(error),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return Ok(samples),
            }
        }
    }

    pub fn set_enabled(&self, source: AudioSource, enabled: bool) -> Result<(), IpcError> {
        let worker = self
            .workers
            .iter()
            .find(|worker| worker.source == source)
            .ok_or_else(|| {
                IpcError::new(
                    "audio_source_unavailable",
                    "This audio source was not enabled when the recording started.",
                    true,
                )
            })?;
        let (response_tx, response_rx) = sync_channel(1);
        worker
            .commands
            .send(AudioWorkerCommand::SetEnabled {
                enabled,
                response: response_tx,
            })
            .map_err(|_| audio_worker_stopped(source))?;
        response_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| audio_worker_stopped(source))?
    }

    pub fn stop(mut self) -> Result<Vec<AudioSample>, IpcError> {
        self.shutdown.store(true, Ordering::Release);
        for worker in &self.workers {
            let _ = worker.commands.send(AudioWorkerCommand::Shutdown);
        }

        let mut samples = Vec::with_capacity(AUDIO_QUEUE_CAPACITY);
        let mut first_error = None;
        while self
            .workers
            .iter()
            .any(|worker| !worker.thread.is_finished())
        {
            self.drain_into(&mut samples, &mut first_error);
            thread::sleep(Duration::from_millis(5));
        }
        for worker in self.workers.drain(..) {
            match worker.thread.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    first_error.get_or_insert(error);
                }
                Err(_) => {
                    first_error.get_or_insert_with(|| {
                        IpcError::new(
                            "audio_worker_panicked",
                            "An audio capture worker stopped unexpectedly.",
                            true,
                        )
                    });
                }
            }
        }
        self.drain_into(&mut samples, &mut first_error);
        match first_error {
            Some(error) => Err(error),
            None => Ok(samples),
        }
    }

    fn start_worker(
        &mut self,
        source: AudioSource,
        device_id: Option<String>,
        packet_tx: SyncSender<AudioMessage>,
    ) -> Result<(), IpcError> {
        let (command_tx, command_rx) = sync_channel(2);
        let (ready_tx, ready_rx) = sync_channel(1);
        let shutdown = self.shutdown.clone();
        let thread = thread::Builder::new()
            .name(format!("recodex-audio-{source:?}").to_lowercase())
            .spawn(move || {
                let mut ready = Some(ready_tx);
                let result = capture_audio(
                    source,
                    device_id.as_deref(),
                    command_rx,
                    packet_tx,
                    shutdown,
                    &mut ready,
                );
                if let Err(error) = &result
                    && let Some(response) = ready.take()
                {
                    let _ = response.send(Err(error.clone()));
                }
                result
            })
            .map_err(|error| {
                IpcError::new(
                    "audio_worker_start_failed",
                    format!("Could not start the {source:?} capture worker: {error}"),
                    true,
                )
            })?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                self.workers.push(AudioWorker {
                    source,
                    commands: command_tx,
                    thread,
                });
                Ok(())
            }
            Ok(Err(error)) => {
                self.shutdown.store(true, Ordering::Release);
                let _ = thread.join();
                Err(error)
            }
            Err(_) => {
                self.shutdown.store(true, Ordering::Release);
                let _ = thread.join();
                Err(IpcError::new(
                    "audio_start_timeout",
                    format!("The {source:?} audio device did not become ready in time."),
                    true,
                ))
            }
        }
    }

    fn drain_into(&self, samples: &mut Vec<AudioSample>, first_error: &mut Option<IpcError>) {
        loop {
            match self.packets.try_recv() {
                Ok(AudioMessage::Samples(sample)) => samples.push(sample),
                Ok(AudioMessage::Failed(error)) => {
                    first_error.get_or_insert(error);
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
            }
        }
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        for worker in &self.workers {
            let _ = worker.commands.send(AudioWorkerCommand::Shutdown);
        }
        while self
            .workers
            .iter()
            .any(|worker| !worker.thread.is_finished())
        {
            while self.packets.try_recv().is_ok() {}
            thread::sleep(Duration::from_millis(5));
        }
        for worker in self.workers.drain(..) {
            let _ = worker.thread.join();
        }
    }
}

fn capture_audio(
    source: AudioSource,
    device_id: Option<&str>,
    commands: Receiver<AudioWorkerCommand>,
    packets: SyncSender<AudioMessage>,
    shutdown: Arc<AtomicBool>,
    ready: &mut Option<SyncSender<Result<(), IpcError>>>,
) -> Result<(), IpcError> {
    let _apartment = ComApartment::initialize()?;
    #[cfg(debug_assertions)]
    {
        let endpoint_name = super::devices::resolve_endpoint_name(source, device_id)
            .unwrap_or_else(|| "unknown Windows endpoint".to_owned());
        let classification = if source == AudioSource::Microphone
            && super::devices::is_playback_mix_endpoint(&endpoint_name)
        {
            " (playback-mix input; it can contain desktop audio)"
        } else {
            ""
        };
        eprintln!(
            "[RecoDex][audio] opening {source:?} WASAPI endpoint: {endpoint_name}{classification}"
        );
    }
    let mut stream = AudioStream::open(source, device_id)?;
    ready
        .take()
        .ok_or_else(|| IpcError::internal("The audio readiness response was already sent."))?
        .send(Ok(()))
        .map_err(|_| audio_worker_stopped(source))?;

    let mut enabled = true;
    let result = 'capture: loop {
        if shutdown.load(Ordering::Acquire) {
            break Ok(());
        }
        loop {
            match commands.try_recv() {
                Ok(AudioWorkerCommand::Shutdown) => break 'capture Ok(()),
                Ok(AudioWorkerCommand::SetEnabled {
                    enabled: next,
                    response,
                }) => {
                    let update = stream.set_enabled(next);
                    if update.is_ok() {
                        enabled = next;
                    }
                    let _ = response.send(update);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break 'capture Ok(()),
            }
        }
        if !enabled {
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        let data_ready = match stream.wait_for_data() {
            Ok(data_ready) => data_ready,
            Err(error) => break Err(error),
        };
        if !data_ready {
            continue;
        }
        loop {
            let sample = match stream.next_sample(source) {
                Ok(Some(sample)) => sample,
                Ok(None) => break,
                Err(error) => break 'capture Err(error),
            };
            if packets.send(AudioMessage::Samples(sample)).is_err() {
                break 'capture Ok(());
            }
        }
    };

    if enabled && let Err(error) = stream.set_enabled(false) {
        let _ = packets.send(AudioMessage::Failed(error.clone()));
        return Err(error);
    }
    if let Err(error) = result {
        let _ = packets.send(AudioMessage::Failed(error.clone()));
        return Err(error);
    }
    Ok(())
}

fn audio_worker_stopped(source: AudioSource) -> IpcError {
    IpcError::new(
        "audio_worker_stopped",
        format!("The {source:?} capture worker stopped unexpectedly."),
        true,
    )
}
