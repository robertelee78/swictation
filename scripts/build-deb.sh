#!/bin/bash
# Swictation DEB Package Builder
# Creates a lightweight DEB package with complete dependency declarations
# Dependencies are installed via apt/pip, not bundled (keeps package small)

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Package metadata
PACKAGE_NAME="swictation"
VERSION="${1:-0.1.0}"
ITERATION="${2:-1}"
ARCH="amd64"
MAINTAINER="Swictation Team <maintainer@example.com>"
DESCRIPTION="Real-time voice-to-text dictation daemon for Sway/Wayland with GPU acceleration"
URL="https://github.com/robertelee78/swictation"
LICENSE="Apache-2.0"

# Directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="/tmp/swictation-deb-build"
STAGING_DIR="${BUILD_DIR}/staging"
OUTPUT_DIR="${PROJECT_ROOT}/dist"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Swictation DEB Package Builder${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Version: ${GREEN}${VERSION}-${ITERATION}${NC}"
echo -e "Architecture: ${GREEN}${ARCH}${NC}"
echo ""

# Check FPM is installed
if ! command -v fpm &> /dev/null; then
    echo -e "${RED}ERROR: FPM not found!${NC}"
    echo -e "Install with: ${YELLOW}sudo gem install fpm${NC}"
    exit 1
fi

# Check Rust wheel is built
WHEEL_PATH="${PROJECT_ROOT}/external/midstream/target/wheels/midstreamer_transform-0.1.0-cp312-abi3-linux_x86_64.whl"
if [ ! -f "$WHEEL_PATH" ]; then
    echo -e "${YELLOW}WARNING: Rust wheel not found at ${WHEEL_PATH}${NC}"
    echo -e "${YELLOW}Building Rust text-transform component...${NC}"
    cd "${PROJECT_ROOT}/external/midstream/crates/text-transform"
    if command -v maturin &> /dev/null; then
        maturin build --release
    else
        echo -e "${RED}ERROR: maturin not found. Install with: pip install maturin${NC}"
        exit 1
    fi
    cd "${PROJECT_ROOT}"
fi

# Clean and create directories
echo -e "${BLUE}Creating build directories...${NC}"
rm -rf "${BUILD_DIR}"
mkdir -p "${STAGING_DIR}"
mkdir -p "${OUTPUT_DIR}"

# Create directory structure
mkdir -p "${STAGING_DIR}/opt/swictation"
mkdir -p "${STAGING_DIR}/usr/local/bin"
mkdir -p "${STAGING_DIR}/usr/lib/systemd/user"
mkdir -p "${STAGING_DIR}/usr/share/doc/swictation"
mkdir -p "${STAGING_DIR}/etc/swictation"

# Copy application files
echo -e "${BLUE}Copying application files...${NC}"
cp -r "${PROJECT_ROOT}/src" "${STAGING_DIR}/opt/swictation/"
cp -r "${PROJECT_ROOT}/scripts" "${STAGING_DIR}/opt/swictation/"
cp -r "${PROJECT_ROOT}/config" "${STAGING_DIR}/opt/swictation/"
cp "${PROJECT_ROOT}/requirements.txt" "${STAGING_DIR}/opt/swictation/"
cp "${PROJECT_ROOT}/README.md" "${STAGING_DIR}/usr/share/doc/swictation/"
cp "${PROJECT_ROOT}/LICENSE" "${STAGING_DIR}/usr/share/doc/swictation/"

# Copy Rust wheel
echo -e "${BLUE}Copying Rust PyO3 wheel...${NC}"
mkdir -p "${STAGING_DIR}/opt/swictation/wheels"
cp "${WHEEL_PATH}" "${STAGING_DIR}/opt/swictation/wheels/"

# Copy example config
cp "${PROJECT_ROOT}/config/config.example.toml" "${STAGING_DIR}/etc/swictation/config.example.toml"

# Create systemd service files for user services
echo -e "${BLUE}Creating systemd service files...${NC}"
cp "${PROJECT_ROOT}/config/swictation.service" "${STAGING_DIR}/usr/lib/systemd/user/"
cp "${PROJECT_ROOT}/config/swictation-tray.service" "${STAGING_DIR}/usr/lib/systemd/user/"

# Create wrapper scripts
echo -e "${BLUE}Creating wrapper scripts...${NC}"
cat > "${STAGING_DIR}/usr/local/bin/swictation" << 'EOF'
#!/bin/bash
# Swictation daemon wrapper
exec python3 /opt/swictation/src/swictationd.py "$@"
EOF

cat > "${STAGING_DIR}/usr/local/bin/swictation-cli" << 'EOF'
#!/bin/bash
# Swictation CLI wrapper
exec python3 /opt/swictation/src/swictation_cli.py "$@"
EOF

chmod +x "${STAGING_DIR}/usr/local/bin/swictation"
chmod +x "${STAGING_DIR}/usr/local/bin/swictation-cli"

# Create pre-install script to check requirements
echo -e "${BLUE}Creating pre-install script...${NC}"
cat > "${BUILD_DIR}/preinst.sh" << 'EOF'
#!/bin/bash
# Swictation pre-installation requirements check

set -e

echo "Checking Python version..."

# Check Python version - require 3.10+
PYTHON_VERSION=$(python3 --version 2>&1 | awk '{print $2}' | cut -d. -f1,2)
PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d. -f1)
PYTHON_MINOR=$(echo $PYTHON_VERSION | cut -d. -f2)

