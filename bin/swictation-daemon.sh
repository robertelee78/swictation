#!/bin/bash
# Swictation daemon wrapper - sets up library paths

# Set library path for sherpa-onnx
export LD_LIBRARY_PATH="/home/robert/.cache/sherpa-rs/x86_64-unknown-linux-gnu/cd22ee337e205674536643d2bc86e984fbbbf9c5c52743c4df6fc8d8dd17371b/sherpa-onnx-v1.12.9-linux-x64-shared/lib:/home/robert/.cache/ort.pyke.io/dfbin/x86_64-unknown-linux-gnu/ED1716DE95974BF47AB0223CA33734A0B5A5D09A181225D0E8ED62D070AEA893/onnxruntime/lib:$LD_LIBRARY_PATH"

# Run daemon
exec /opt/swictation/rust-crates/target/release/swictation-daemon "$@"
