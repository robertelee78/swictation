//! Direct ONNX Runtime implementation of Silero VAD
//! Replaces sherpa-rs dependency with modern ort crate

use crate::{Result, VadError};
use ndarray::{Array2, Array3, ArrayView3};
#[cfg(target_os = "macos")]
use ort::execution_providers::coreml::{CoreMLComputeUnits, CoreMLExecutionProvider};
#[cfg(not(target_os = "macos"))]
use ort::execution_providers::CUDAExecutionProvider;
use ort::{execution_providers::CPUExecutionProvider, inputs, session::Session, value::Tensor};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Windows of audio kept before the threshold crossing and prepended to a
/// segment when speech triggers.
///
/// Buffering only from the first window that crosses the threshold clips the
/// onset of the word that caused the crossing — the consonant is what pushes
/// the probability up, so it is always in the window that is thrown away. Two
/// 512-sample windows are 64 ms at 16 kHz, enough to cover that onset.
const PREROLL_WINDOWS: usize = 2;

/// Turns per-window speech probabilities into bounded speech segments.
///
/// Deliberately holds no ONNX session: the segmentation rules (pre-roll,
/// silence tolerance, minimum and maximum durations) are the part with real
/// edge cases, and this way they are unit-testable by feeding probabilities
/// directly, with no model on disk.
struct SpeechSegmenter {
    threshold: f32,
    min_speech_samples: usize,
    min_silence_samples: usize,
    max_speech_samples: usize,

    triggered: bool,
    /// Whether a window at or above the threshold has entered the buffer since
    /// the last emit.
    ///
    /// A cap-split emits and stays triggered with an EMPTY buffer, so the
    /// silence that follows is buffered under the tolerance and closed by
    /// min_silence as a segment of its own — pure silence, passing min_speech
    /// on length alone. Content, not length, decides whether a segment is
    /// worth transcribing.
    voiced: bool,
    /// Sample position of the last window that was above threshold.
    temp_end: usize,
    current_sample: usize,

    speech_buffer: Vec<f32>,
    /// The most recent [`PREROLL_WINDOWS`] windows seen while not triggered.
    preroll: VecDeque<Vec<f32>>,
}

impl SpeechSegmenter {
    fn new(
        threshold: f32,
        min_speech_samples: usize,
        min_silence_samples: usize,
        max_speech_samples: usize,
    ) -> Self {
        Self {
            threshold,
            min_speech_samples,
            min_silence_samples,
            max_speech_samples,
            triggered: false,
            voiced: false,
            temp_end: 0,
            current_sample: 0,
            speech_buffer: Vec::new(),
            preroll: VecDeque::with_capacity(PREROLL_WINDOWS),
        }
    }

    /// Feed one window and its speech probability.
    ///
    /// Returns a completed speech segment when the window ends one, either
    /// because silence ran past `min_silence_samples` or because the segment
    /// reached `max_speech_samples`.
    fn push(&mut self, speech_prob: f32, audio_chunk: &[f32]) -> Option<Vec<f32>> {
        self.current_sample += audio_chunk.len();

        if speech_prob >= self.threshold {
            if !self.triggered {
                self.triggered = true;
                // Prepend the windows just before the crossing so the word
                // onset that triggered it survives.
                for window in self.preroll.drain(..) {
                    self.speech_buffer.extend_from_slice(&window);
                }
            }
            // temp_end tracks the END of speech (last window above threshold)
            self.temp_end = self.current_sample;
            self.voiced = true;
            self.speech_buffer.extend_from_slice(audio_chunk);
        } else if self.triggered {
            if self.current_sample - self.temp_end > self.min_silence_samples {
                // Silence duration exceeded threshold - speech segment complete
                let speech = std::mem::take(&mut self.speech_buffer);
                self.triggered = false;
                self.remember_window(audio_chunk);
                // Too short to be speech, or never voiced at all, means it was
                // noise; drop it
                return self.emit_if_voiced(speech);
            }
            // Still within silence tolerance, keep buffering
            self.speech_buffer.extend_from_slice(audio_chunk);
        } else {
            self.remember_window(audio_chunk);
        }

        // Cap the segment. Without this, a speaker who never pauses grows one
        // unbounded buffer that only reaches STT when the stream ends. The
        // split lands on the window boundary and leaves the detector
        // triggered, so the next window continues the following segment.
        if self.triggered && self.speech_buffer.len() >= self.max_speech_samples {
            let speech = std::mem::take(&mut self.speech_buffer);
            // Taken either way, so the buffer stays bounded even when the cap
            // is reached by silence alone (min_silence >= max_speech).
            return self.voiced_since_emit().then_some(speech);
        }

        None
    }

