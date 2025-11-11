//! Audio capture with cpal
//!
//! Provides real-time audio input from microphone or system audio loopback.
//! Uses lock-free circular buffer for zero-copy operations.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig, SampleFormat};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::buffer::CircularBuffer;
use crate::error::{AudioError, Result};
use crate::resampler::Resampler;
use crate::AudioConfig;

/// Callback for audio chunks (streaming mode)
pub type ChunkCallback = Arc<dyn Fn(Vec<f32>) + Send + Sync>;

/// Audio device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub index: usize,
    pub name: String,
    pub is_default: bool,
    pub max_input_channels: u16,
    pub max_output_channels: u16,
    pub default_sample_rate: u32,
}

/// Audio capture implementation
pub struct AudioCapture {
    config: AudioConfig,
    buffer: Arc<Mutex<CircularBuffer>>,
    chunk_buffer: Arc<Mutex<Vec<f32>>>,
    stream: Option<Stream>,
    is_recording: Arc<AtomicBool>,
    total_frames: Arc<AtomicUsize>,
    host: Host,
    device: Option<Device>,
    chunk_callback: Option<ChunkCallback>,
    resampler: Arc<Mutex<Option<Resampler>>>,
    resample_buffer: Arc<Mutex<Vec<f32>>>,  // Buffer for accumulating samples before resampling
}

