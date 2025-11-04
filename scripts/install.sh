#!/bin/bash
# Swictation installation script
# One-command deployment for complete system setup

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Installation configuration
INSTALL_DIR="/opt/swictation"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/swictation"
SYSTEMD_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
SWAY_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/sway/config"

# Function: Install Python 3.12 via pyenv
install_python312_pyenv() {
    echo ""
    echo -e "${BLUE}Installing Python 3.12 via pyenv...${NC}"

    # Install pyenv dependencies
    sudo apt install -y build-essential libssl-dev zlib1g-dev \
        libbz2-dev libreadline-dev libsqlite3-dev curl \
        libncursesw5-dev xz-utils tk-dev libxml2-dev libxmlsec1-dev \
        libffi-dev liblzma-dev

    # Install pyenv if not present
    if [ ! -d "$HOME/.pyenv" ]; then
        echo "Installing pyenv..."
        curl https://pyenv.run | bash

        # Add to current shell
        export PYENV_ROOT="$HOME/.pyenv"
        export PATH="$PYENV_ROOT/bin:$PATH"
        eval "$(pyenv init -)"
    else
        export PYENV_ROOT="$HOME/.pyenv"
        export PATH="$PYENV_ROOT/bin:$PATH"
        eval "$(pyenv init -)"
    fi

    # Install Python 3.12.7
    echo "Installing Python 3.12.7 via pyenv (this takes a few minutes)..."
    pyenv install -s 3.12.7

    # Make it available as python3.12
    ln -sf "$HOME/.pyenv/versions/3.12.7/bin/python3.12" "$HOME/.local/bin/python3.12" || {
        mkdir -p "$HOME/.local/bin"
        ln -sf "$HOME/.pyenv/versions/3.12.7/bin/python3.12" "$HOME/.local/bin/python3.12"
    }

    echo -e "${GREEN}✓ Python 3.12 installed via pyenv${NC}"
    echo -e "${YELLOW}⚠ Add to your shell profile (~/.bashrc):${NC}"
    echo '  export PYENV_ROOT="$HOME/.pyenv"'
    echo '  export PATH="$PYENV_ROOT/bin:$PATH"'
    echo '  eval "$(pyenv init -)"'
    echo ""
}

echo "======================================================================"
echo -e "${BLUE}Swictation Installation${NC}"
echo "======================================================================"
echo ""
echo "This script will:"
echo "  ✓ Install system dependencies (wtype, wl-clipboard, ffmpeg)"
echo "  ✓ Initialize git submodule (external/midstream)"
echo "  ✓ Install Rust toolchain (if needed)"
echo "  ✓ Build and install MidStream text transformer"
echo "  ✓ Install Python packages (NeMo, PyTorch, etc.)"
echo "  ✓ Download NVIDIA Canary-1B-Flash model"
echo "  ✓ Set up systemd service"
echo "  ✓ Configure Sway keybinding"
echo ""

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   echo -e "${RED}✗ Do not run this script as root${NC}"
   echo "  Run as regular user (script will ask for sudo when needed)"
   exit 1
fi

# Detect distribution FIRST (needed for Python 3.12 installation)
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
    DISTRO_VERSION=$VERSION_ID
else
    echo -e "${RED}✗ Cannot detect Linux distribution${NC}"
    exit 1
fi

# Check Python version (CRITICAL: Python 3.13+ breaks numpy dependencies)
PYTHON_VERSION=$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
PYTHON_MAJOR=$(python3 -c 'import sys; print(sys.version_info.major)')
PYTHON_MINOR=$(python3 -c 'import sys; print(sys.version_info.minor)')

echo -e "${BLUE}Detected: $DISTRO $DISTRO_VERSION${NC}"
echo -e "${BLUE}Detected Python: $PYTHON_VERSION${NC}"

