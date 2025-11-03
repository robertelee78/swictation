# Swictation Installation Troubleshooting

## Python 3.13 Dependency Conflict

### Problem
If you're running Python 3.13+, you'll see this error:

```
ERROR: Cannot install ... because these package versions have conflicting dependencies.
The conflict is caused by:
    ml-dtypes 0.5.3 depends on numpy>=2.1.0; python_version >= "3.13"
    nemo-toolkit 2.5.2 depends on numpy>=1.22 (but <2.0.0)
```

### Root Cause
- Python 3.13 requires `numpy>=2.1.0` for `ml-dtypes`
- NeMo toolkit requires `numpy<2.0.0`
- This is a fundamental incompatibility

### Solution 1: Use Python 3.12 Virtual Environment (Recommended)

```bash
# Run the helper script
cd /opt/swictation
bash scripts/install-python312-venv.sh

# Then activate and continue
source venv/bin/activate
bash scripts/install.sh
```

### Solution 2: Manual Python 3.12 Setup

```bash
# Install Python 3.12
sudo apt install python3.12 python3.12-venv python3.12-dev

# Create virtual environment
cd /opt/swictation
python3.12 -m venv venv
source venv/bin/activate

# Verify Python version
python --version  # Should show 3.12.x

# Install dependencies
pip install --upgrade pip
pip install -r requirements.txt

# Continue with installation
bash scripts/install.sh
```

### Solution 3: Use pyenv

```bash
# Install pyenv
curl https://pyenv.run | bash

# Add to shell (follow pyenv instructions)
# Then:
pyenv install 3.12.7
cd /opt/swictation
pyenv local 3.12.7

# Create venv and install
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt
bash scripts/install.sh
```

## Ubuntu 25.04 Specific Issues

### Python 3.13 is Default
Ubuntu 25.04 ships with Python 3.13 as default. Always use Python 3.12 virtual environment.

### Package Installation
The install script now automatically installs Python 3.12:
```bash
sudo apt install python3.12 python3.12-venv python3.12-dev
```

## Common Issues

### Issue: "wtype not found"
**Solution:**
```bash
sudo apt install wtype wl-clipboard
```

### Issue: "CUDA not available"
**Symptom:** STT works but is slow

**Check:**
```bash
python3 -c "import torch; print(torch.cuda.is_available())"
```

**Solution:** Install NVIDIA drivers and CUDA toolkit:
```bash
# Check NVIDIA driver
nvidia-smi

# If not installed:
sudo apt install nvidia-driver-550 nvidia-utils-550
```

### Issue: "maturin build failed"
**Solution:** Install Rust build dependencies:
```bash
sudo apt install build-essential pkg-config libssl-dev
source ~/.cargo/env
```

### Issue: "transformer verification failed"
**Solution:** Rebuild transformer:
```bash
cd /opt/swictation/external/midstream/crates/text-transform
maturin build --release --features pyo3
pip install --force-reinstall ../../target/wheels/midstreamer_transform-*.whl
```

## Verification Steps

After installation, verify components:

```bash
# 1. Check Python version
python --version  # Should be 3.12.x

# 2. Test imports
python3 -c "import torch; import nemo; import midstreamer_transform; print('✓ All imports successful')"

# 3. Check CUDA
python3 -c "import torch; print(f'CUDA: {torch.cuda.is_available()}')"

# 4. Test transformer
python3 -c "import midstreamer_transform; print(midstreamer_transform.get_stats())"

# 5. Check daemon
python3 /opt/swictation/src/swictation_cli.py status
```

## Getting Help

If you're still having issues:

1. Check Python version: `python --version`
2. Check if in venv: `echo $VIRTUAL_ENV`
3. Verify requirements installed: `pip list | grep -E "(torch|nemo|numpy)"`
4. Check system: `uname -a`
5. Check logs: `journalctl --user -u swictation.service -n 50`

Report issues at: https://github.com/YOUR_REPO/issues