if [ "$PYTHON_MAJOR" -ne 3 ] || [ "$PYTHON_MINOR" -lt 10 ]; then
    echo ""
    echo "ERROR: Python 3.10+ required!"
    echo ""
    echo "Current: Python $PYTHON_VERSION"
    echo "Required: Python 3.10 or newer"
    echo ""
    exit 1
fi

echo "✓ Python $PYTHON_VERSION detected"
if [ "$PYTHON_MINOR" -ge 13 ]; then
    echo "  Note: Python 3.13+ detected. NeMo officially requires numpy<2.0,"
    echo "  but works fine with numpy 2.x. Installation will proceed with"
    echo "  --break-system-packages flag."
fi
echo ""

echo "Checking GPU requirements..."

# Check for NVIDIA driver (check actual binary, not just command -v)
if [ ! -x /usr/bin/nvidia-smi ] && [ ! -x /usr/local/bin/nvidia-smi ]; then
    echo ""
    echo "ERROR: NVIDIA driver not found!"
    echo ""
    echo "Swictation requires an NVIDIA GPU with driver installed."
    echo ""
    echo "Install NVIDIA driver first:"
    echo "  - Download latest from: https://www.nvidia.com/Download/index.aspx"
    echo "  - Or: apt search nvidia-driver (install latest available version)"
    echo ""
    exit 1
fi

# Check for CUDA toolkit (check for nvcc binary or cuda directory)
NVCC_PATH=""
if [ -x /usr/local/cuda/bin/nvcc ]; then
    NVCC_PATH=/usr/local/cuda/bin/nvcc
elif [ -x /usr/bin/nvcc ]; then
    NVCC_PATH=/usr/bin/nvcc
elif command -v nvcc &> /dev/null 2>&1; then
    NVCC_PATH=$(command -v nvcc)
fi

if [ -z "$NVCC_PATH" ] && [ ! -d "/usr/local/cuda" ]; then
    echo ""
    echo "ERROR: CUDA toolkit not found!"
    echo ""
    echo "Swictation requires CUDA 12.4+ for GPU inference."
    echo ""
    echo "Install latest CUDA from NVIDIA:"
    echo "  https://developer.nvidia.com/cuda-downloads"
    echo ""
    echo "Get CUDA 12.6 or newer (NOT ancient apt packages)"
    echo ""
    exit 1
fi

# If we got here, requirements are met
CUDA_VERSION=$($NVCC_PATH --version 2>/dev/null | grep "release" | awk '{print $5}' | tr -d ',' || echo "unknown")
GPU_NAME=$(/usr/bin/nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1 || /usr/local/bin/nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1 || echo "unknown")

echo "✓ NVIDIA GPU detected: $GPU_NAME"
echo "✓ CUDA toolkit found: Version $CUDA_VERSION"
echo ""
EOF

chmod +x "${BUILD_DIR}/preinst.sh"

