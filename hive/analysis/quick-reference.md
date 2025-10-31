# Memory Analysis Quick Reference
**ANALYST VERDICT**: VRAM Exhaustion - Model Too Large

## The Problem in 30 Seconds
```
Current Setup:    Canary-1B-Flash (3.6GB) on 4GB VRAM = 91% usage
Safe Threshold:   70% VRAM = 2.8GB max model size
Result:           350MB headroom (INSUFFICIENT)
Consequence:      4,759 CUDA errors when speech detected
```

## What Happened
1. Service ran stable for 10h50m at 3.6GB VRAM (91%)
2. Speech detected at 07:14:57
3. VAD attempted inference (needs ~200-400MB VRAM)
4. VRAM at 100% - allocation failed
5. CUDA error cascade: 40-50 errors/second
6. System RAM spiked 789MB → 6GB
7. Swap activated: 0 → 1.3GB

## The Fix
**IMMEDIATE**: Switch to smaller model
```bash
# Option 1: Canary-500M (RECOMMENDED)
MODEL="nvidia/canary-500m"     # 1.2GB = 30% VRAM ✅

# Option 2: Distil-Whisper Medium
MODEL="distil-whisper/medium"  # 1.5GB = 37% VRAM ✅

# Option 3: Whisper Small
MODEL="openai/whisper-small"   # 2.4GB = 59% VRAM ⚠️
```

## Safe Operating Zones
```
VRAM (4GB total):
  ✅ Safe:     0% - 70%   (0 - 2.8GB)
  ⚠️  Warning: 70% - 85%  (2.8 - 3.5GB)
  🔴 Danger:   85% - 95%  (3.5 - 3.9GB)
  💀 Critical: 95% - 100% (3.9 - 4.1GB) ← YOU ARE HERE

Current: 91% (3.74GB used)
Target:  30% (1.2GB Canary-500M)
```

## No Memory Leak
- Memory constant at 3600.1 MB for 10h50m
- No gradual growth detected
- Event-triggered spike (not time-based)
- Problem is capacity, not leak

## Priority Actions
1. 🔴 **NOW**: Deploy Canary-500M or ≤2.8GB model
2. 🟡 **This Week**: Add VRAM monitoring/alerts
3. 🟢 **Next Sprint**: Test INT8 quantization

## Full Details
See: `/opt/swictation/hive/analysis/memory-pattern-analysis.md`