    /// Return any buffered speech at end of stream.
    fn flush(&mut self) -> Option<Vec<f32>> {
        let speech = std::mem::take(&mut self.speech_buffer);
        self.triggered = false;
        self.preroll.clear();
        self.emit_if_voiced(speech)
    }

    fn reset(&mut self) {
        self.triggered = false;
        self.voiced = false;
        self.temp_end = 0;
        self.current_sample = 0;
        self.speech_buffer.clear();
        self.preroll.clear();
    }

    fn emit_if_voiced(&mut self, speech: Vec<f32>) -> Option<Vec<f32>> {
        self.voiced_since_emit()
            .then(|| self.emit(speech))
            .flatten()
    }

    /// Whether the segment now ending contained speech, and clear the flag so
    /// the next one starts unvoiced.
    fn voiced_since_emit(&mut self) -> bool {
        std::mem::take(&mut self.voiced)
    }

    fn emit(&self, speech: Vec<f32>) -> Option<Vec<f32>> {
        (!speech.is_empty() && speech.len() >= self.min_speech_samples).then_some(speech)
    }

    fn remember_window(&mut self, audio_chunk: &[f32]) {
        if self.preroll.len() == PREROLL_WINDOWS {
            self.preroll.pop_front();
        }
        self.preroll.push_back(audio_chunk.to_vec());
    }
}

/// Silero VAD model using direct ONNX Runtime
pub struct SileroVadOrt {
    session: Arc<Mutex<Session>>,
    sample_rate: i32,
    window_size: usize,

    // State for streaming - Silero VAD v6 uses LSTM with separate h and c states
    // Each state is [2 layers, 1 batch, 64 hidden units]
    h_state: Array3<f32>, // LSTM hidden state [2, 1, 64]
    c_state: Array3<f32>, // LSTM cell state [2, 1, 64]

    // Threshold crossing / segment buffering
    segmenter: SpeechSegmenter,

    // Debug mode
    debug: bool,
}

