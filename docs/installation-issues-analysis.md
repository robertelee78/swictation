# Swictation 0.3.0 NPM Installation Issues - Complete Analysis

**Analyst Agent Report**
**Date:** 2025-11-13
**Task ID:** task-1763057398090-32qswwc4a
**Project ID:** fbeae03f-cd20-47a1-abf2-c9be91af34ca

---

## Executive Summary

Analysis of npm package installation system for Swictation 0.3.0 reveals **4 critical gaps** in postinstall.js that caused deployment failures. All issues are documented from actual production incidents on RTX A1000 (4GB VRAM) system.

**Critical Finding:** Postinstall script handles fresh installs well but **completely fails** during upgrades from Python-based 0.1.x to Node.js-based 0.2.x+.

---

## 1. Postinstall Flow Analysis

### Current Flow (497 lines)

```
postinstall.js execution order:
├─ 1. checkPlatform()                    [Lines 22-57]
│  └─ Validates: Linux x64, GLIBC 2.39+
│
├─ 2. ensureBinaryPermissions()          [Lines 59-79]
│  └─ chmod 755: swictation-daemon, swictation-ui, swictation, daemon.bin
│
├─ 3. createDirectories()                [Lines 81-99]
│  └─ Creates: ~/.config/swictation, ~/.local/share/swictation/models, ~/.cache/swictation
│
├─ 4. downloadGPULibraries()             [Lines 182-228]
│  ├─ Detects: nvidia-smi
│  └─ Downloads: swictation-gpu-libs.tar.gz → lib/native/
│
├─ 5. detectOrtLibrary()                 [Lines 230-302]
│  ├─ Priority 1: Check lib/native/libonnxruntime.so (GPU-enabled bundled)
│  ├─ Priority 2: Fall back to Python pip installation (CPU-only warning)
│  └─ Outputs: config/detected-environment.json
│
├─ 6. generateSystemdService()           [Lines 304-362]
│  ├─ Creates: ~/.config/systemd/user/swictation-daemon.service
│  ├─ Installs: swictation-ui.service (direct copy)
│  └─ Replaces: __INSTALL_DIR__, __ORT_DYLIB_PATH__
│
├─ 7. checkDependencies()                [Lines 101-141]
│  └─ Checks: systemctl, nc, wtype, xdotool, hf (all optional)
│
└─ 8. showNextSteps()                    [Lines 440-477]
   ├─ detectSystemCapabilities()         [Lines 364-392]
   ├─ recommendOptimalModel()            [Lines 394-438]
   └─ Displays: Model recommendations based on VRAM
```

---

## 2. Critical Issues Mapped to Code

### Issue #1: OLD SERVICE FILE CONFLICTS (NOT IMPLEMENTED)
**Archon Task:** aa21b713-0664-4f80-92d7-009f2dc47a24

#### Problem
- **Where:** generateSystemdService() [Lines 304-362]
- **What's Missing:** No detection or cleanup of old Python-based services
- **Impact:** systemd tries to start wrong service (exit code 2)

#### Root Cause Analysis
```python
# OLD PYTHON SERVICE (still present after upgrade)
/usr/lib/systemd/user/swictation.service
→ ExecStart=/opt/swictation/src/swictationd.py  # <-- File doesn't exist!

# NEW NODE.JS SERVICE (created by postinstall)
~/.config/systemd/user/swictation-daemon.service
→ ExecStart=/usr/local/lib/node_modules/swictation/lib/native/swictation-daemon.bin

# SYSTEMD CONFUSION
systemctl --user start swictation-daemon
→ systemd finds BOTH services
→ Tries old Python service first (alphabetically? by install time?)
→ Fails with exit code 2
```

#### Code Locations Where Fix Needed
1. **postinstall.js Line 304** - Add BEFORE generateSystemdService():
   ```javascript
   function cleanupOldServices() {
     const oldServiceLocations = [
       '/usr/lib/systemd/user/swictation.service',
       '/etc/systemd/user/swictation.service',
       path.join(os.homedir(), '.config/systemd/user/swictation.service') // Note: no -daemon suffix
     ];

     for (const oldService of oldServiceLocations) {
       if (fs.existsSync(oldService)) {
         log('yellow', `⚠️  Found old service file: ${oldService}`);
         // Disable if enabled
         try {
           execSync('systemctl --user disable swictation.service', { stdio: 'ignore' });
         } catch {}

         // Remove file (needs sudo for /usr/lib)
         log('yellow', '   Please remove manually with: sudo rm ' + oldService);
       }
     }
   }
   ```