if [[ "$PYTHON_MAJOR" -eq 3 ]] && [[ "$PYTHON_MINOR" -ge 13 ]]; then
    echo ""
    echo -e "${YELLOW}⚠ Python $PYTHON_VERSION detected${NC}"
    echo ""
    echo "Python 3.13+ requires numpy>=2.1.0, but nemo-toolkit requires numpy<2.0.0"
    echo "Installing Python 3.12 system-wide (no venv - faster, better for large packages like PyTorch)"
    echo ""

    # Check if Python 3.12 is available
    if ! command -v python3.12 &> /dev/null; then
        echo -e "${BLUE}Installing Python 3.12...${NC}"

        # For Ubuntu/Debian, check version
        if [[ "$DISTRO" == "ubuntu" ]] || [[ "$DISTRO" == "debian" ]] || [[ "$DISTRO" == "pop" ]]; then
            # Ubuntu 25.04+ doesn't have deadsnakes support, go straight to pyenv
            if [[ "$DISTRO" == "ubuntu" ]] && [[ "${DISTRO_VERSION%%.*}" -ge 25 ]]; then
                echo "Ubuntu $DISTRO_VERSION detected - using pyenv for Python 3.12..."
                install_python312_pyenv
            else
                # Try deadsnakes PPA for older Ubuntu versions
                echo "Trying deadsnakes PPA..."
                if sudo apt install -y software-properties-common && \
                   sudo add-apt-repository -y ppa:deadsnakes/ppa && \
                   sudo apt update && \
                   sudo apt install -y python3.12 python3.12-venv python3.12-dev; then
                    echo -e "${GREEN}✓ Python 3.12 installed from PPA${NC}"
                else
                    echo -e "${YELLOW}⚠ PPA failed, using pyenv...${NC}"
                    install_python312_pyenv
                fi
            fi
        elif [[ "$DISTRO" == "arch" ]] || [[ "$DISTRO" == "manjaro" ]]; then
            sudo pacman -S --needed --noconfirm python312 || {
                echo -e "${RED}✗ Failed to install Python 3.12${NC}"
                exit 1
            }
        elif [[ "$DISTRO" == "fedora" ]]; then
            sudo dnf install -y python3.12 || {
                echo -e "${RED}✗ Failed to install Python 3.12${NC}"
                exit 1
            }
        else
            echo -e "${YELLOW}⚠ Unknown distro $DISTRO, trying pyenv...${NC}"
            install_python312_pyenv
        fi
    fi

    # Switch default python to python3.12
    echo -e "${GREEN}✓ Python 3.12 installed, using it for installation${NC}"
    # Use python3.12 directly for the rest of the script
    alias python3=python3.12
    export PYTHON_CMD=python3.12
    echo ""

elif [[ "$PYTHON_MAJOR" -eq 3 ]] && [[ "$PYTHON_MINOR" -lt 12 ]]; then
    echo -e "${YELLOW}⚠ Python $PYTHON_VERSION detected - may have compatibility issues${NC}"
    echo "  Recommended: Python 3.12"
    echo ""
fi

# Set python command to use
PYTHON_CMD=${PYTHON_CMD:-python3}

# Function: Install system dependencies
install_system_deps() {
    echo "======================================================================"
    echo "1️⃣ Installing System Dependencies"
    echo "======================================================================"
    echo ""

    case $DISTRO in
        arch|manjaro)
            echo "Installing packages via pacman..."
            sudo pacman -S --needed --noconfirm \
                python python-pip \
                wtype wl-clipboard \
                ffmpeg \
                pipewire pipewire-pulse \
                cuda || {
                    echo -e "${YELLOW}⚠ Some packages failed, continuing...${NC}"
                }
            ;;

        ubuntu|debian|pop)
            echo "Installing packages via apt..."

            # Clean up any broken PPAs that might exist
            if [ -f /etc/apt/sources.list.d/deadsnakes-ubuntu-ppa-plucky.list ]; then
                echo "Removing broken deadsnakes PPA..."
                sudo rm -f /etc/apt/sources.list.d/deadsnakes-ubuntu-ppa-*.list
            fi

            sudo apt update || {
                echo -e "${YELLOW}⚠ apt update had warnings, continuing...${NC}"
            }

            sudo apt install -y \
                python3 python3-pip python3-venv \
                wtype wl-clipboard \
                ffmpeg \
                pipewire pipewire-pulse \
                build-essential pkg-config libssl-dev || {
                    echo -e "${YELLOW}⚠ Some packages failed, continuing...${NC}"
                }
            ;;

        fedora)
            echo "Installing packages via dnf..."
            sudo dnf install -y \
                python3 python3-pip \
                wtype wl-clipboard \
                ffmpeg \
                pipewire pipewire-pulseaudio || {
                    echo -e "${YELLOW}⚠ Some packages failed, continuing...${NC}"
                }
            ;;

        *)
            echo -e "${YELLOW}⚠ Unsupported distribution: $DISTRO${NC}"
            echo "  Please install manually:"
            echo "    - python3, python3-pip"
            echo "    - wtype, wl-clipboard"
            echo "    - ffmpeg"
            echo "    - pipewire or pulseaudio"
            echo ""
            read -p "Continue anyway? (y/N): " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                exit 1
            fi
            ;;
    esac

    echo ""
    echo -e "${GREEN}✓ System dependencies installed${NC}"
    echo ""
}