impl SileroVadOrt {
    /// Create new Silero VAD with ONNX Runtime
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model_path: &str,
        threshold: f32,
        sample_rate: i32,
        window_size: usize,
        min_speech_duration_ms: i32,
        min_silence_duration_ms: i32,
        max_speech_duration_ms: i32,
        provider: Option<String>,
        debug: bool,
    ) -> Result<Self> {
        // Build session with appropriate execution provider
        let mut session_builder = Session::builder().map_err(|e| {
            VadError::initialization(format!("Failed to create session builder: {}", e))
        })?;

        // macOS: use CoreML for Apple Silicon acceleration
        #[cfg(target_os = "macos")]
        {
            if let Some(ref prov) = provider {
                if prov.contains("coreml") {
                    session_builder = session_builder
                        .with_execution_providers([
                            CoreMLExecutionProvider::default()
                                .with_compute_units(CoreMLComputeUnits::CPUAndNeuralEngine)
                                .build(),
                            CPUExecutionProvider::default().build(),
                        ])
                        .map_err(|e| {
                            VadError::initialization(format!(
                                "Failed to set CoreML execution providers: {}",
                                e
                            ))
                        })?;
                    println!("Silero VAD: Using CoreML provider (CPU + Neural Engine)");
                } else {
                    session_builder = session_builder
                        .with_execution_providers([CPUExecutionProvider::default().build()])
                        .map_err(|e| {
                            VadError::initialization(format!("Failed to set CPU provider: {}", e))
                        })?;
                    println!("Silero VAD: Using CPU provider");
                }
            } else {
                session_builder = session_builder
                    .with_execution_providers([CPUExecutionProvider::default().build()])
                    .map_err(|e| {
                        VadError::initialization(format!("Failed to set CPU provider: {}", e))
                    })?;
            }
        }

        // Linux/Windows: use CUDA if requested, with CPU fallback
        #[cfg(not(target_os = "macos"))]
        {
            if let Some(ref prov) = provider {
                if prov.contains("cuda") || prov.contains("CUDA") {
                    session_builder = session_builder
                        .with_execution_providers([
                            CUDAExecutionProvider::default().build(),
                            CPUExecutionProvider::default().build(),
                        ])
                        .map_err(|e| {
                            VadError::initialization(format!(
                                "Failed to set CUDA execution providers: {}",
                                e
                            ))
                        })?;
                    println!("Silero VAD: Using CUDA provider with CPU fallback");
                } else {
                    session_builder = session_builder
                        .with_execution_providers([CPUExecutionProvider::default().build()])
                        .map_err(|e| {
                            VadError::initialization(format!("Failed to set CPU provider: {}", e))
                        })?;
                }
            } else {
                session_builder = session_builder
                    .with_execution_providers([CPUExecutionProvider::default().build()])
                    .map_err(|e| {
                        VadError::initialization(format!("Failed to set CPU provider: {}", e))
                    })?;
            }
        }

        let session = session_builder
            .commit_from_file(model_path)
            .map_err(|e| VadError::initialization(format!("Failed to load model: {}", e)))?;

        // Print model input/output names for debugging
        println!("=== ONNX Model Metadata ===");
        println!("Model inputs:");
        for input in session.inputs.iter() {
            println!("  - name: '{}' (type: {:?})", input.name, input.input_type);
        }
        println!("Model outputs:");
        for output in session.outputs.iter() {
            println!(
                "  - name: '{}' (type: {:?})",
                output.name, output.output_type
            );
        }
        println!("===========================");

        // Calculate sample counts from durations
        let min_speech_samples =
            (min_speech_duration_ms as f32 * sample_rate as f32 / 1000.0) as usize;
        let min_silence_samples =
            (min_silence_duration_ms as f32 * sample_rate as f32 / 1000.0) as usize;
        // At least one window, so the cap can always be reached
        let max_speech_samples = ((max_speech_duration_ms as f32 * sample_rate as f32 / 1000.0)
            as usize)
            .max(window_size);

        // Initialize LSTM states (2 layers, 1 batch, 64 hidden units each)
        let h_state = Array3::<f32>::zeros((2, 1, 64));
        let c_state = Array3::<f32>::zeros((2, 1, 64));

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            sample_rate,
            window_size,
            h_state,
            c_state,
            segmenter: SpeechSegmenter::new(
                threshold,
                min_speech_samples,
                min_silence_samples,
                max_speech_samples,
            ),
            debug,
        })
    }

    /// Process audio chunk and detect speech
    pub fn process(&mut self, audio_chunk: &[f32]) -> Result<Option<Vec<f32>>> {
        if audio_chunk.len() != self.window_size {
            return Err(VadError::processing(format!(
                "Expected {} samples, got {}",
                self.window_size,
                audio_chunk.len()
            )));
        }

        // Convert audio to ndarray with proper shape (batch_size=1, sequence_len)
        // CRITICAL: Must use standard (C-contiguous) layout, not Fortran layout
        let input_array = Array2::from_shape_vec((1, audio_chunk.len()), audio_chunk.to_vec())
            .map_err(|e| VadError::processing(format!("Failed to reshape input: {}", e)))?;

        if self.debug && self.segmenter.current_sample == 0 {
            eprintln!("VAD Debug:");
            eprintln!("  input shape: {:?}", input_array.shape());
            eprintln!(
                "  input is C-contiguous: {}",
                input_array.is_standard_layout()
            );
            eprintln!(
                "  input range: [{:.6}, {:.6}]",
                input_array.iter().copied().fold(f32::INFINITY, f32::min),
                input_array
                    .iter()
                    .copied()
                    .fold(f32::NEG_INFINITY, f32::max)
            );
            eprintln!("  h_state shape: {:?}", self.h_state.shape());
            eprintln!("  c_state shape: {:?}", self.c_state.shape());
        }

        // Create tensors from OWNED arrays - ort 2.0.0-rc.10 requires owned, not views
        // Model expects inputs: "x", "h", "c" (NOT "input", "state", "sr")
        let input_value = Tensor::from_array(input_array)
            .map_err(|e| VadError::processing(format!("Failed to create input: {}", e)))?;
        let h_value = Tensor::from_array(self.h_state.clone())
            .map_err(|e| VadError::processing(format!("Failed to create h state: {}", e)))?;
        let c_value = Tensor::from_array(self.c_state.clone())
            .map_err(|e| VadError::processing(format!("Failed to create c state: {}", e)))?;

        // Run inference with correct tensor names
        let mut session_guard = self
            .session
            .lock()
            .map_err(|e| VadError::processing(format!("Failed to lock session: {}", e)))?;

        let outputs = session_guard
            .run(inputs![
                "x" => input_value,
                "h" => h_value,
                "c" => c_value
            ])
            .map_err(|e| VadError::processing(format!("Failed to run inference: {}", e)))?;

        if self.debug && self.segmenter.current_sample == 0 {
            eprintln!("  Model returned {} outputs", outputs.len());
        }

        // Extract speech probability from the output (shape: [batch, 1])
        // Model outputs: "prob", "new_h", "new_c"
        let output_array: ndarray::ArrayView2<f32> = outputs["prob"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract prob: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape prob: {}", e)))?;
        let speech_prob = output_array[[0, 0]];

        if self.debug && self.segmenter.current_sample == 0 {
            eprintln!("  Output array shape: {:?}", output_array.shape());
            eprintln!("  Speech probability: {}", speech_prob);
        }

        // Extract and update LSTM states
        let new_h: ArrayView3<f32> = outputs["new_h"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract new_h: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape new_h: {}", e)))?;

        let new_c: ArrayView3<f32> = outputs["new_c"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract new_c: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape new_c: {}", e)))?;

        // Copy state data
        if self.debug && self.segmenter.current_sample == 0 {
            eprintln!("  h_state before: sum={}", self.h_state.sum());
            eprintln!("  c_state before: sum={}", self.c_state.sum());
            eprintln!("  new_h sum: {}", new_h.sum());
            eprintln!("  new_c sum: {}", new_c.sum());
        }
        self.h_state.assign(&new_h);
        self.c_state.assign(&new_c);

        if self.debug {
            // Print every chunk between 1-2 seconds where we expect speech (RMS=0.087746 in second 1)
            let next_sample = self.segmenter.current_sample + audio_chunk.len();
            let time_s = next_sample as f32 / self.sample_rate as f32;
            if (1.0..=2.0).contains(&time_s)
                || (3.0..=4.5).contains(&time_s)
                || next_sample.is_multiple_of(self.sample_rate as usize)
            {
                eprintln!(
                    "VAD: t={:.2}s, prob={:.6}, threshold={:.3}",
                    time_s, speech_prob, self.segmenter.threshold
                );
            }
        }

        Ok(self.segmenter.push(speech_prob, audio_chunk))
    }

    /// Reset the VAD state
    pub fn reset(&mut self) {
        self.h_state.fill(0.0);
        self.c_state.fill(0.0);
        self.segmenter.reset();
    }

    /// Flush any remaining buffered speech (call at end of stream)
    pub fn flush(&mut self) -> Option<Vec<f32>> {
        self.segmenter.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: usize = 512;

    /// A window of constant `value`, so its position inside an emitted segment
    /// is identifiable by content.
    fn window(value: f32) -> Vec<f32> {
        vec![value; WINDOW]
    }

    fn segmenter(
        min_speech_windows: usize,
        min_silence_windows: usize,
        max_speech_windows: usize,
    ) -> SpeechSegmenter {
        SpeechSegmenter::new(
            0.5,
            min_speech_windows * WINDOW,
            min_silence_windows * WINDOW,
            max_speech_windows * WINDOW,
        )
    }

    #[test]
    fn preroll_is_prepended_when_speech_triggers() {
        let mut seg = segmenter(1, 1, 100);

        // Three windows below threshold; only the last two are pre-roll.
        assert!(seg.push(0.0, &window(1.0)).is_none());
        assert!(seg.push(0.0, &window(2.0)).is_none());
        assert!(seg.push(0.0, &window(3.0)).is_none());

        // Threshold crossing.
        assert!(seg.push(0.9, &window(9.0)).is_none());

        // Silence long enough to close the segment.
        assert!(seg.push(0.0, &window(0.0)).is_none());
        let speech = seg
            .push(0.0, &window(0.0))
            .expect("segment should close after silence exceeds the tolerance");

        assert_eq!(
            speech[0], 2.0,
            "segment must start two windows before the crossing"
        );
        assert_eq!(speech[WINDOW], 3.0, "second pre-roll window must follow");
        assert_eq!(
            speech[2 * WINDOW],
            9.0,
            "the triggering window must come after the pre-roll"
        );
        assert!(
            !speech.contains(&1.0),
            "pre-roll is bounded to {PREROLL_WINDOWS} windows"
        );
    }

    #[test]
    fn continuous_speech_is_split_at_max_speech() {
        const MAX_WINDOWS: usize = 4;
        const FED_WINDOWS: usize = 10;
        let mut seg = segmenter(1, 10, MAX_WINDOWS);

        let mut segments = Vec::new();
        for i in 0..FED_WINDOWS {
            // Never drops below threshold: one unbroken utterance.
            if let Some(speech) = seg.push(0.9, &window(i as f32)) {
                segments.push(speech);
            }
        }
        if let Some(tail) = seg.flush() {
            segments.push(tail);
        }

        let lengths: Vec<usize> = segments.iter().map(Vec::len).collect();
        assert!(
            segments.len() >= 2,
            "unbroken speech past the cap must be split, got {lengths:?}"
        );
        for len in &lengths {
            assert!(
                *len <= MAX_WINDOWS * WINDOW,
                "segment of {len} samples exceeds the cap: {lengths:?}"
            );
        }

        // Splitting must lose nothing and must not reorder audio.
        let rejoined: Vec<f32> = segments.concat();
        let expected: Vec<f32> = (0..FED_WINDOWS).flat_map(|i| window(i as f32)).collect();
        assert_eq!(
            rejoined, expected,
            "split must preserve every sample in order"
        );
    }

    #[test]
    fn split_leaves_detection_running() {
        let mut seg = segmenter(1, 10, 2);

        // Two windows fill the cap and emit.
        assert!(seg.push(0.9, &window(1.0)).is_none());
        assert!(seg.push(0.9, &window(1.0)).is_some());

        // Still triggered, so the next windows build the following segment
        // rather than being treated as a fresh (pre-rolled) trigger.
        assert!(seg.triggered, "detector must stay triggered across a split");
        assert!(seg.push(0.9, &window(2.0)).is_none());
        let next = seg
            .push(0.9, &window(2.0))
            .expect("the following segment must also be capped");
        assert!(
            next.iter().all(|s| *s == 2.0),
            "the second segment must contain only audio recorded after the split"
        );
    }

    #[test]
    fn cap_split_followed_by_silence_emits_nothing() {
        let mut seg = segmenter(1, 1, 2);

        assert!(seg.push(0.9, &window(1.0)).is_none());
        assert!(
            seg.push(0.9, &window(1.0)).is_some(),
            "the cap must force-emit"
        );

        // The force-emit leaves the detector triggered with an empty buffer.
        // The silence that follows fills that buffer under the tolerance, and
        // min_silence then closes it — a segment of pure silence, long enough
        // to pass min_speech on length alone.
        assert!(seg.push(0.0, &window(0.0)).is_none());
        assert!(
            seg.push(0.0, &window(0.0)).is_none(),
            "silence after a cap-split holds no speech and must be dropped"
        );
        assert!(seg.push(0.0, &window(0.0)).is_none());
    }

    #[test]
    fn cap_split_followed_by_stop_flushes_nothing() {
        let mut seg = segmenter(1, 10, 3);

        assert!(seg.push(0.9, &window(1.0)).is_none());
        assert!(seg.push(0.9, &window(1.0)).is_none());
        assert!(
            seg.push(0.9, &window(1.0)).is_some(),
            "the cap must force-emit"
        );

        // Tolerated silence buffers, but it is still silence: the
        // end-of-stream flush must not hand it to STT.
        assert!(seg.push(0.0, &window(0.0)).is_none());
        assert!(
            seg.flush().is_none(),
            "a flush with no voiced content must emit nothing"
        );
    }

    #[test]
    fn cap_split_then_more_speech_emits_that_speech() {
        let mut seg = segmenter(1, 1, 3);

        for _ in 0..2 {
            assert!(seg.push(0.9, &window(1.0)).is_none());
        }
        assert!(
            seg.push(0.9, &window(1.0)).is_some(),
            "the cap must force-emit"
        );

        // Speech resumes after the split, so the segment that silence closes
        // is real dictation and must survive.
        assert!(seg.push(0.9, &window(7.0)).is_none());
        assert!(seg.push(0.0, &window(0.0)).is_none());
        let speech = seg
            .push(0.0, &window(0.0))
            .expect("speech recorded after a cap-split must still be emitted");
        assert_eq!(
            speech[0], 7.0,
            "the segment must start with the audio recorded after the split"
        );
    }

    #[test]
    fn burst_shorter_than_min_speech_is_discarded() {
        // The burst buffers 4 windows in total (2 pre-roll + the crossing +
        // one tolerated silence window), still short of the 6-window minimum.
        let mut seg = segmenter(6, 1, 100);

        assert!(seg.push(0.0, &window(0.0)).is_none());
        assert!(seg.push(0.0, &window(0.0)).is_none());
        assert!(seg.push(0.9, &window(9.0)).is_none());
        assert!(seg.push(0.0, &window(0.0)).is_none());
        assert!(
            seg.push(0.0, &window(0.0)).is_none(),
            "a burst below min_speech is noise and must be dropped"
        );
        assert!(!seg.triggered);
    }
}
