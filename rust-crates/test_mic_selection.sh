#!/bin/bash
# Quick test: Record from each device and check audio levels

echo "Testing audio capture from each device..."

for i in 0 1 2 3; do
    echo ""
    echo "=== Testing device $i ==="

    # Update config with device index
    sed -i "s/^audio_device_index = .*/audio_device_index = $i/" /home/robert/.config/swictation/config.toml

    # Kill and restart daemon
    pkill swictation-daemon 2>/dev/null
    sleep 1

    export LD_LIBRARY_PATH="/home/robert/.cache/sherpa-rs/x86_64-unknown-linux-gnu/cd22ee337e205674536643d2bc86e984fbbbf9c5c52743c4df6fc8d8dd17371b/sherpa-onnx-v1.12.9-linux-x64-shared/lib:/home/robert/.cache/ort.pyke.io/dfbin/x86_64-unknown-linux-gnu/ED1716DE95974BF47AB0223CA33734A0B5A5D09A181225D0E8ED62D070AEA893/onnxruntime/lib:$LD_LIBRARY_PATH"

    /opt/swictation/rust-crates/target/release/swictation-daemon > /tmp/test-device-$i.log 2>&1 &
    DAEMON_PID=$!

    sleep 5

    # Check if daemon started
    if ! ps -p $DAEMON_PID > /dev/null 2>&1; then
        echo "Device $i: FAILED TO START"
        cat /tmp/test-device-$i.log | tail -5
        continue
    fi

    # Get device name from log
    DEVICE=$(grep "Device:" /tmp/test-device-$i.log | head -1)
    echo "$DEVICE"

    # Start recording
    echo "toggle" | nc -U /tmp/swictation.sock 2>/dev/null
    sleep 1

    # Play audio
    mplayer -really-quiet /home/robert/Documents/python/translate-stream/examples/en-short.mp3 >/dev/null 2>&1
    sleep 2

    # Stop recording
    echo "toggle" | nc -U /tmp/swictation.sock 2>/dev/null
    sleep 1

    # Check for speech detection
    if grep -q "VAD detected speech" /tmp/test-device-$i.log; then
        echo "✅ Device $i DETECTED SPEECH!"
        grep "Transcribed:" /tmp/test-device-$i.log | head -3
    else
        SILENCE_COUNT=$(grep -c "VAD detected silence" /tmp/test-device-$i.log)
        echo "❌ Device $i: Only silence detected ($SILENCE_COUNT samples)"
    fi

    kill $DAEMON_PID 2>/dev/null
    sleep 1
done

echo ""
echo "Test complete. Check /tmp/test-device-*.log for details"