impl AudioCapture {
    /// Create new audio capture instance
    pub fn new(config: AudioConfig) -> Result<Self> {
        let host = cpal::default_host();

        // Calculate buffer capacity
        let buffer_capacity = (config.buffer_duration * config.sample_rate as f32) as usize;
        let buffer = Arc::new(Mutex::new(CircularBuffer::new(buffer_capacity)));

        // Calculate chunk buffer size for streaming mode
        let chunk_capacity = if config.streaming_mode {
            (config.chunk_duration * config.sample_rate as f32) as usize
        } else {
            0
        };
        let chunk_buffer = Arc::new(Mutex::new(Vec::with_capacity(chunk_capacity)));

        Ok(Self {
            config,
            buffer,
            chunk_buffer,
            stream: None,
            is_recording: Arc::new(AtomicBool::new(false)),
            total_frames: Arc::new(AtomicUsize::new(0)),
            host,
            device: None,
            chunk_callback: None,
            resampler: Arc::new(Mutex::new(None)),
            resample_buffer: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Set callback for audio chunks (streaming mode)
    pub fn set_chunk_callback<F>(&mut self, callback: F)
    where
        F: Fn(Vec<f32>) + Send + Sync + 'static,
    {
        self.chunk_callback = Some(Arc::new(callback));
    }

    /// List all available audio devices
    pub fn list_devices() -> Result<Vec<DeviceInfo>> {
        let host = cpal::default_host();
        let mut devices = Vec::new();

        let default_input = host.default_input_device();
        let default_output = host.default_output_device();

        for (index, device) in host.input_devices()
            .map_err(|e| AudioError::device(format!("Failed to enumerate devices: {}", e)))?
            .enumerate()
        {
            let name = device.name()
                .unwrap_or_else(|_| format!("Unknown Device {}", index));

            let is_default_input = default_input.as_ref()
                .and_then(|d| d.name().ok())
                .map(|n| n == name)
                .unwrap_or(false);

            let is_default_output = default_output.as_ref()
                .and_then(|d| d.name().ok())
                .map(|n| n == name)
                .unwrap_or(false);

            // Get supported config
            let supported_config = device.default_input_config()
                .ok();

            let (max_input_channels, default_sample_rate) = if let Some(config) = supported_config {
                (config.channels(), config.sample_rate().0)
            } else {
                (0, 0)
            };

            // Try to get output channels too
            let max_output_channels = device.default_output_config()
                .ok()
                .map(|c| c.channels())
                .unwrap_or(0);

            devices.push(DeviceInfo {
                index,
                name,
                is_default: is_default_input || is_default_output,
                max_input_channels,
                max_output_channels,
                default_sample_rate,
            });
        }

        Ok(devices)
    }

    /// Auto-detect best available audio device (cross-platform)
    fn auto_select_device(&self) -> Result<Device> {
        // Check for environment variable override first
        if let Ok(device_name) = std::env::var("SWICTATION_AUDIO_DEVICE") {
            println!("Looking for device from env var: {}", device_name);

            // Find device by name (cross-platform)
            for device in self.host.input_devices()
                .map_err(|e| AudioError::device(format!("Failed to enumerate devices: {}", e)))? {

                if let Ok(name) = device.name() {
                    if name.to_lowercase().contains(&device_name.to_lowercase()) {
                        println!("Found matching device: {}", name);
                        return Ok(device);
                    }
                }
            }
            println!("Device '{}' not found, continuing with auto-detection", device_name);
        }

        // Auto-detect best device based on capabilities
        let mut best_device = None;
        let mut best_score = 0;

        for device in self.host.input_devices()
            .map_err(|e| AudioError::device(format!("Failed to enumerate devices: {}", e)))? {

            let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            let mut score = 0;

            // Check if device supports our desired format
            if let Ok(config) = device.default_input_config() {
                // Prefer devices that support standard sample rates
                if config.sample_rate().0 == 44100 || config.sample_rate().0 == 48000 {
                    score += 10;
                }

                // Prefer devices with standard channel counts
                if config.channels() <= 2 {
                    score += 5;
                }

                // On Linux, prefer "plughw" devices for better compatibility
                #[cfg(target_os = "linux")]
                if name.contains("plughw") {
                    score += 20;
                }

                // Prefer USB devices (often external mics)
                if name.to_lowercase().contains("usb") || name.to_lowercase().contains("camera") {
                    score += 15;
                }

                // Avoid certain problematic devices
                if name.to_lowercase().contains("monitor") ||
                   name.to_lowercase().contains("loopback") ||
                   name.to_lowercase().contains("virtual") {
                    score = 0;  // Skip these
                }

                if score > best_score {
                    best_score = score;
                    println!("  → New best device: {} (score: {})", name, score);
                    best_device = Some(device);
                }
            }
        }

        // Print final selection
        if let Some(ref device) = best_device {
            if let Ok(name) = device.name() {
                println!("\n✓ Auto-selected audio device: {} (score: {})", name, best_score);
            }
        }

        // Fall back to default if no good device found
        best_device.or_else(|| self.host.default_input_device())
            .ok_or_else(|| AudioError::device("No suitable input device available".to_string()))
    }

    /// Print device list in formatted output (matches Python version)
    pub fn print_devices() -> Result<()> {
        let devices = Self::list_devices()?;

        println!("\n{}", "=".repeat(78));
        println!("Available Audio Devices:");
        println!("{}\n", "=".repeat(78));

        for device in devices {
            let mut type_parts = Vec::new();
            if device.max_input_channels > 0 {
                type_parts.push("INPUT");
            }
            if device.max_output_channels > 0 {
                type_parts.push("OUTPUT");
            }

            let type_str = type_parts.join("/");
            let default_marker = if device.is_default { " [DEFAULT INPUT]" } else { "" };

            println!("{:3}: {}", device.index, device.name);
            println!("     Type: {}{}", type_str, default_marker);
            println!("     Channels: IN={}, OUT={}",
                     device.max_input_channels,
                     device.max_output_channels);
            println!("     Sample Rate: {} Hz\n", device.default_sample_rate);
        }

        println!("{}", "=".repeat(78));
        Ok(())
    }

    /// Start audio capture
    pub fn start(&mut self) -> Result<()> {
        if self.is_recording.load(Ordering::Relaxed) {
            println!("Warning: Already recording");
            return Ok(());
        }

        // List available devices for debugging
        println!("\n=== Available Input Devices ===");
        for (idx, dev) in self.host.input_devices()
            .map_err(|e| AudioError::device(format!("Failed to enumerate devices: {}", e)))?
            .enumerate()
        {
            let name = dev.name().unwrap_or_else(|_| "Unknown".to_string());
            println!("  [{}] {}", idx, name);
        }

        // Select device
        let device = if let Some(index) = self.config.device_index {
            println!("Selecting device index: {}", index);
            let mut devices = self.host.input_devices()
                .map_err(|e| AudioError::device(format!("Failed to enumerate devices: {}", e)))?;
            devices.nth(index)
                .ok_or_else(|| AudioError::device(format!("Device index {} not found", index)))?
        } else {
            println!("Auto-detecting best audio device...");
            self.auto_select_device()?
        };

        let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());

        // Get supported config
        let supported_config = device.default_input_config()
            .map_err(|e| AudioError::device(format!("Failed to get device config: {}", e)))?;

        let source_sample_rate = supported_config.sample_rate().0;
        let source_channels = supported_config.channels();

        println!("\n=== Starting Audio Capture ===");
        println!("Device: {}", device_name);
        println!("Sample Rate: {} Hz → {} Hz", source_sample_rate, self.config.sample_rate);
        println!("Channels: {} → {}", source_channels, self.config.channels);
        println!("Blocksize: {} samples", self.config.blocksize);

        if self.config.streaming_mode {
            println!("Streaming Mode: ENABLED (chunk duration: {}s)", self.config.chunk_duration);
        }

        // Clear buffers
        self.buffer.lock().clear();
        self.chunk_buffer.lock().clear();
        self.resample_buffer.lock().clear();
        self.total_frames.store(0, Ordering::Relaxed);

        let target_channels = self.config.channels;

        // Initialize resampler if needed
        if source_sample_rate != self.config.sample_rate {
            println!("Creating resampler: {} Hz → {} Hz", source_sample_rate, self.config.sample_rate);
            let resampler = Resampler::new(
                source_sample_rate,
                self.config.sample_rate,
                target_channels,
            )?;
            *self.resampler.lock() = Some(resampler);
        } else {
            *self.resampler.lock() = None;
        }

        // Build stream config
        let stream_config = StreamConfig {
            channels: source_channels,
            sample_rate: cpal::SampleRate(source_sample_rate),
            buffer_size: cpal::BufferSize::Default,  // Let ALSA choose optimal buffer size
        };

        // Clone Arc references for the callback
        let buffer = Arc::clone(&self.buffer);
        let chunk_buffer = Arc::clone(&self.chunk_buffer);
        let total_frames = Arc::clone(&self.total_frames);
        let is_recording = Arc::clone(&self.is_recording);
        let chunk_callback = self.chunk_callback.clone();
        let resampler = Arc::clone(&self.resampler);
        let resample_buffer = Arc::clone(&self.resample_buffer);

        let streaming_mode = self.config.streaming_mode;
        let chunk_frames = (self.config.chunk_duration * self.config.sample_rate as f32) as usize;
        let resample_chunk_size = (source_sample_rate as f32 * 0.1) as usize;  // 100ms chunks at source rate

        // Determine the sample format and build appropriate stream
        let sample_format = supported_config.sample_format();
        println!("Device sample format: {:?}", sample_format);

        // Create audio stream with proper format handling
        let stream = match sample_format {
            SampleFormat::I16 => {
                // Build stream for i16 format (most common for USB mics)
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if !is_recording.load(Ordering::Relaxed) {
                            return;
                        }

                        // Convert i16 to f32 with proper normalization
                        let f32_data: Vec<f32> = data.iter()
                            .map(|&sample| sample as f32 / i16::MAX as f32)
                            .collect();

                        Self::process_audio_data(
                            &f32_data,
                            source_channels,
                            target_channels,
                            &buffer,
                            &chunk_buffer,
                            &total_frames,
                            &chunk_callback,
                            &resampler,
                            &resample_buffer,
                            streaming_mode,
                            chunk_frames,
                            resample_chunk_size,
                        );
                    },
                    |err| {
                        eprintln!("Audio stream error: {}", err);
                    },
                    None,
                )
            },
            SampleFormat::F32 => {
                // Build stream for f32 format
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if !is_recording.load(Ordering::Relaxed) {
                            return;
                        }

                        Self::process_audio_data(
                            data,
                            source_channels,
                            target_channels,
                            &buffer,
                            &chunk_buffer,
                            &total_frames,
                            &chunk_callback,
                            &resampler,
                            &resample_buffer,
                            streaming_mode,
                            chunk_frames,
                            resample_chunk_size,
                        );
                    },
                    |err| {
                        eprintln!("Audio stream error: {}", err);
                    },
                    None,
                )
            },
            _ => {
                return Err(AudioError::device(
                    format!("Unsupported sample format: {:?}", sample_format)
                ));
            }
        }.map_err(|e| AudioError::stream(format!("Failed to build stream: {}", e)))?;

        // Start the stream
        stream.play()
            .map_err(|e| AudioError::stream(format!("Failed to start stream: {}", e)))?;

        self.stream = Some(stream);
        self.device = Some(device);
        self.is_recording.store(true, Ordering::Relaxed);

        println!("✓ Audio capture started (cpal backend)");

        Ok(())
    }

    /// Common audio data processing logic
    fn process_audio_data(
        data: &[f32],
        source_channels: u16,
        target_channels: u16,
        buffer: &Arc<Mutex<CircularBuffer>>,
        chunk_buffer: &Arc<Mutex<Vec<f32>>>,
        total_frames: &Arc<AtomicUsize>,
        chunk_callback: &Option<ChunkCallback>,
        resampler: &Arc<Mutex<Option<Resampler>>>,
        resample_buffer: &Arc<Mutex<Vec<f32>>>,
        streaming_mode: bool,
        chunk_frames: usize,
        resample_chunk_size: usize,
    ) {
        // Convert multi-channel to mono if needed
        let mono_audio: Vec<f32> = if source_channels > target_channels {
            // Average all channels to preserve amplitude from any channel
            data.chunks(source_channels as usize)
                .map(|frame| {
                    frame.iter().sum::<f32>() / frame.len() as f32
                })
                .collect()
        } else {
            data.to_vec()
        };

        // Resample if needed
        let mut audio = mono_audio;
        if resampler.lock().is_some() {
            // Accumulate samples for resampling
            let mut resample_buf = resample_buffer.lock();
            resample_buf.extend_from_slice(&audio);

            // Process when we have enough samples
            if resample_buf.len() >= resample_chunk_size {
                // Extract chunk
                let chunk_to_resample: Vec<f32> = resample_buf.drain(..resample_chunk_size).collect();

                // Resample
                if let Some(ref mut resampler_lock) = resampler.lock().as_mut() {
                    match resampler_lock.process(&chunk_to_resample) {
                        Ok(resampled) => {
                            audio = resampled;
                        }
                        Err(e) => {
                            eprintln!("Resampling error: {}", e);
                            return;
                        }
                    }
                }
            } else {
                // Not enough samples yet, return without processing
                return;
            }
        }

        let frames = audio.len();
        total_frames.fetch_add(frames, Ordering::Relaxed);

        // Streaming mode: accumulate chunks and invoke callback
        if streaming_mode {
            let mut chunk_buf = chunk_buffer.lock();
            chunk_buf.extend_from_slice(&audio);

            // Process complete chunks
            while chunk_buf.len() >= chunk_frames {
                // Extract chunk
                let chunk: Vec<f32> = chunk_buf.drain(..chunk_frames).collect();

                // Invoke chunk callback if set
                if let Some(ref callback) = chunk_callback {
                    eprintln!("AUDIO: Invoking chunk callback with {} samples", chunk.len());
                    callback(chunk);
                } else {
                    eprintln!("AUDIO: No chunk callback set!");
                }
            }
        } else {
            // Non-streaming mode: write to circular buffer for later retrieval
            let mut buf = buffer.lock();
            let written = buf.write(&audio);
            if written < audio.len() {
                eprintln!("Warning: Buffer overflow, dropped {} samples", audio.len() - written);
            }
        }
    }

    /// Stop audio capture and return buffered audio
    pub fn stop(&mut self) -> Result<Vec<f32>> {
        if !self.is_recording.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }

        println!("\n=== Stopping Audio Capture ===");

        self.is_recording.store(false, Ordering::Relaxed);

        // Stop and drop stream
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }

        // Get buffered audio
        let audio = {
            let mut buf = self.buffer.lock();
            buf.read_all()
        };

        let total_frames = self.total_frames.load(Ordering::Relaxed);
        let duration = total_frames as f32 / self.config.sample_rate as f32;

        println!("Captured {} frames ({:.2}s)", total_frames, duration);

        Ok(audio)
    }

    /// Get current buffer contents without stopping
    pub fn get_buffer(&self) -> Vec<f32> {
        let buf = self.buffer.lock();
        let available = buf.available();

        // We can't actually read without consuming in the current CircularBuffer
        // This would need a peek implementation
        // For now, use the peek method (which returns zeros for now)
        buf.peek(available)
    }

    /// Get buffer duration in seconds
    pub fn get_buffer_duration(&self) -> f32 {
        let buf = self.buffer.lock();
        buf.available() as f32 / self.config.sample_rate as f32
    }

    /// Check if currently recording
    pub fn is_active(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    /// Get chunk buffer size (streaming mode)
    pub fn get_chunk_buffer_size(&self) -> usize {
        self.chunk_buffer.lock().len()
    }

    /// Get chunk buffer progress (0.0 to 1.0)
    pub fn get_chunk_buffer_progress(&self) -> f32 {
        let chunk_frames = (self.config.chunk_duration * self.config.sample_rate as f32) as usize;
        if chunk_frames == 0 {
            return 0.0;
        }

        let size = self.chunk_buffer.lock().len();
        (size as f32 / chunk_frames as f32).min(1.0)
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        if self.is_recording.load(Ordering::Relaxed) {
            let _ = self.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        let devices = AudioCapture::list_devices().unwrap();
        assert!(!devices.is_empty(), "Should have at least one audio device");

        for device in &devices {
            println!("{}: {} ({} input channels)",
                     device.index, device.name, device.max_input_channels);
        }
    }

    #[test]
    fn test_audio_capture_creation() {
        let config = AudioConfig::default();
        let capture = AudioCapture::new(config).unwrap();
        assert!(!capture.is_active());
    }

    #[test]
    fn test_buffer_duration() {
        let config = AudioConfig {
            buffer_duration: 5.0,
            ..Default::default()
        };
        let capture = AudioCapture::new(config).unwrap();
        assert_eq!(capture.get_buffer_duration(), 0.0); // Empty initially
    }
}