# Create post-install script
echo -e "${BLUE}Creating post-install script...${NC}"
cat > "${BUILD_DIR}/postinst.sh" << 'EOF'
#!/bin/bash
# Swictation post-installation script

set -e

# Detect actual user (not root when installed via sudo)
ACTUAL_USER="${SUDO_USER:-$USER}"
ACTUAL_HOME=$(getent passwd "$ACTUAL_USER" | cut -d: -f6)

echo "Installing Swictation dependencies..."
echo "Installing for user: $ACTUAL_USER"

# Install Rust wheel for text transformation
if [ -f /opt/swictation/wheels/midstreamer_transform-0.1.0-cp312-abi3-linux_x86_64.whl ]; then
    echo "Checking midstreamer_transform..."
    if ! pip3 show midstreamer-transform &>/dev/null; then
        echo "Installing midstreamer_transform..."
        pip3 install --break-system-packages /opt/swictation/wheels/midstreamer_transform-*.whl 2>/dev/null || \
        pip3 install /opt/swictation/wheels/midstreamer_transform-*.whl
    else
        echo "✓ midstreamer_transform already installed"
    fi
fi

# Check if main dependencies are already installed
echo "Checking Python dependencies..."
NEEDS_INSTALL=false
if ! python3 -c "import nemo" &>/dev/null; then
    echo "  NeMo not found - will install dependencies"
    NEEDS_INSTALL=true
elif ! python3 -c "import torch" &>/dev/null; then
    echo "  PyTorch not found - will install dependencies"
    NEEDS_INSTALL=true
elif ! python3 -c "import transformers" &>/dev/null; then
    echo "  Transformers not found - will install dependencies"
    NEEDS_INSTALL=true
else
    echo "✓ Core dependencies already installed"
fi

# Only install if needed
if [ "$NEEDS_INSTALL" = true ] && [ -f /opt/swictation/requirements.txt ]; then
    echo ""
    echo "Installing Python packages from requirements.txt..."
    echo "This may take several minutes (PyTorch, NeMo, etc.)..."
    pip3 install --break-system-packages -r /opt/swictation/requirements.txt 2>/dev/null || \
    pip3 install -r /opt/swictation/requirements.txt
else
    echo "Skipping dependency installation (already installed)"
fi

# Create user config directory (as actual user, not root)
sudo -u "$ACTUAL_USER" mkdir -p "$ACTUAL_HOME/.config/swictation"

# Handle config file (don't overwrite existing)
if [ ! -f "$ACTUAL_HOME/.config/swictation/config.toml" ]; then
    sudo -u "$ACTUAL_USER" cp /etc/swictation/config.example.toml "$ACTUAL_HOME/.config/swictation/config.toml"
    echo "✓ Created config: $ACTUAL_HOME/.config/swictation/config.toml"
else
    echo "✓ Config already exists: $ACTUAL_HOME/.config/swictation/config.toml (not modified)"
fi

# Setup Sway keybinding (gracefully handles existing binding)
if [ -f "$ACTUAL_HOME/.config/sway/config" ]; then
    BINDING_EXISTS=$(grep -c "swictation" "$ACTUAL_HOME/.config/sway/config" 2>/dev/null || echo "0")
    if [ "$BINDING_EXISTS" -eq 0 ]; then
        echo ""
        echo "Adding Sway keybinding..."
        sudo -u "$ACTUAL_USER" /opt/swictation/scripts/setup-sway.sh 2>/dev/null || echo "Note: Run setup-sway.sh manually if needed"
        echo "✓ Sway keybinding added (reload Sway to activate)"
    else
        echo "✓ Sway keybinding already configured"
    fi
fi