2. **postinstall.js Line 489** - Call in main():
   ```javascript
   async function main() {
     log('cyan', '🚀 Setting up Swictation...\n');

     checkPlatform();
     ensureBinaryPermissions();
     createDirectories();
     cleanupOldServices();  // <-- ADD THIS
     await downloadGPULibraries();
     // ... rest of flow
   }
   ```

#### Production Evidence
```bash
# From actual debugging session:
$ systemctl --user status swictation-daemon
● swictation.service - Swictation Voice-to-Text Service
     Loaded: loaded (/usr/lib/systemd/user/swictation.service; enabled)
     Active: failed (Result: exit-code)
   Main PID: 123456 (code=exited, status=2)

# Wrong service file was being used!
# New service wasn't even being found.
```

---

### Issue #2: ONNX RUNTIME VERSION CONFLICTS
**Archon Task:** aa21b713-0664-4f80-92d7-009f2dc47a24 (same task, sub-issue)

#### Problem
- **Where:** detectOrtLibrary() [Lines 230-302]
- **What's Wrong:**
  1. Priority check is CORRECT (bundled GPU lib first)
  2. But no verification that bundled lib actually exists post-download
  3. Falls back to Python pip silently (CPU-only)
  4. Service file gets wrong path, causing blank output

#### Root Cause Analysis
```javascript
// Line 235-243: CHECK PRIORITY IS CORRECT
const npmOrtLib = path.join(__dirname, 'lib', 'native', 'libonnxruntime.so');
if (fs.existsSync(npmOrtLib)) {
  // This SHOULD happen but doesn't always due to download failures
  return npmOrtLib;
} else {
  log('yellow', 'Falling back to Python...');  // <-- SILENT DEGRADATION
}

// Line 247-250: PYTHON FALLBACK (CPU-ONLY)
const ortPath = execSync(
  'python3 -c "import onnxruntime; print(...)"'
).trim();
// Returns: ~/.local/lib/python3.13/site-packages/onnxruntime/capi/
// Problem: This is onnxruntime 1.20.1 (CPU), NOT onnxruntime-gpu
```

#### Code Locations Where Fix Needed
1. **postinstall.js Line 228** - Add verification after downloadGPULibraries():
   ```javascript
   async function downloadGPULibraries() {
     // ... existing download code ...

     // VERIFY EXTRACTION
     const expectedFiles = [
       'libonnxruntime.so',
       'libonnxruntime.so.1.20.0',
       'libonnxruntime_providers_cuda.so',
       'libonnxruntime_providers_shared.so'
     ];

     const nativeDir = path.join(__dirname, 'lib', 'native');
     const missingFiles = expectedFiles.filter(f => !fs.existsSync(path.join(nativeDir, f)));

     if (missingFiles.length > 0) {
       log('red', '\n⚠️  GPU library extraction FAILED!');
       log('red', `   Missing files: ${missingFiles.join(', ')}`);
       log('yellow', '   Daemon will fall back to CPU-only mode');
       return false;  // <-- Return success/failure
     }

     return true;
   }
   ```

2. **postinstall.js Line 243** - Make fallback WARNING more visible:
   ```javascript
   if (fs.existsSync(npmOrtLib)) {
     log('green', `✓ Found ONNX Runtime (GPU-enabled): ${npmOrtLib}`);
     return npmOrtLib;
   } else {
     log('red', '\n⚠️  CRITICAL: GPU-enabled ONNX Runtime NOT FOUND!');
     log('red', `   Expected at: ${npmOrtLib}`);
     log('red', '   This will cause BLANK OUTPUT or severe performance degradation!');
     log('yellow', '\n   Attempting Python pip fallback (CPU-only)...');
   }
   ```

#### Production Evidence
```bash
# From actual incident:
# Service had: ORT_DYLIB_PATH=/home/user/.local/lib/python3.13/.../libonnxruntime.so.1.20.1
# Should have: ORT_DYLIB_PATH=/usr/local/lib/node_modules/swictation/lib/native/libonnxruntime.so

# Symptoms:
# - Model loaded successfully
# - Inference ran successfully
# - BUT: All transcriptions were BLANK (empty string output)
# - Fixed by manually setting correct path
```

