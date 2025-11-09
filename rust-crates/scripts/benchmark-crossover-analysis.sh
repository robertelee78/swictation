#!/bin/bash
# GPU vs CPU Crossover Point Analysis
# Tests various audio lengths to find when GPU becomes faster than CPU

set -e

MODEL_110M="/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000"
OUTPUT="/tmp/crossover-analysis.csv"

echo "GPU vs CPU Crossover Point Analysis"
echo "===================================="
echo ""
echo "Testing audio lengths from 0.5s to 30s..."
echo ""

# Create CSV header
echo "audio_duration_s,gpu_time_ms,cpu_time_ms,speedup,faster" > "$OUTPUT"

# Generate test audio files at different lengths
for duration in 0.5 1 2 3 4 5 7 10 15 20 30; do
    echo "Testing ${duration}s audio..."

    # Generate test audio
    TEST_FILE="/tmp/test-${duration}s.wav"
    ffmpeg -f lavfi -i "sine=frequency=1000:duration=${duration}" -ar 16000 -ac 1 "$TEST_FILE" -y 2>/dev/null

    # Test GPU (3 runs, take median)
    gpu_times=()
    for i in {1..3}; do
        result=$(cargo run --release --quiet --example test_audio_file -- "$MODEL_110M" true "$TEST_FILE" 2>/dev/null | grep "Inference:" | awk '{print $2}' | tr -d 'ms')
        gpu_times+=($result)
    done
    gpu_time=$(printf '%s\n' "${gpu_times[@]}" | sort -n | sed -n '2p')

    # Test CPU (3 runs, take median)
    cpu_times=()
    for i in {1..3}; do
        result=$(cargo run --release --quiet --example test_audio_file -- "$MODEL_110M" false "$TEST_FILE" 2>/dev/null | grep "Inference:" | awk '{print $2}' | tr -d 'ms')
        cpu_times+=($result)
    done
    cpu_time=$(printf '%s\n' "${cpu_times[@]}" | sort -n | sed -n '2p')

    # Calculate speedup
    speedup=$(echo "scale=2; $cpu_time / $gpu_time" | bc)
    faster=$(if (( $(echo "$speedup > 1" | bc -l) )); then echo "GPU"; else echo "CPU"; fi)

    echo "  GPU: ${gpu_time}ms, CPU: ${cpu_time}ms, Speedup: ${speedup}x (${faster} faster)"
    echo "${duration},${gpu_time},${cpu_time},${speedup},${faster}" >> "$OUTPUT"

    # Cleanup
    rm "$TEST_FILE"
done

echo ""
echo "Results saved to: $OUTPUT"
echo ""
echo "Crossover Analysis:"
echo "==================="

# Find crossover point
crossover=$(awk -F',' 'NR>1 && $5=="GPU" {print $1; exit}' "$OUTPUT")
if [ -n "$crossover" ]; then
    echo "✓ GPU becomes faster at: ${crossover}s"
else
    echo "✗ GPU never becomes faster in tested range"
fi

echo ""
echo "Visual representation:"
awk -F',' '
NR>1 {
    printf "%5.1fs: ", $1
    if ($5=="GPU") {
        printf "GPU █████████ %4dms vs CPU %4dms (%.2fx faster)\n", $2, $3, $4
    } else {
        printf "CPU █████████ %4dms vs GPU %4dms (%.2fx slower)\n", $3, $2, $4
    }
}' "$OUTPUT"