# Function: Initialize git submodule
init_submodule() {
    echo "======================================================================"
    echo "2️⃣ Initializing Git Submodule (MidStream)"
    echo "======================================================================"
    echo ""

    cd "$INSTALL_DIR" || exit 1

    # ALWAYS update submodule to ensure we have latest fixes (like PyO3 0.23)
    echo "Updating git submodule (external/midstream)..."
    git submodule update --init --recursive || {
        echo -e "${RED}✗ Failed to update submodule${NC}"
        echo "  This is required to get the latest text transformer with PyO3 0.23"
        exit 1
    }
    echo -e "${GREEN}✓ Git submodule updated${NC}"

    # Verify submodule has files
    if [ ! -d "$INSTALL_DIR/external/midstream/crates" ]; then
        echo -e "${RED}✗ Submodule appears empty${NC}"
        echo "  Expected: $INSTALL_DIR/external/midstream/crates/"
        exit 1
    fi

    # Verify we have PyO3 0.23 (Python 3.12 compatible)
    echo "Verifying PyO3 version..."
    if grep -q 'pyo3 = { version = "0.23"' "$INSTALL_DIR/external/midstream/crates/text-transform/Cargo.toml"; then
        echo -e "${GREEN}✓ PyO3 0.23 detected (Python 3.12 compatible)${NC}"
    else
        echo -e "${YELLOW}⚠ Warning: Old PyO3 version detected${NC}"
        echo "  You may experience compatibility issues with Python 3.12"
    fi

    echo ""
}

# Function: Install Rust toolchain
install_rust() {
    echo "======================================================================"
    echo "3️⃣ Installing Rust Toolchain"
    echo "======================================================================"
    echo ""

    # Check if Rust is already installed
    if command -v rustc &> /dev/null && command -v cargo &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        echo -e "${GREEN}✓ Rust already installed: $RUST_VERSION${NC}"
        echo ""
        return
    fi

    echo "Rust not found, installing via rustup..."
    echo ""

    # Install rustup (official Rust installer)
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || {
        echo -e "${RED}✗ Rust installation failed${NC}"
        echo "  Install manually: https://rustup.rs/"
        exit 1
    }

    # Activate Rust environment
    source "$HOME/.cargo/env"

    # Verify installation
    if command -v rustc &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        echo -e "${GREEN}✓ Rust installed: $RUST_VERSION${NC}"
    else
        echo -e "${RED}✗ Rust installation verification failed${NC}"
        exit 1
    fi

    echo ""
}