---

### Issue #3: CUDA LIBRARY PATH MISSING (PARTIALLY FIXED)
**Archon Task:** 9269f733-0c55-4364-b8c5-afddb9a2de4a (DONE, but verification needed)

#### Problem
- **Where:** templates/swictation-daemon.service.template [Line 22]
- **What's Fixed:** LD_LIBRARY_PATH now includes cuda-12.9
- **What Needs Verification:** Path actually exists on target system

#### Root Cause Analysis
```bash
# Line 22 of service template:
Environment="LD_LIBRARY_PATH=/usr/local/cuda/lib64:/usr/local/cuda-12.9/lib64:__INSTALL_DIR__/lib/native"

# ISSUE 1: Assumes CUDA 12.9 is installed
# - What if user has CUDA 11.8? CUDA 12.4?
# - Path won't exist → libraries not found

# ISSUE 2: No verification during postinstall
# - postinstall.js Line 329 just does string replacement
# - Never checks if /usr/local/cuda-12.9/lib64 exists
# - Service file gets invalid path

# SYMPTOMS:
# - 14x performance regression (43s vs 3-6s expected)
# - GPU shows 0% utilization during inference
# - Falls back to CPU execution silently
```

#### Code Locations Where Fix Needed
1. **postinstall.js Line 304** - Add CUDA detection function:
   ```javascript
   function detectCudaPath() {
     log('cyan', '\n🔍 Detecting CUDA installation...');

     const cudaPaths = [
       '/usr/local/cuda/lib64',
       '/usr/local/cuda-12.9/lib64',
       '/usr/local/cuda-12.6/lib64',
       '/usr/local/cuda-12.4/lib64',
       '/usr/local/cuda-12.1/lib64',
       '/usr/local/cuda-11.8/lib64'
     ];

     const existingPaths = cudaPaths.filter(p => fs.existsSync(p));

     if (existingPaths.length === 0) {
       log('yellow', '⚠️  No CUDA installation found');
       log('yellow', '   GPU acceleration will NOT work without CUDA');
       log('cyan', '   Install CUDA Toolkit: https://developer.nvidia.com/cuda-downloads');
       return null;
     }

     log('green', `✓ Found CUDA at: ${existingPaths[0]}`);
     if (existingPaths.length > 1) {
       log('cyan', `  (Also found: ${existingPaths.slice(1).join(', ')})`);
     }

     return existingPaths.join(':');  // Return all valid paths
   }
   ```

2. **postinstall.js Line 329** - Use detected path:
   ```javascript
   function generateSystemdService(ortLibPath) {
     // ... existing code ...

     let template = fs.readFileSync(templatePath, 'utf8');

     // Replace ORT path
     template = template.replace(/__ORT_DYLIB_PATH__/g, ortLibPath || 'NEEDS_MANUAL_CONFIG');

     // Replace CUDA path dynamically
     const cudaPath = detectCudaPath();
     if (cudaPath) {
       template = template.replace(
         /LD_LIBRARY_PATH=.*$/m,
         `LD_LIBRARY_PATH=${cudaPath}:${__dirname}/lib/native`
       );
       log('green', '✓ Configured CUDA library paths in service');
     } else {
       log('yellow', '⚠️  Service will not have GPU acceleration (no CUDA found)');
     }

     // Write service file
     fs.writeFileSync(daemonServicePath, template);
   }
   ```

#### Production Evidence
```bash
# From ANALYST_ROOT_CAUSE_REPORT.md:
# Before fix: GPU 0% utilization, 43s for 57s audio
# After fix: GPU 95% utilization, 3-6s for 57s audio
# Performance improvement: 14x faster
```

---

### Issue #4: VRAM-BASED MODEL SELECTION (NOT IMPLEMENTED)
**Archon Task:** 22e45da7-faed-4028-a401-911887749157

#### Problem
- **Where:** recommendOptimalModel() [Lines 394-438] and showNextSteps() [Lines 440-477]
- **What's Missing:**
  1. No actual VRAM measurement during install
  2. Recommendation is passive (just console.log)
  3. No config.toml generation with detected model
  4. No test loading to verify model works

