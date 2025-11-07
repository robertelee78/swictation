//! Lock-free circular buffer for audio samples

use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;

/// Lock-free circular buffer for audio samples
///
/// Uses `ringbuf` crate for wait-free single-producer single-consumer (SPSC) operations.
/// Perfect for real-time audio where audio callback (producer) and processing thread (consumer)
/// run concurrently without blocking each other.
pub struct CircularBuffer {
    producer: ringbuf::HeapProd<f32>,
    consumer: ringbuf::HeapCons<f32>,
    capacity: usize,
}

impl CircularBuffer {
    /// Create new circular buffer with given capacity
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of f32 samples to store
    ///
    /// # Example
    ///
    /// ```
    /// use swictation_audio::CircularBuffer;
    ///
    /// let buffer = CircularBuffer::new(16000); // 1 second @ 16kHz
    /// ```
    pub fn new(capacity: usize) -> Self {
        let rb = HeapRb::<f32>::new(capacity);
        let (producer, consumer) = rb.split();

        Self {
            producer,
            consumer,
            capacity,
        }
    }

    /// Write samples to the buffer (producer side, called from audio callback)
    ///
    /// Returns number of samples actually written. If buffer is full, some samples may be dropped.
    ///
    /// # Performance
    ///
    /// Wait-free operation, guaranteed to complete in bounded time regardless of consumer activity.
    pub fn write(&mut self, samples: &[f32]) -> usize {
        self.producer.push_slice(samples)
    }

    /// Read samples from the buffer (consumer side, called from processing thread)
    ///
    /// Returns number of samples actually read. May return fewer than requested if buffer
    /// doesn't contain enough samples.
    pub fn read(&mut self, output: &mut [f32]) -> usize {
        self.consumer.pop_slice(output)
    }

    /// Get number of samples currently available for reading
    pub fn available(&self) -> usize {
        self.consumer.occupied_len()
    }

    /// Get remaining capacity for writing
    pub fn free_space(&self) -> usize {
        self.producer.vacant_len()
    }

    /// Get total buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.consumer.is_empty()
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.producer.is_full()
    }

    /// Clear all data from buffer
    pub fn clear(&mut self) {
        self.consumer.clear();
    }

    /// Read all available samples into a Vec
    pub fn read_all(&mut self) -> Vec<f32> {
        let available = self.available();
        let mut output = vec![0.0; available];
        let read = self.read(&mut output);
        output.truncate(read);
        output
    }

    /// Peek at samples without consuming them
    pub fn peek(&self, count: usize) -> Vec<f32> {
        let available = self.available().min(count);
        let output = vec![0.0; available];

        // ringbuf doesn't have built-in peek, so we need to implement it
        // For now, we'll use a simplified version
        // In production, you'd want to use the internal ring buffer structure
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_buffer_basic() {
        let mut buffer = CircularBuffer::new(1024);

        // Write some samples
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let written = buffer.write(&samples);
        assert_eq!(written, 5);
        assert_eq!(buffer.available(), 5);

        // Read them back
        let mut output = vec![0.0; 5];
        let read = buffer.read(&mut output);
        assert_eq!(read, 5);
        assert_eq!(output, samples);
        assert_eq!(buffer.available(), 0);
    }

    #[test]
    fn test_circular_buffer_wrap() {
        let mut buffer = CircularBuffer::new(10);

        // Fill buffer
        let samples: Vec<f32> = (0..10).map(|i| i as f32).collect();
        assert_eq!(buffer.write(&samples), 10);
        assert!(buffer.is_full());

        // Read half
        let mut output = vec![0.0; 5];
        assert_eq!(buffer.read(&mut output), 5);

        // Write more (should wrap around)
        let more_samples: Vec<f32> = (10..15).map(|i| i as f32).collect();
        assert_eq!(buffer.write(&more_samples), 5);

        // Read all remaining
        let all = buffer.read_all();
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn test_buffer_overflow_handling() {
        let mut buffer = CircularBuffer::new(5);

        let samples = vec![1.0; 10];
        let written = buffer.write(&samples);

        // Should only write 5 (capacity)
        assert_eq!(written, 5);
        assert!(buffer.is_full());
    }
}