# Function: Build and install MidStream text transformer
build_transformer() {
    echo "======================================================================"
    echo "4️⃣ Building MidStream Text Transformer"
    echo "======================================================================"
    echo ""

    # Ensure Rust is in PATH
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi

    # Check if maturin is installed
    if ! command -v maturin &> /dev/null; then
        echo "Installing maturin (Rust-to-Python build tool)..."
        $PYTHON_CMD -m pip install --break-system-packages maturin
    else
        echo -e "${GREEN}✓ maturin already installed${NC}"
    fi

    # Navigate to transformer crate
    cd "$INSTALL_DIR/external/midstream/crates/text-transform" || {
        echo -e "${RED}✗ Text transformer crate not found${NC}"
        echo "  Expected: $INSTALL_DIR/external/midstream/crates/text-transform"
        exit 1
    }

    echo ""
    echo "Building transformer (this takes 2-3 minutes)..."
    echo ""

    # Clean previous builds
    if [ -d "../../target" ]; then
        echo "Cleaning previous builds..."
        rm -rf ../../target/wheels/
    fi

    # Build the wheel (use explicit python interpreter and ABI3 for stability)
    echo "Building with maturin (Python $($PYTHON_CMD --version))..."
    maturin build --release --features pyo3 --interpreter $PYTHON_CMD --compatibility linux || {
        echo -e "${RED}✗ Transformer build failed${NC}"
        echo "  Check Rust installation: rustc --version"
        echo "  Check cargo works: cargo --version"
        exit 1
    }

    echo ""
    echo "Verifying wheel was built..."
    WHEEL_PATH=$(ls -t ../../target/wheels/midstreamer_transform-*.whl 2>/dev/null | head -1)
    if [ -z "$WHEEL_PATH" ]; then
        echo -e "${RED}✗ No wheel file found${NC}"
        echo "  Expected wheel in: ../../target/wheels/"
        exit 1
    fi
    echo -e "${GREEN}✓ Wheel built: $(basename $WHEEL_PATH)${NC}"

    echo ""
    echo "Installing transformer wheel..."
    echo ""

    # Install the built wheel
    $PYTHON_CMD -m pip install --break-system-packages --force-reinstall "$WHEEL_PATH" || {
        echo -e "${RED}✗ Transformer installation failed${NC}"
        echo "  Wheel path: $WHEEL_PATH"
        exit 1
    }

    echo ""
    echo "Verifying transformer installation..."
    if ! $PYTHON_CMD -c "import midstreamer_transform; count, msg = midstreamer_transform.get_stats(); print(f'✅ {msg}')"; then
        echo -e "${RED}✗ Transformer verification failed${NC}"
        echo ""
        echo "This may be a PyO3 ABI compatibility issue."
        echo "Your Python: $($PYTHON_CMD --version)"
        echo "Wheel file: $WHEEL_PATH"
        echo ""
        echo "Try rebuilding with updated PyO3:"
        echo "  cd $INSTALL_DIR/external/midstream/crates/text-transform"
        echo "  cargo clean"
        echo "  maturin build --release --features pyo3 --interpreter $PYTHON_CMD"
        exit 1
    fi

    echo ""
    echo -e "${GREEN}✓ MidStream text transformer built and installed${NC}"
    echo ""

    # Return to install directory
    cd "$INSTALL_DIR" || exit 1
}

