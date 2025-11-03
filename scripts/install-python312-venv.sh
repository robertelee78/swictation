#!/bin/bash
# Helper script for Python 3.13+ systems
# Creates a Python 3.12 virtual environment to avoid numpy dependency conflicts

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

INSTALL_DIR="/opt/swictation"
VENV_DIR="$INSTALL_DIR/venv"

echo "======================================================================"
echo -e "${BLUE}Python 3.12 Virtual Environment Setup${NC}"
echo "======================================================================"
echo ""
echo "This script creates a Python 3.12 virtual environment for Swictation"
echo "to avoid numpy dependency conflicts with Python 3.13+"
echo ""

# Check current Python version
PYTHON_VERSION=$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
echo -e "${BLUE}System Python: $PYTHON_VERSION${NC}"
echo ""

# Check if Python 3.12 is available
if ! command -v python3.12 &> /dev/null; then
    echo -e "${RED}✗ Python 3.12 not found${NC}"
    echo ""
    echo "Installing Python 3.12..."

    # Detect distro
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        case $ID in
            ubuntu|debian|pop)
                sudo apt update
                sudo apt install -y python3.12 python3.12-venv python3.12-dev
                ;;
            arch|manjaro)
                sudo pacman -S python312
                ;;
            fedora)
                sudo dnf install -y python3.12
                ;;
            *)
                echo -e "${RED}✗ Unsupported distribution${NC}"
                echo "Please install Python 3.12 manually"
                exit 1
                ;;
        esac
    fi
fi

# Verify Python 3.12
if ! command -v python3.12 &> /dev/null; then
    echo -e "${RED}✗ Python 3.12 installation failed${NC}"
    exit 1
fi

PYTHON312_VERSION=$(python3.12 --version)
echo -e "${GREEN}✓ Found: $PYTHON312_VERSION${NC}"
echo ""

# Create virtual environment
if [ -d "$VENV_DIR" ]; then
    echo -e "${YELLOW}⚠ Virtual environment already exists: $VENV_DIR${NC}"
    read -p "Remove and recreate? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$VENV_DIR"
    else
        echo "Keeping existing venv"
        exit 0
    fi
fi

echo "Creating Python 3.12 virtual environment..."
python3.12 -m venv "$VENV_DIR" || {
    echo -e "${RED}✗ Failed to create virtual environment${NC}"
    exit 1
}

echo -e "${GREEN}✓ Virtual environment created: $VENV_DIR${NC}"
echo ""

# Activate and verify
source "$VENV_DIR/bin/activate"
VENV_PYTHON_VERSION=$(python --version)
echo -e "${GREEN}✓ Activated: $VENV_PYTHON_VERSION${NC}"
echo ""

# Upgrade pip
echo "Upgrading pip..."
pip install --upgrade pip setuptools wheel
echo ""

# Install requirements
if [ -f "$INSTALL_DIR/requirements.txt" ]; then
    echo "Installing Python packages (this takes 5-10 minutes)..."
    echo ""
    pip install -r "$INSTALL_DIR/requirements.txt" || {
        echo ""
        echo -e "${RED}✗ Installation failed${NC}"
        exit 1
    }
    echo ""
    echo -e "${GREEN}✓ Python packages installed${NC}"
else
    echo -e "${YELLOW}⚠ requirements.txt not found${NC}"
    echo "  You can install packages later with:"
    echo "    source $VENV_DIR/bin/activate"
    echo "    pip install -r $INSTALL_DIR/requirements.txt"
fi

echo ""
echo "======================================================================"
echo -e "${GREEN}✓ Setup Complete${NC}"
echo "======================================================================"
echo ""
echo "Virtual environment created at: $VENV_DIR"
echo ""
echo "To activate the virtual environment:"
echo "  source $VENV_DIR/bin/activate"
echo ""
echo "To continue installation:"
echo "  source $VENV_DIR/bin/activate"
echo "  cd $INSTALL_DIR"
echo "  bash scripts/install.sh"
echo ""
echo "To add to your shell profile (~/.bashrc or ~/.zshrc):"
echo "  echo 'source $VENV_DIR/bin/activate' >> ~/.bashrc"
echo ""
echo "======================================================================"