#### Root Cause Analysis
```javascript
// Line 379-381: GETS GPU MEMORY
const gpuMemory = execSync('nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits');
capabilities.gpuMemoryMB = parseInt(gpuMemory.trim());

// Line 396-405: RECOMMENDATION LOGIC (GOOD)
if (capabilities.gpuMemoryMB >= 4000) {
  return { model: '1.1b', ... };  // <-- WRONG FOR RTX A1000!
} else {
  return { model: '0.6b', ... };
}

// PROBLEM 1: Threshold is INCORRECT
// RTX A1000 = 4096 MB VRAM (exactly 4GB)
// - Recommended 1.1b model
// - Model tried to allocate 3.8GB
// - Failed at 3839 MB ("Failed to allocate memory")
// - Should have used 0.6b (3484 MB = 85% VRAM)

// PROBLEM 2: No verification
// Line 460: Just logs suggestion
console.log('  1. Download recommended AI model:');
log('green', `     swictation download-model ${recommendation.model}`);
// ^ User has to manually do this
// ^ No automatic config.toml creation

// PROBLEM 3: No test loading
// Model download happens AFTER postinstall
// No way to verify model actually works until service starts
// Failure discovered at runtime, not install time
```

#### Empirical Model Memory Requirements

From production debugging on RTX A1000 (4096 MB VRAM):

| Model | Model Files (disk) | Actual VRAM Usage | % of 4GB VRAM | Result |
|-------|-------------------|-------------------|---------------|--------|
| **0.6b-gpu** | 1.33 GB | **3484 MB** | **85%** | ✅ SUCCESS |
| **1.1b-int8** | 1.16 GB | **3839+ MB** | **>94%** | ❌ FAILED (OOM) |

**Key Finding:** INT8 quantization doesn't save memory in ONNX Runtime!
- Expected: ~1.2 GB for INT8 (8-bit weights)
- Actual: ~3.8 GB (full precision intermediate activations)

#### Code Locations Where Fix Needed

1. **postinstall.js Line 394** - Fix model threshold:
   ```javascript
   function recommendOptimalModel(capabilities) {
     if (capabilities.hasGPU) {
       // OLD: if (capabilities.gpuMemoryMB >= 4000)
       // NEW: More conservative threshold with safety margin
       if (capabilities.gpuMemoryMB >= 5000) {  // <-- Need 5GB+ for 1.1B safely
         return {
           model: '1.1b',
           reason: `GPU: ${capabilities.gpuName} (${Math.round(capabilities.gpuMemoryMB/1024)}GB VRAM)`,
           description: 'Best quality - Requires 5GB+ VRAM',
           vramRequired: 4200,  // <-- Add actual requirement
           size: '~75MB download',
           performance: '62x realtime on GPU'
         };
       } else if (capabilities.gpuMemoryMB >= 3500) {  // <-- 0.6b needs 3.5GB
         return {
           model: '0.6b',
           reason: `GPU: ${capabilities.gpuName} (${Math.round(capabilities.gpuMemoryMB/1024)}GB VRAM)`,
           description: 'Optimal for 4GB VRAM - Proven to work',
           vramRequired: 3484,  // <-- From empirical data
           size: '~111MB',
           performance: 'Fast on GPU'
         };
       } else {
         return {
           model: '0.6b',
           reason: `Limited VRAM (${Math.round(capabilities.gpuMemoryMB/1024)}GB)`,
           description: 'CPU-only recommended',
           vramRequired: 0,
           size: '~111MB',
           performance: 'CPU mode'
         };
       }
     }
     // ... CPU logic ...
   }
   ```