# Handle systemd services (enable + start/restart as actual user)
if command -v systemctl &> /dev/null; then
    echo ""
    echo "Configuring systemd services..."

    # Get user's XDG_RUNTIME_DIR and DBUS_SESSION_BUS_ADDRESS
    USER_UID=$(id -u "$ACTUAL_USER")
    export XDG_RUNTIME_DIR="/run/user/$USER_UID"
    export DBUS_SESSION_BUS_ADDRESS="unix:path=${XDG_RUNTIME_DIR}/bus"

    # Reload daemon as user
    sudo -u "$ACTUAL_USER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" systemctl --user daemon-reload 2>/dev/null || true

    # Check if services were already running (upgrade case)
    DAEMON_RUNNING=false
    TRAY_RUNNING=false
    if sudo -u "$ACTUAL_USER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" systemctl --user is-active --quiet swictation.service 2>/dev/null; then
        DAEMON_RUNNING=true
    fi
    if sudo -u "$ACTUAL_USER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" systemctl --user is-active --quiet swictation-tray.service 2>/dev/null; then
        TRAY_RUNNING=true
    fi

    # Enable services
    sudo -u "$ACTUAL_USER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" systemctl --user enable swictation.service 2>/dev/null || true
    sudo -u "$ACTUAL_USER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" systemctl --user enable swictation-tray.service 2>/dev/null || true

    # Start or restart services
    if [ "$DAEMON_RUNNING" = true ]; then
        echo "Restarting swictation.service (upgrade)..."
        sudo -u "$ACTUAL_USER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" systemctl --user restart swictation.service 2>/dev/null || true
    else
        echo "Starting swictation.service..."
        sudo -u "$ACTUAL_USER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" systemctl --user start swictation.service 2>/dev/null || true
    fi

    if [ "$TRAY_RUNNING" = true ]; then
        echo "Restarting swictation-tray.service (upgrade)..."
        sudo -u "$ACTUAL_USER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" systemctl --user restart swictation-tray.service 2>/dev/null || true
    else
        echo "Starting swictation-tray.service..."
        sudo -u "$ACTUAL_USER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" systemctl --user start swictation-tray.service 2>/dev/null || true
    fi

    echo "✓ Services enabled and started"
fi

echo ""
echo "======================================"
echo "Validating installation..."
echo "======================================"
echo ""

# Determine Python version (prefer 3.12, fallback to python3)
PYTHON_CMD="python3"
if command -v python3.12 &> /dev/null; then
    PYTHON_CMD="python3.12"
fi

echo "Python version:"
$PYTHON_CMD --version
echo ""

# Check PyTorch
echo "PyTorch + CUDA:"
$PYTHON_CMD -c "import torch; print(f'  PyTorch: {torch.__version__}'); print(f'  CUDA: {torch.version.cuda}'); print(f'  CUDA available: {torch.cuda.is_available()}'); print(f'  Architectures: {torch.cuda.get_arch_list()}' if torch.cuda.is_available() else '  Architectures: N/A (CUDA not available)')" 2>&1 || echo "  ERROR: PyTorch import failed"
echo ""

# Check NeMo
echo "NVIDIA NeMo:"
$PYTHON_CMD -c "import nemo; print(f'  NeMo: {nemo.__version__}')" 2>&1 || echo "  ERROR: NeMo import failed"
echo ""

# Check texterrors
echo "texterrors:"
$PYTHON_CMD -c "from importlib.metadata import version; print(f'  texterrors: {version(\"texterrors\")}')" 2>&1 || echo "  ERROR: texterrors not installed"
echo ""

# Check GPU details
echo "GPU information:"
if command -v nvidia-smi &> /dev/null; then
    # Get CUDA version from nvidia-smi header
    CUDA_VER=$(nvidia-smi 2>/dev/null | grep "CUDA Version" | awk '{print $9}' || echo "unknown")

    nvidia-smi --query-gpu=name,compute_cap,driver_version --format=csv,noheader 2>&1 | while IFS=, read -r name compute_cap driver; do
        echo "  GPU: $name"
        echo "  Compute capability: $compute_cap"
        echo "  Driver version: $driver"
        echo "  CUDA version (driver): $CUDA_VER"
    done
else
    echo "  ERROR: nvidia-smi not found"
fi
echo ""

# Simple GPU compute test
echo "GPU compute test:"
$PYTHON_CMD -c "
import torch
if torch.cuda.is_available():
    try:
        x = torch.tensor([1.0, 2.0, 3.0]).cuda()
        result = (x + x).cpu()
        print(f'  ✓ GPU compute successful (3 + 3 = {result[0]:.1f}, {result[1]:.1f}, {result[2]:.1f})')
    except Exception as e:
        print(f'  ✗ GPU compute failed: {e}')
else:
    print('  ✗ CUDA not available - check driver installation')
" 2>&1 || echo "  ERROR: Test failed"
echo ""

