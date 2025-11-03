# Swictation Installation Scripts

## Quick Start

### For Python 3.12 Systems (Recommended)
```bash
cd /opt/swictation
bash scripts/install.sh
```

### For Python 3.13+ Systems
```bash
# First, create Python 3.12 virtual environment
cd /opt/swictation
bash scripts/install-python312-venv.sh

# Then activate and install
source venv/bin/activate
bash scripts/install.sh
```

## Scripts Overview

### `install.sh` - Main Installation Script
Complete one-command installation that:
- Checks Python version compatibility (warns on 3.13+)
- Installs system dependencies (wtype, wl-clipboard, ffmpeg, etc.)
- Initializes git submodules (MidStream)
- Installs Rust toolchain if needed
- Builds and installs MidStream text transformer
- Installs Python packages
- Downloads NVIDIA Canary-1B-Flash model
- Sets up systemd service
- Configures Sway keybinding

**Requirements:**
- Python 3.12.x (3.13+ not supported due to numpy conflicts)
- Ubuntu/Debian/Arch/Fedora Linux
- Wayland compositor (Sway recommended)

### `install-python312-venv.sh` - Python 3.12 Venv Helper
Creates a Python 3.12 virtual environment for systems running Python 3.13+.

**What it does:**
- Installs Python 3.12 if not present
- Creates virtual environment at `/opt/swictation/venv`
- Installs all Python dependencies
- Provides activation instructions

**Usage:**
```bash
bash scripts/install-python312-venv.sh
source /opt/swictation/venv/bin/activate
```

### `setup-sway.sh` - Sway Configuration
Adds Swictation keybinding to Sway config.

**Default binding:** `$mod+Shift+d` (toggle recording)

### `install-systemd-service.sh` - Systemd Service Setup
Installs and enables systemd user service for Swictation daemon.

## Python Version Compatibility

| Python Version | Status | Notes |
|---------------|--------|-------|
| 3.12.x | ✅ Recommended | Fully tested and supported |
| 3.13.x | ❌ Not compatible | numpy dependency conflict |
| 3.11.x | ⚠️ May work | Not officially tested |

### Why Python 3.13 Doesn't Work

Python 3.13 requires `numpy>=2.1.0`, but NeMo toolkit requires `numpy<2.0.0`. This is a fundamental incompatibility.

**Error you'll see:**
```
ERROR: Cannot install ... because these package versions have conflicting dependencies.
The conflict is caused by:
    ml-dtypes 0.5.3 depends on numpy>=2.1.0; python_version >= "3.13"
```

**Solution:** Use Python 3.12 virtual environment (see above)

## Troubleshooting

See [INSTALL_TROUBLESHOOTING.md](../docs/INSTALL_TROUBLESHOOTING.md) for:
- Python 3.13 dependency conflicts
- CUDA setup issues
- Maturin build failures
- Common installation problems

## Manual Installation Steps

If automated install fails, follow these steps:

### 1. System Dependencies
```bash
# Ubuntu/Debian
sudo apt install python3.12 python3.12-venv python3.12-dev \
    wtype wl-clipboard ffmpeg pipewire \
    build-essential pkg-config libssl-dev

# Arch
sudo pacman -S python312 wtype wl-clipboard ffmpeg pipewire
```

### 2. Rust Toolchain
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 3. Python Environment
```bash
cd /opt/swictation
python3.12 -m venv venv
source venv/bin/activate
pip install --upgrade pip
pip install -r requirements.txt
```

### 4. Build Transformer
```bash
pip install maturin
cd external/midstream/crates/text-transform
maturin build --release --features pyo3
pip install --force-reinstall ../../target/wheels/midstreamer_transform-*.whl
```

### 5. Download Model
```bash
python3 -c "from nemo.collections.asr.models import EncDecMultiTaskModel; EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')"
```

### 6. Setup Service
```bash
mkdir -p ~/.config/systemd/user
cp config/swictation.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now swictation.service
```

## Post-Installation

### Verify Installation
```bash
# Check Python version
python --version  # Should be 3.12.x

# Test components
python3 -c "import torch; import nemo; import midstreamer_transform"

# Check daemon status
python3 src/swictation_cli.py status
```

### Start Using
1. Start daemon: `systemctl --user start swictation.service`
2. Open any text editor
3. Press `$mod+Shift+d` to start recording
4. Speak continuously
5. Press `$mod+Shift+d` to stop

## Support

- **Documentation:** `/opt/swictation/docs/`
- **Issues:** Check logs with `journalctl --user -u swictation.service -f`
- **Testing:** Run `python3 src/swictation_cli.py test`