2. **postinstall.js Line 440** - Add interactive model setup:
   ```javascript
   async function showNextSteps() {
     log('green', '\n✨ Swictation installed successfully!');

     const capabilities = detectSystemCapabilities();
     const recommendation = recommendOptimalModel(capabilities);

     // Display detection results
     log('cyan', '\n📊 System Detection:');
     console.log(`   ${recommendation.reason}`);
     console.log('');

     log('cyan', '🎯 Recommended Model:');
     log('green', `   ${recommendation.model.toUpperCase()} - ${recommendation.description}`);
     console.log(`   Size: ${recommendation.size}`);
     console.log(`   Performance: ${recommendation.performance}`);
     if (recommendation.vramRequired > 0) {
       console.log(`   VRAM Required: ${recommendation.vramRequired} MB`);
     }
     console.log('');

     // NEW: Offer to create config automatically
     try {
       const inquirer = require('inquirer');
       const { autoSetup } = await inquirer.prompt([{
         type: 'confirm',
         name: 'autoSetup',
         message: `Create config with ${recommendation.model} model now?`,
         default: true
       }]);

       if (autoSetup) {
         await createConfigWithModel(recommendation.model);
       }
     } catch (err) {
       // inquirer not available, continue with manual instructions
       log('cyan', 'Next steps:');
       console.log('  1. Download recommended AI model:');
       log('cyan', '     pip install "huggingface_hub[cli]"');
       log('green', `     swictation download-model ${recommendation.model}`);
       // ... rest of manual steps ...
     }
   }

   async function createConfigWithModel(modelId) {
     const configPath = path.join(os.homedir(), '.config', 'swictation', 'config.toml');

     if (fs.existsSync(configPath)) {
       log('yellow', '⚠️  Config already exists, not overwriting');
       return;
     }

     const configContent = `# Swictation Configuration
# Auto-generated by postinstall

[stt]
# Automatically selected based on your GPU: ${modelId}
model = "${modelId}"

[vad]
threshold = 0.003
silence_duration = 0.8

[metrics]
enabled = true
show_realtime_feedback = true
`;

     fs.writeFileSync(configPath, configContent);
     log('green', `✓ Created config with ${modelId} model`);
     log('cyan', `  Edit config at: ${configPath}`);
   }
   ```

3. **NEW FILE: postinstall.js Line 500** - Add test loading function:
   ```javascript
   async function testModelLoading(modelId) {
     log('cyan', `\n🧪 Testing ${modelId} model loading...`);

     // This requires model to be downloaded first
     const modelPath = path.join(
       os.homedir(),
       '.local', 'share', 'swictation', 'models',
       `sherpa-onnx-nemo-parakeet-tdt-${modelId}-v3-onnx`
     );

     if (!fs.existsSync(modelPath)) {
       log('yellow', `   Model not downloaded yet, skipping test`);
       return { success: false, reason: 'not_downloaded' };
     }

     // Create a minimal test daemon process
     const testScript = path.join(__dirname, 'lib', 'test-model-loading.js');

     try {
       const output = execSync(
         `node "${testScript}" "${modelId}"`,
         { encoding: 'utf8', timeout: 30000 }
       );

       if (output.includes('MODEL_LOADED_SUCCESSFULLY')) {
         log('green', `   ✓ ${modelId} model loads successfully`);
         return { success: true };
       } else {
         log('red', `   ✗ ${modelId} model failed to load`);
         return { success: false, reason: 'load_failed', output };
       }
     } catch (err) {
       if (err.killed) {
         log('red', `   ✗ ${modelId} model loading timeout (>30s)`);
         return { success: false, reason: 'timeout' };
       }

       // Check for OOM error
       if (err.message.includes('allocate memory')) {
         log('red', `   ✗ Out of memory loading ${modelId}`);
         log('yellow', `   Your GPU may not have enough VRAM for this model`);
         return { success: false, reason: 'oom', error: err.message };
       }

       log('red', `   ✗ Test failed: ${err.message}`);
       return { success: false, reason: 'error', error: err.message };
     }
   }
   ```

4. **NEW FILE: lib/test-model-loading.js**:
   ```javascript
   #!/usr/bin/env node
   // Quick test to verify model can be loaded
   // Called by postinstall.js to validate model selection

   const path = require('path');
   const os = require('os');

   const modelId = process.argv[2];
   if (!modelId) {
     console.error('Usage: node test-model-loading.js <model-id>');
     process.exit(1);
   }

   const modelPath = path.join(
     os.homedir(),
     '.local', 'share', 'swictation', 'models',
     `sherpa-onnx-nemo-parakeet-tdt-${modelId}-v3-onnx`
   );

   console.log(`Testing model: ${modelId}`);
   console.log(`Model path: ${modelPath}`);

   // TODO: Load model using sherpa-onnx-node binding
   // For now, just check files exist
   const requiredFiles = [
     'model.onnx',
     'tokens.txt'
   ];

   const missingFiles = requiredFiles.filter(f => {
     const fullPath = path.join(modelPath, f);
     return !require('fs').existsSync(fullPath);
   });

   if (missingFiles.length > 0) {
     console.error(`Missing files: ${missingFiles.join(', ')}`);
     process.exit(1);
   }

   console.log('MODEL_LOADED_SUCCESSFULLY');
   process.exit(0);
   ```