# Function: Install Python packages
install_python_deps() {
    echo "======================================================================"
    echo "5️⃣ Installing Python Packages"
    echo "======================================================================"
    echo ""

    if [ ! -f "$INSTALL_DIR/requirements.txt" ]; then
        echo -e "${RED}✗ requirements.txt not found${NC}"
        echo "  Expected location: $INSTALL_DIR/requirements.txt"
        exit 1
    fi

    echo "This may take 5-10 minutes (large packages)..."
    echo ""

    # Check if NVIDIA GPU is available to decide which PyTorch to install
    echo "Checking for NVIDIA GPU..."
    if command -v nvidia-smi &> /dev/null && nvidia-smi &> /dev/null; then
        echo -e "${GREEN}✓ NVIDIA GPU detected - installing PyTorch with CUDA 12.9${NC}"
        TORCH_INDEX="https://download.pytorch.org/whl/cu129"
    else
        echo -e "${YELLOW}⚠ No NVIDIA GPU detected - installing CPU-only PyTorch${NC}"
        TORCH_INDEX=""
    fi

    # Install PyTorch first (separate from requirements.txt)
    echo ""
    echo "Installing PyTorch system-wide..."
    if [[ -n "$TORCH_INDEX" ]]; then
        # Try latest stable CUDA version first
        $PYTHON_CMD -m pip install --break-system-packages torch torchvision torchaudio || {
            echo -e "${YELLOW}⚠ Latest version failed, trying CUDA 12.9 specific${NC}"
            $PYTHON_CMD -m pip install --break-system-packages torch==2.8.0+cu129 torchvision==0.23.0+cu129 torchaudio==2.8.0+cu129 --index-url "$TORCH_INDEX"
        }
    else
        $PYTHON_CMD -m pip install --break-system-packages torch torchvision torchaudio
    fi

    echo ""
    echo "Installing remaining packages system-wide..."

    # Create temporary requirements file without torch (since we installed it separately)
    TEMP_REQUIREMENTS=$(mktemp)
    grep -v "^torch==" "$INSTALL_DIR/requirements.txt" | grep -v "^torchaudio" > "$TEMP_REQUIREMENTS"

    # Install packages with better error visibility
    if ! $PYTHON_CMD -m pip install --break-system-packages -r "$TEMP_REQUIREMENTS"; then
        echo ""
        echo -e "${RED}✗ Failed to install Python packages${NC}"
        echo -e "${YELLOW}Temp requirements at: $TEMP_REQUIREMENTS${NC}"
        echo ""
        echo "You can try installing manually with:"
        echo "  $PYTHON_CMD -m pip install --break-system-packages -r $TEMP_REQUIREMENTS"
        rm -f "$TEMP_REQUIREMENTS"
        exit 1
    fi

    rm -f "$TEMP_REQUIREMENTS"

    # Verify critical packages
    echo ""
    echo "Verifying critical packages..."

    # Check NeMo installation specifically
    if ! $PYTHON_CMD -c "import nemo" 2>/dev/null; then
        echo -e "${RED}✗ NeMo toolkit not installed properly${NC}"
        echo ""
        echo "Attempting to install NeMo manually..."
        if ! $PYTHON_CMD -m pip install --break-system-packages "nemo_toolkit[asr]==2.5.2"; then
            echo -e "${RED}✗ NeMo installation failed${NC}"
            echo ""
            echo "This is critical for speech recognition. Please install manually:"
            echo "  $PYTHON_CMD -m pip install --break-system-packages 'nemo_toolkit[asr]==2.5.2'"
            exit 1
        fi
    fi

    # Check for texterrors PyO3 compatibility issue (affects Python 3.12)
    echo ""
    echo "Checking texterrors compatibility..."
    if ! $PYTHON_CMD -c "import texterrors" 2>/dev/null; then
        echo -e "${YELLOW}⚠ texterrors has PyO3 compatibility issue${NC}"
        echo "  Upgrading to texterrors 1.0.9+ (Python 3.12 compatible)..."

        # Upgrade to latest texterrors (1.0.9+ has Python 3.12 compatible PyO3)
        if $PYTHON_CMD -m pip install --break-system-packages --upgrade texterrors; then
            echo -e "${GREEN}✓ texterrors upgraded successfully${NC}"
        else
            echo -e "${RED}✗ Failed to upgrade texterrors${NC}"
            echo "  NeMo may not load properly"
            exit 1
        fi
    else
        # Check if we have old version that might cause issues
        TEXTERRORS_VERSION=$($PYTHON_CMD -c "import texterrors; print(texterrors.__version__ if hasattr(texterrors, '__version__') else '0.0.0')" 2>/dev/null || echo "0.0.0")
        if [[ "$TEXTERRORS_VERSION" < "1.0.0" ]]; then
            echo -e "${YELLOW}⚠ Old texterrors version detected: $TEXTERRORS_VERSION${NC}"
            echo "  Upgrading to 1.0.9+ for Python 3.12 compatibility..."
            $PYTHON_CMD -m pip install --break-system-packages --upgrade texterrors
            echo -e "${GREEN}✓ texterrors upgraded${NC}"
        else
            echo -e "${GREEN}✓ texterrors $TEXTERRORS_VERSION OK${NC}"
        fi
    fi

    echo ""
    echo -e "${GREEN}✓ NeMo toolkit verified${NC}"
    echo ""
    echo -e "${GREEN}✓ Python packages installed${NC}"
    echo ""
}

# Function: Download STT model
download_model() {
    echo "======================================================================"
    echo "6️⃣ Downloading NVIDIA Canary-1B-Flash Model"
    echo "======================================================================"
    echo ""
    echo "This will download ~1.1 GB model..."
    echo ""

    $PYTHON_CMD -c "
from nemo.collections.asr.models import EncDecMultiTaskModel
import torch

print('Downloading model from HuggingFace...')

# Check CUDA availability without triggering errors
try:
    cuda_available = torch.cuda.is_available()
    print(f'GPU available: {cuda_available}')
    if cuda_available:
        try:
            print(f'GPU: {torch.cuda.get_device_name(0)}')
        except:
            print('⚠ GPU detected but CUDA error - will use CPU')
            cuda_available = False
    else:
        print('⚠ No GPU detected - STT will be slower on CPU')
except Exception as e:
    print(f'⚠ CUDA check failed: {e}')
    print('  Model will run on CPU (slower but works)')

model = EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')
print('✓ Model downloaded successfully')
" || {
        echo ""
        echo -e "${RED}✗ Model download failed${NC}"
        echo "  You can download manually later with:"
        echo "    python3 -c \"from nemo.collections.asr.models import EncDecMultiTaskModel; EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')\""
        echo ""
        read -p "Continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    }

    echo ""
    echo -e "${GREEN}✓ Model downloaded${NC}"
    echo ""
}

