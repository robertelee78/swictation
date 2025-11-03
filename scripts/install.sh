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
    echo "This script will automatically set up a Python 3.12 virtual environment."
    echo ""

    # Check if Python 3.12 is available
    if ! command -v python3.12 &> /dev/null; then
        echo -e "${BLUE}Installing Python 3.12...${NC}"

        # For Ubuntu/Debian, check if we can use apt or need pyenv
        if [[ "$DISTRO" == "ubuntu" ]] || [[ "$DISTRO" == "debian" ]] || [[ "$DISTRO" == "pop" ]]; then
            # Try deadsnakes PPA first (works for Ubuntu 24.04 and earlier)
            echo "Trying deadsnakes PPA..."
            sudo apt update
            sudo apt install -y software-properties-common

            # Add PPA and check if it works
            if sudo add-apt-repository -y ppa:deadsnakes/ppa 2>/dev/null && sudo apt update 2>/dev/null; then
                if sudo apt install -y python3.12 python3.12-venv python3.12-dev 2>/dev/null; then
                    echo -e "${GREEN}✓ Python 3.12 installed from PPA${NC}"
                else
                    echo -e "${YELLOW}⚠ PPA doesn't have Python 3.12, using pyenv...${NC}"
                    install_python312_pyenv
                fi
            else
                echo -e "${YELLOW}⚠ PPA not available for $DISTRO_VERSION, using pyenv...${NC}"
                install_python312_pyenv
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

    # Create virtual environment
    VENV_DIR="$INSTALL_DIR/venv"
    if [ ! -d "$VENV_DIR" ]; then
        echo ""
        echo -e "${BLUE}Creating Python 3.12 virtual environment...${NC}"
        python3.12 -m venv "$VENV_DIR" || {
            echo -e "${RED}✗ Failed to create virtual environment${NC}"
            exit 1
        }
        echo -e "${GREEN}✓ Virtual environment created: $VENV_DIR${NC}"
    fi

    # Activate the venv for this script
    source "$VENV_DIR/bin/activate"

    # Update Python version info
    PYTHON_VERSION=$(python -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
    echo -e "${GREEN}✓ Using Python $PYTHON_VERSION from virtual environment${NC}"
    echo ""

elif [[ "$PYTHON_MAJOR" -eq 3 ]] && [[ "$PYTHON_MINOR" -lt 12 ]]; then
    echo -e "${YELLOW}⚠ Python $PYTHON_VERSION detected - may have compatibility issues${NC}"
    echo "  Recommended: Python 3.12"
    echo ""
fi

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
            sudo apt update
            sudo apt install -y \
                python3 python3-pip python3-venv \
                python3.12 python3.12-venv python3.12-dev \
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

    # Check if submodule is already initialized
    if [ -f "$INSTALL_DIR/external/midstream/Cargo.toml" ]; then
        echo -e "${GREEN}✓ Submodule already initialized${NC}"
    else
        echo "Initializing git submodule (external/midstream)..."
        git submodule update --init --recursive || {
            echo -e "${RED}✗ Failed to initialize submodule${NC}"
            echo "  Try manually: cd $INSTALL_DIR && git submodule update --init --recursive"
            exit 1
        }
        echo -e "${GREEN}✓ Git submodule initialized${NC}"
    fi

    # Verify submodule has files
    if [ ! -d "$INSTALL_DIR/external/midstream/crates" ]; then
        echo -e "${RED}✗ Submodule appears empty${NC}"
        echo "  Expected: $INSTALL_DIR/external/midstream/crates/"
        exit 1
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
        python3 -m pip install --user maturin || {
            echo ""
            echo -e "${YELLOW}⚠ Failed with --user, trying --break-system-packages${NC}"
            python3 -m pip install --break-system-packages maturin
        }
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

    # Build the wheel
    maturin build --release --features pyo3 || {
        echo -e "${RED}✗ Transformer build failed${NC}"
        echo "  Check Rust installation: rustc --version"
        echo "  Check cargo works: cargo --version"
        exit 1
    }

    echo ""
    echo "Installing transformer wheel..."
    echo ""

    # Install the built wheel
    python3 -m pip install --break-system-packages --force-reinstall ../../target/wheels/midstreamer_transform-*.whl || {
        echo -e "${RED}✗ Transformer installation failed${NC}"
        echo "  Check wheel exists: ls ../../target/wheels/"
        exit 1
    }

    echo ""
    echo "Verifying transformer installation..."
    python3 -c "import midstreamer_transform; count, msg = midstreamer_transform.get_stats(); print(f'✅ {msg}')" || {
        echo -e "${RED}✗ Transformer verification failed${NC}"
        echo "  Import test failed - transformer may not be properly installed"
        exit 1
    }

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

    # Check if we're in a virtual environment
    if [[ -n "$VIRTUAL_ENV" ]]; then
        echo -e "${GREEN}✓ Using virtual environment: $VIRTUAL_ENV${NC}"
        pip install -r "$INSTALL_DIR/requirements.txt" || {
            echo -e "${RED}✗ Failed to install Python packages${NC}"
            exit 1
        }
    else
        echo -e "${YELLOW}⚠ Not in a virtual environment${NC}"
        echo "  Installing with --break-system-packages (not recommended)"
        echo ""

        # Install with user site-packages
        python3 -m pip install --user -r "$INSTALL_DIR/requirements.txt" || {
            echo ""
            echo -e "${YELLOW}⚠ Some packages failed, trying with --break-system-packages${NC}"
            python3 -m pip install --break-system-packages -r "$INSTALL_DIR/requirements.txt" || {
                echo ""
                echo -e "${RED}✗ Installation failed${NC}"
                echo ""
                echo "Recommended: Create a Python 3.12 virtual environment:"
                echo "  python3.12 -m venv $INSTALL_DIR/venv"
                echo "  source $INSTALL_DIR/venv/bin/activate"
                echo "  python3 -m pip install -r $INSTALL_DIR/requirements.txt"
                exit 1
            }
        }
    fi

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

    python3 -c "
from nemo.collections.asr.models import EncDecMultiTaskModel
import torch

print('Downloading model from HuggingFace...')
print(f'GPU available: {torch.cuda.is_available()}')

if torch.cuda.is_available():
    print(f'GPU: {torch.cuda.get_device_name(0)}')
else:
    print('⚠ No GPU detected - STT will be slower on CPU')

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

    # Copy service file
    if [ -f "$INSTALL_DIR/config/swictation.service" ]; then
        cp "$INSTALL_DIR/config/swictation.service" "$SYSTEMD_DIR/"
        echo -e "${GREEN}✓ Copied service file${NC}"

        # Update service file to use venv if it was created
        if [[ -n "$VIRTUAL_ENV" ]]; then
            echo -e "${BLUE}Updating service to use Python 3.12 venv${NC}"
        fi

        # Reload systemd
        systemctl --user daemon-reload
        echo -e "${GREEN}✓ Reloaded systemd${NC}"

        # Enable service
        systemctl --user enable swictation.service
        echo -e "${GREEN}✓ Enabled swictation.service${NC}"

        # Offer to start now
        echo ""
        read -p "Start daemon now? (Y/n): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            systemctl --user start swictation.service
            sleep 2
            systemctl --user status swictation.service --no-pager
            echo ""
            echo -e "${GREEN}✓ Daemon started${NC}"
        fi
    else
        echo -e "${YELLOW}⚠ Service file not found: $INSTALL_DIR/config/swictation.service${NC}"
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
    echo -n "  Python imports... "
    python3 -c "
import torch
import numpy
import sounddevice
from nemo.collections.asr.models import EncDecMultiTaskModel
" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    # Test wtype
    echo -n "  wtype... "
    which wtype >/dev/null 2>&1 && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    # Test wl-clipboard
    echo -n "  wl-clipboard... "
    which wl-copy >/dev/null 2>&1 && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    # Test CUDA
    echo -n "  CUDA... "
    python3 -c "import torch; assert torch.cuda.is_available()" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${YELLOW}⚠ (CPU only)${NC}"

    # Test transformer
    echo -n "  Text transformer... "
    python3 -c "import midstreamer_transform; count, msg = midstreamer_transform.get_stats(); assert count > 0" 2>/dev/null && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

    # Test daemon startup (if not already running)
    echo -n "  Daemon connection... "
    python3 "$INSTALL_DIR/src/swictation_cli.py" status >/dev/null 2>&1 && echo -e "${GREEN}✓${NC}" || echo -e "${YELLOW}⚠ (not running)${NC}"

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

    # Check if venv was created
    if [[ -n "$VIRTUAL_ENV" ]]; then
        echo -e "${BLUE}IMPORTANT: Python 3.12 virtual environment created${NC}"
        echo ""
        echo "To activate the virtual environment in new shells:"
        echo "  source $VIRTUAL_ENV/bin/activate"
        echo ""
        echo "To add to your shell profile (~/.bashrc or ~/.zshrc):"
        echo "  echo 'source $VIRTUAL_ENV/bin/activate' >> ~/.bashrc"
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