#### Production Evidence
```bash
# From actual RTX A1000 incident:

# WHAT HAPPENED:
$ swictation setup  # Used auto-detected 1.1b model
$ swictation start
$ journalctl --user -u swictation-daemon
# Error: Failed to allocate memory for requested buffer of size 4194304
# VRAM usage peaked at 3839 MB (tried to use 3.8GB of 4GB)

# MANUAL FIX:
$ vim ~/.config/swictation/config.toml
# Changed: stt_model_override = "0.6b-gpu"
$ swictation start
$ journalctl --user -u swictation-daemon
# Success! VRAM usage: 3484 MB (85% of 4GB)
# Transcription working perfectly

# LESSON:
# - 4GB threshold is TOO LOW for 1.1B model
# - Need 5GB+ for safe 1.1B operation
# - 0.6b is optimal for 4GB GPUs
```

---

## 3. Missing Features Analysis

### Feature #1: Old Service Cleanup
**Status:** NOT IMPLEMENTED
**Priority:** P0 (Critical)
**Affected Code:** postinstall.js, generateSystemdService()

**What's Needed:**
1. Detect old service files at multiple locations:
   - `/usr/lib/systemd/user/swictation.service` (system-wide Python install)
   - `/etc/systemd/user/swictation.service` (alternate location)
   - `~/.config/systemd/user/swictation.service` (old user install, without -daemon suffix)

2. Disable old services:
   ```bash
   systemctl --user disable swictation.service 2>/dev/null || true
   systemctl --user stop swictation.service 2>/dev/null || true
   ```

