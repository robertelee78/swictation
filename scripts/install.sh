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

echo "======================================================================"
echo -e "${BLUE}Swictation Installation${NC}"
echo "======================================================================"
echo ""
echo "This script will:"
echo "  ✓ Install system dependencies (wtype, wl-clipboard, ffmpeg)"
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

# Detect distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
else
    echo -e "${RED}✗ Cannot detect Linux distribution${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Detected distribution: $DISTRO${NC}"
echo ""

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
                python3 python3-pip \
                wtype wl-clipboard \
                ffmpeg \
                pipewire pipewire-pulse || {
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

# Function: Install Python packages
install_python_deps() {
    echo "======================================================================"
    echo "2️⃣ Installing Python Packages"
    echo "======================================================================"
    echo ""

    if [ ! -f "$INSTALL_DIR/requirements.txt" ]; then
        echo -e "${RED}✗ requirements.txt not found${NC}"
        echo "  Expected location: $INSTALL_DIR/requirements.txt"
        exit 1
    fi

    echo "This may take 5-10 minutes (large packages)..."
    echo ""

    # Install with user site-packages
    python3 -m pip install --user -r "$INSTALL_DIR/requirements.txt" || {
        echo ""
        echo -e "${YELLOW}⚠ Some packages failed, trying with --break-system-packages${NC}"
        python3 -m pip install --break-system-packages -r "$INSTALL_DIR/requirements.txt"
    }

    echo ""
    echo -e "${GREEN}✓ Python packages installed${NC}"
    echo ""
}

# Function: Download STT model
download_model() {
    echo "======================================================================"
    echo "3️⃣ Downloading NVIDIA Canary-1B-Flash Model"
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
    echo "4️⃣ Creating Configuration Directory"
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
toggle = "Mod1+Shift+d"  # Alt+Shift+d
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
    echo "5️⃣ Setting Up systemd Service"
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
    echo "6️⃣ Setting Up Sway Keybinding"
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
    echo "7️⃣ Testing Installation"
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

    # Test daemon startup (if not already running)
    echo -n "  Daemon connection... "
    python3 "$INSTALL_DIR/src/swictation_cli.py" status >/dev/null 2>&1 && echo -e "${GREEN}✓${NC}" || echo -e "${YELLOW}⚠ (not running)${NC}"

    echo ""
}

# Main installation flow
main() {
    install_system_deps
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
    echo "Next steps:"
    echo "  1. Start daemon (if not auto-started):"
    echo "       systemctl --user start swictation.service"
    echo ""
    echo "  2. Test keybinding:"
    echo "       Press Alt+Shift+d in any text editor"
    echo "       Speak your text"
    echo "       Press Alt+Shift+d again to transcribe"
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