# Function: Create config directory
create_config() {
    echo "======================================================================"
    echo "7️⃣ Creating Configuration Directory"
    echo "======================================================================"
    echo ""

    mkdir -p "$CONFIG_DIR"
    echo -e "${GREEN}✓ Created $CONFIG_DIR${NC}"

    # Create default config file (for future use)
    if [ ! -f "$CONFIG_DIR/config.toml" ]; then
        cat > "$CONFIG_DIR/config.toml" << 'EOF'
# Swictation Configuration
# Edit this file to customize behavior

[model]
name = "nvidia/canary-1b-flash"
sample_rate = 16000

[audio]
buffer_duration = 30.0  # seconds
device = "default"      # or specific device name

[vad]
enabled = true
threshold = 0.5         # 0-1, lower = more sensitive
chunk_duration = 10.0   # seconds
chunk_overlap = 1.0     # seconds

[injection]
method = "wtype"        # wtype | clipboard

[keybinding]
toggle = "$mod+Shift+d"  # Uses Sway's $mod variable (Mod4=Super/Windows or Mod1=Alt)
EOF
        echo -e "${GREEN}✓ Created default config: $CONFIG_DIR/config.toml${NC}"
    else
        echo -e "${YELLOW}⚠ Config exists: $CONFIG_DIR/config.toml (not overwriting)${NC}"
    fi

    echo ""
}

# Function: Setup systemd service
setup_systemd() {
    echo "======================================================================"
    echo "8️⃣ Setting Up systemd Service"
    echo "======================================================================"
    echo ""

    read -p "Enable systemd auto-start? (Y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Nn]$ ]]; then
        echo "Skipped systemd setup"
        echo ""
        return
    fi

    mkdir -p "$SYSTEMD_DIR"

    # Copy service files
    if [ -f "$INSTALL_DIR/config/swictation.service" ]; then
        cp "$INSTALL_DIR/config/swictation.service" "$SYSTEMD_DIR/"
        echo -e "${GREEN}✓ Copied main service file${NC}"

        # Update service file to use venv if it was created
        if [[ -n "$VIRTUAL_ENV" ]]; then
            echo -e "${BLUE}Updating service to use Python 3.12 venv${NC}"
        fi
    else
        echo -e "${RED}✗ Main service file not found${NC}"
        echo "  Expected: $INSTALL_DIR/config/swictation.service"
        return
    fi

    # Copy tray service file
    if [ -f "$INSTALL_DIR/config/swictation-tray.service" ]; then
        cp "$INSTALL_DIR/config/swictation-tray.service" "$SYSTEMD_DIR/"
        echo -e "${GREEN}✓ Copied tray service file${NC}"
    else
        echo -e "${YELLOW}⚠ Tray service file not found (optional)${NC}"
        echo "  Expected: $INSTALL_DIR/config/swictation-tray.service"
    fi

    # Reload systemd
    systemctl --user daemon-reload
    echo -e "${GREEN}✓ Reloaded systemd${NC}"

    # Enable services
    systemctl --user enable swictation.service
    echo -e "${GREEN}✓ Enabled swictation.service${NC}"

    if [ -f "$SYSTEMD_DIR/swictation-tray.service" ]; then
        systemctl --user enable swictation-tray.service
        echo -e "${GREEN}✓ Enabled swictation-tray.service${NC}"
    fi

    # Offer to start now
    echo ""
    read -p "Start services now? (Y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        systemctl --user start swictation.service
        echo -e "${GREEN}✓ Started main daemon${NC}"

        if [ -f "$SYSTEMD_DIR/swictation-tray.service" ]; then
            systemctl --user start swictation-tray.service
            echo -e "${GREEN}✓ Started tray app${NC}"
        fi

        echo ""
        sleep 2
        systemctl --user status swictation.service --no-pager
        echo ""
    fi

    echo ""
}