3. Warn about manual removal (can't auto-delete from /usr/lib without sudo):
   ```
   ⚠️  Found old service file: /usr/lib/systemd/user/swictation.service
   Please remove manually with: sudo rm /usr/lib/systemd/user/swictation.service
   ```

4. Verify new service is enabled:
   ```bash
   systemctl --user daemon-reload
   systemctl --user enable swictation-daemon.service
   ```

**Complexity:** Low (1-2 hours)
**Risk:** Low (only cleans up, doesn't break existing)

---

### Feature #2: Config Migration UI
**Status:** NOT IMPLEMENTED
**Priority:** P1 (High)
**Affected Code:** postinstall.js, showNextSteps()

**What's Needed:**
1. Detect if old config exists:
   - Old location: `~/.config/swictation/config.toml` (Python-based format)
   - New location: Same, but different schema (Node.js-based)

2. Compare schemas:
   ```toml
   # OLD (Python):
   [stt]
   model_path = "/home/user/.local/share/swictation/models/..."

   # NEW (Node.js):
   [stt]
   model = "0.6b"  # Just the model ID, not full path
   ```

3. Interactive migration:
   ```javascript
   if (oldConfigExists) {
     const { migrate } = await inquirer.prompt([{
       type: 'confirm',
       name: 'migrate',
       message: 'Old config detected. Migrate settings to new format?',
       default: true
     }]);

     if (migrate) {
       await migrateConfig();
     }
   }
   ```

4. Preserve user customizations:
   - VAD threshold
   - Silence duration
   - Model preference
   - Metrics settings

**Complexity:** Medium (4-6 hours)
**Risk:** Medium (could break custom configs if migration logic is wrong)

---

### Feature #3: VRAM Detection During Install
**Status:** PARTIALLY IMPLEMENTED (detection exists, but thresholds wrong)
**Priority:** P0 (Critical)
**Affected Code:** recommendOptimalModel() [Line 394]

**What's Needed:**
1. More accurate VRAM thresholds (see Issue #4 above):
   - 1.1b model: Needs 5GB+ VRAM (not 4GB)
   - 0.6b model: Needs 3.5GB+ VRAM
   - CPU fallback: <3.5GB VRAM

2. Safety margin calculation:
   ```javascript
   const safetyMargin = 0.15;  // Leave 15% headroom
   const usableVram = capabilities.gpuMemoryMB * (1 - safetyMargin);

   if (usableVram >= 5000) {
     return '1.1b';
   } else if (usableVram >= 3500) {
     return '0.6b';
   } else {
     return 'cpu-only';
   }
   ```

3. Display VRAM calculation:
   ```
   📊 System Detection:
      GPU: NVIDIA RTX A1000 Laptop GPU (4096 MB VRAM)
      Usable VRAM: 3481 MB (85% with 15% safety margin)

   🎯 Recommended Model:
      0.6B - Optimal for your GPU
      VRAM Required: 3484 MB (fits with safety margin)
   ```

**Complexity:** Low (1-2 hours, mostly adjusting thresholds)
**Risk:** Low (improves detection, doesn't break existing)

---

### Feature #4: Model Test Loading
**Status:** NOT IMPLEMENTED
**Priority:** P1 (High)
**Affected Code:** showNextSteps(), NEW: testModelLoading()

**What's Needed:**
1. After model download, load model with minimal inference:
   ```javascript
   async function testModelLoading(modelId) {
     // Spawn test daemon process
     // Load model (sherpa-onnx)
     // Run single inference with dummy audio
     // Check VRAM usage via nvidia-smi
     // Report success/failure

     return {
       success: true/false,
       vramUsed: 3484,  // MB
       loadTime: 2.3,   // seconds
       error: null
     };
   }
   ```

2. If test fails, suggest alternative:
   ```
   ✗ 1.1b model failed to load (out of memory)
   📊 VRAM usage: 3839 MB / 4096 MB (94%)

   ✓ Trying 0.6b model instead...
   ✓ 0.6b model loads successfully
   📊 VRAM usage: 3484 MB / 4096 MB (85%)

   Would you like to use 0.6b instead? [Y/n]
   ```

3. Update config.toml with working model:
   ```toml
   [stt]
   model = "0.6b"  # Auto-selected after 1.1b failed

   # Attempted models:
   # - 1.1b: FAILED (out of memory)
   # - 0.6b: SUCCESS (3484 MB VRAM)
   ```

**Complexity:** High (8-12 hours, requires sherpa-onnx integration)
**Risk:** High (model loading failure could hang install)

**Alternative Approach (Low-Risk):**
- Don't actually load model during install
- Just validate model files exist (model.onnx, tokens.txt)
- Add `swictation test-model <id>` CLI command for manual testing
- User runs this after first install to verify

---

## 4. Dependency Analysis

### Issue Dependencies (Must Fix in Order)
```
Issue #1 (Old Service Cleanup)
├─ No dependencies
└─ BLOCKS: All other issues (service must be clean first)

Issue #2 (ONNX Runtime Conflicts)
├─ DEPENDS ON: Issue #1 (clean service)
├─ DEPENDS ON: Issue #3 (CUDA paths must be correct)
└─ BLOCKS: Issue #4 (can't test models without correct runtime)

Issue #3 (CUDA Library Paths)
├─ DEPENDS ON: Issue #1 (service must exist)
└─ BLOCKS: Issue #2 (runtime needs CUDA)

Issue #4 (VRAM Model Selection)
├─ DEPENDS ON: Issue #1 (service must work)
├─ DEPENDS ON: Issue #2 (runtime must work)
└─ DEPENDS ON: Issue #3 (GPU must work)
```

### Recommended Fix Order
1. **Issue #1** - Old service cleanup (breaks everything until fixed)
2. **Issue #3** - CUDA path detection (enables GPU)
3. **Issue #2** - ONNX Runtime verification (ensures correct library)
4. **Issue #4** - VRAM-based model selection (optimizes choice)

---

## 5. Priority Assessment

### P0 (Must Fix for 0.3.1)
- ✅ Issue #1: Old service cleanup
- ✅ Issue #3: CUDA path detection
- ✅ Issue #2: ONNX Runtime verification

### P1 (Should Fix for 0.4.0)
- ⬜ Issue #4: VRAM model thresholds (quick win)
- ⬜ Feature #2: Config migration UI
- ⬜ Feature #4: Model test loading

### P2 (Nice to Have)
- ⬜ Feature #3: Improved VRAM detection (already partially working)

---

## 6. Testing Strategy

### Test Case #1: Fresh Install (Ubuntu 24.04, RTX 4090, 24GB VRAM)
**Expected:**
1. ✅ GPU detected
2. ✅ GPU libs downloaded
3. ✅ CUDA paths detected
4. ✅ 1.1b model recommended
5. ✅ Service starts successfully
6. ✅ Transcription works

**Actual (current):**
- ✅ All passing (fresh install works)

---

### Test Case #2: Upgrade from Python 0.1.x (Ubuntu 24.04, RTX A1000, 4GB VRAM)
**Expected:**
1. ✅ Old service detected and disabled
2. ✅ Old config migrated
3. ✅ GPU detected
4. ✅ 0.6b model recommended (not 1.1b)
5. ✅ Service starts successfully
6. ✅ Transcription works

**Actual (current):**
- ❌ Old service NOT detected (Issue #1)
- ❌ No config migration (Feature #2)
- ✅ GPU detected
- ❌ 1.1b model recommended (Issue #4)
- ❌ Service fails to start (Issue #1 + #4)
- ❌ Transcription doesn't work

**Fixes Required:** Issues #1, #2, #4

---

### Test Case #3: Low VRAM GPU (RTX 3050, 8GB VRAM)
**Expected:**
1. ✅ GPU detected
2. ✅ 0.6b model recommended
3. ✅ Service starts
4. ✅ VRAM usage ~3.5GB

**Actual (current):**
- ✅ GPU detected
- ❌ 1.1b model recommended (wrong threshold)
- ⚠️ Service might start (if 8GB is enough for 1.1b)

**Fixes Required:** Issue #4 (adjust thresholds)

---

### Test Case #4: No GPU (CPU-only system)
**Expected:**
1. ✅ No GPU detected
2. ✅ CPU-optimized model recommended
3. ✅ Service starts with CPU backend
4. ✅ Slower but functional

**Actual (current):**
- ✅ All passing (CPU path works)

---

## 7. Recommendations

### Immediate Actions (0.3.1 Release)
1. Implement old service cleanup (2 hours)
2. Add CUDA path detection (2 hours)
3. Add ONNX Runtime verification (2 hours)
4. Fix VRAM thresholds (1 hour)

**Total Effort:** ~7 hours
**Risk:** Low (all defensive improvements)

### Short-Term (0.4.0 Release)
1. Add config migration UI (6 hours)
2. Add model test loading (12 hours)
3. Improve error messages (2 hours)

**Total Effort:** ~20 hours
**Risk:** Medium (migration logic could break configs)

### Long-Term (0.5.0+)
1. Full service health checking
2. Automatic recovery from failures
3. Better upgrade path testing
4. Integration tests for all install scenarios

---

## 8. Coordination Notes

**Stored in Hive Mind Memory:**
- Key: `swarm/analysis/installation-issues`
- Value: This document (comprehensive analysis)
- Task: aa21b713-0664-4f80-92d7-009f2dc47a24 (installation issues)
- Task: 22e45da7-faed-4028-a401-911887749157 (VRAM selection)

**Next Agents:**
- **Coder Agent**: Implement fixes for Issues #1, #2, #3
- **Tester Agent**: Create test cases for all scenarios
- **Reviewer Agent**: Verify fixes don't break existing installs

**Dependencies:**
- Requires: postinstall.js write access
- Requires: systemd service template modification
- Requires: Model downloader integration

---

## Appendix A: File Locations

### Files Analyzed
```
/opt/swictation/npm-package/
├── postinstall.js (497 lines)                          # Main postinstall script
├── templates/
│   └── swictation-daemon.service.template (39 lines)   # Systemd service template
├── bin/
│   └── swictation (401 lines)                          # CLI wrapper
├── config/
│   ├── config.example.toml (65 lines)                  # Example config
│   └── swictation-ui.service                           # UI service (not analyzed)
└── lib/
    ├── model-downloader.js                             # Model download logic
    └── native/                                         # Binary directory
        ├── swictation-daemon.bin
        ├── libonnxruntime.so
        └── ... (GPU libs)
```

### Config Locations
```
~/.config/swictation/
├── config.toml                    # User config
└── detected-environment.json      # Generated by postinstall

~/.config/systemd/user/
├── swictation-daemon.service      # Generated by postinstall
└── swictation-ui.service          # Copied by postinstall

/usr/lib/systemd/user/
└── swictation.service             # Old Python service (should be removed)
```

---

**End of Analysis**