echo ""
echo "======================================"
echo "Swictation installed successfully!"
echo "======================================"
echo ""
echo "Next steps:"
echo "1. Edit config (if needed): ~/.config/swictation/config.toml"
echo "2. Reload Sway: swaymsg reload"
echo "3. Test: Press \$mod+Shift+d and speak"
echo ""
echo "Services are already running:"
echo "  • swictation.service (daemon)"
echo "  • swictation-tray.service (system tray UI)"
echo ""
echo "Check status:"
echo "  systemctl --user status swictation.service"
echo "  systemctl --user status swictation-tray.service"
echo ""
EOF

chmod +x "${BUILD_DIR}/postinst.sh"

# Create pre-remove script
echo -e "${BLUE}Creating pre-remove script...${NC}"
cat > "${BUILD_DIR}/prerm.sh" << 'EOF'
#!/bin/bash
# Swictation pre-removal script

if command -v systemctl &> /dev/null; then
    echo "Stopping and disabling swictation services..."
    systemctl --user stop swictation.service 2>/dev/null || true
    systemctl --user stop swictation-tray.service 2>/dev/null || true
    systemctl --user disable swictation.service 2>/dev/null || true
    systemctl --user disable swictation-tray.service 2>/dev/null || true
    systemctl --user daemon-reload 2>/dev/null || true
fi
EOF

chmod +x "${BUILD_DIR}/prerm.sh"

# Build the DEB package
echo -e "${BLUE}Building DEB package...${NC}"
echo ""

fpm -s dir -t deb \
    -n "${PACKAGE_NAME}" \
    -v "${VERSION}" \
    --iteration "${ITERATION}" \
    -a "${ARCH}" \
    --category "utils" \
    --license "${LICENSE}" \
    --vendor "Swictation Project" \
    --maintainer "${MAINTAINER}" \
    --description "${DESCRIPTION}" \
    --url "${URL}" \
    \
    `# System Dependencies` \
    -d "python3 >= 3.10" \
    -d "python3-pip" \
    -d "wtype" \
    -d "wl-clipboard" \
    -d "ffmpeg" \
    -d "pipewire | pulseaudio" \
    \
    `# Note: NVIDIA driver and CUDA toolkit checked in post-install script` \
    `# Not declared as DEB dependencies to avoid forcing old versions` \
    \
    `# Systemd service files (installed as regular files, managed in postinst)` \
    `# NOT using --deb-systemd to avoid FPM wrapping postinst with silent output redirects` \
    --deb-no-default-config-files \
    \
    `# Installation hooks` \
    --before-install "${BUILD_DIR}/preinst.sh" \
    --after-install "${BUILD_DIR}/postinst.sh" \
    --before-remove "${BUILD_DIR}/prerm.sh" \
    \
    `# Package source` \
    -C "${STAGING_DIR}" \
    opt/ usr/ etc/

# Move package to output directory
mv *.deb "${OUTPUT_DIR}/"

# Get package filename
PACKAGE_FILE="${OUTPUT_DIR}/${PACKAGE_NAME}_${VERSION}-${ITERATION}_${ARCH}.deb"

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Package built successfully!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "Package: ${BLUE}${PACKAGE_FILE}${NC}"
echo -e "Size: ${BLUE}$(du -h "${PACKAGE_FILE}" | cut -f1)${NC}"
echo ""
echo -e "${YELLOW}Installation instructions:${NC}"
echo ""
echo -e "  ${GREEN}# Install package and dependencies:${NC}"
echo -e "  sudo apt-get install ${PACKAGE_FILE}"
echo ""
echo -e "  ${GREEN}# Or use gdebi (better dependency handling):${NC}"
echo -e "  sudo apt-get install gdebi-core"
echo -e "  sudo gdebi ${PACKAGE_FILE}"
echo ""
echo -e "${YELLOW}Package contents:${NC}"
dpkg-deb -c "${PACKAGE_FILE}" | head -20
echo ""
echo -e "${YELLOW}Package info:${NC}"
dpkg-deb -I "${PACKAGE_FILE}"
echo ""

# Clean up
rm -rf "${BUILD_DIR}"

echo -e "${GREEN}Build complete!${NC}"