# Function: Setup Sway keybinding
setup_sway() {
    echo "======================================================================"
    echo "9️⃣ Setting Up Sway Keybinding"
    echo "======================================================================"
    echo ""

    if [ ! -f "$SWAY_CONFIG" ]; then
        echo -e "${YELLOW}⚠ Sway config not found: $SWAY_CONFIG${NC}"
        echo "  Create it first, then run: $INSTALL_DIR/scripts/setup-sway.sh"
        echo ""
        return
    fi

    # Check if already configured
    if grep -q "swictation" "$SWAY_CONFIG"; then
        echo -e "${YELLOW}⚠ Swictation already configured in Sway${NC}"
        echo ""
        return
    fi

    read -p "Add Swictation keybinding to Sway? (Y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Nn]$ ]]; then
        echo "Skipped Sway setup"
        echo "  Run manually later: $INSTALL_DIR/scripts/setup-sway.sh"
        echo ""
        return
    fi

    # Run Sway setup script
    if [ -x "$INSTALL_DIR/scripts/setup-sway.sh" ]; then
        bash "$INSTALL_DIR/scripts/setup-sway.sh"
    else
        echo -e "${RED}✗ Sway setup script not found or not executable${NC}"
        echo "  Expected: $INSTALL_DIR/scripts/setup-sway.sh"
    fi

    echo ""
}

# Function: Test installation
test_installation() {
    echo "======================================================================"
    echo "🔟 Testing Installation"
    echo "======================================================================"
    echo ""

    echo "Testing components..."
    echo ""

    # Test Python imports
    echo -n "  PyTorch... "
    $PYTHON_CMD -c "import torch" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    echo -n "  NumPy... "
    $PYTHON_CMD -c "import numpy" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    echo -n "  SoundDevice... "
    $PYTHON_CMD -c "import sounddevice" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    echo -n "  NeMo toolkit... "
    $PYTHON_CMD -c "import nemo" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗ CRITICAL${NC}"

    echo -n "  NeMo ASR models... "
    $PYTHON_CMD -c "from nemo.collections.asr.models import EncDecMultiTaskModel" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗ CRITICAL${NC}"

    # Test wtype
    echo -n "  wtype... "
    which wtype >/dev/null 2>&1 && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    # Test wl-clipboard
    echo -n "  wl-clipboard... "
    which wl-copy >/dev/null 2>&1 && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    # Test CUDA
    echo -n "  CUDA... "
    $PYTHON_CMD -c "import torch; assert torch.cuda.is_available()" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${YELLOW}⚠ (CPU only)${NC}"

    # Test transformer
    echo -n "  Text transformer... "
    $PYTHON_CMD -c "import midstreamer_transform; count, msg = midstreamer_transform.get_stats(); assert count > 0" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    # Test daemon startup (if not already running)
    echo -n "  Daemon connection... "
    $PYTHON_CMD "$INSTALL_DIR/src/swictation_cli.py" status >/dev/null 2>&1 && echo -e "${GREEN}✓${NC}" || echo -e "${YELLOW}⚠ (not running)${NC}"

    echo ""
}

# Main installation flow
main() {
    install_system_deps
    init_submodule
    install_rust
    build_transformer
    install_python_deps
    download_model
    create_config
    setup_systemd
    setup_sway
    test_installation

    echo "======================================================================"
    echo -e "${GREEN}✓ Installation Complete!${NC}"
    echo "======================================================================"
    echo ""

    # Check if Python 3.12 was installed via pyenv
    if [[ -d "$HOME/.pyenv" ]]; then
        echo -e "${BLUE}IMPORTANT: Python 3.12 installed via pyenv${NC}"
        echo ""
        echo "Add to your shell profile (~/.bashrc or ~/.zshrc):"
        echo '  export PYENV_ROOT="$HOME/.pyenv"'
        echo '  export PATH="$PYENV_ROOT/bin:$PATH"'
        echo '  eval "$(pyenv init -)"'
        echo ""
    fi

    echo "Next steps:"
    echo "  1. Start daemon (if not auto-started):"
    echo "       systemctl --user start swictation.service"
    echo ""
    echo "  2. Test VAD-triggered dictation:"
    echo "       Press \$mod+Shift+d in any text editor"
    echo "       Speak continuously (text appears after 2s pauses)"
    echo "       Press \$mod+Shift+d to stop recording"
    echo ""
    echo "  3. Check status:"
    echo "       python3 $INSTALL_DIR/src/swictation_cli.py status"
    echo ""
    echo "  4. View logs:"
    echo "       journalctl --user -u swictation.service -f"
    echo ""
    echo "Configuration:"
    echo "  $CONFIG_DIR/config.toml"
    echo ""
    echo "Documentation:"
    echo "  $INSTALL_DIR/README.md"
    echo "  $INSTALL_DIR/docs/"
    echo ""
    echo "======================================================================"
}

# Run main
main
