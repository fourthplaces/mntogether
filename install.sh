#!/usr/bin/env bash
#
# devkit installer
# Usage: curl -fsSL https://devkit.sh/install | sh
#
set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

DEVKIT_VERSION="${DEVKIT_VERSION:-latest}"
REPO_URL="https://github.com/crcn/devkit"
RAW_URL="https://raw.githubusercontent.com/crcn/devkit/main"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# =============================================================================
# Helper Functions
# =============================================================================

log_info() {
    echo -e "${CYAN}==>${NC} ${BOLD}$1${NC}"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1" >&2
}

check_command() {
    command -v "$1" >/dev/null 2>&1
}

ensure_command() {
    if ! check_command "$1"; then
        log_error "$1 is required but not installed."
        exit 1
    fi
}

# =============================================================================
# Platform Detection
# =============================================================================

detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)

    case "$os" in
        linux*)
            case "$arch" in
                x86_64|amd64)
                    echo "linux-x86_64"
                    ;;
                aarch64|arm64)
                    echo "linux-aarch64"
                    ;;
                *)
                    log_error "Unsupported architecture: $arch"
                    exit 1
                    ;;
            esac
            ;;
        darwin*)
            case "$arch" in
                x86_64|amd64)
                    echo "macos-x86_64"
                    ;;
                arm64|aarch64)
                    echo "macos-aarch64"
                    ;;
                *)
                    log_error "Unsupported architecture: $arch"
                    exit 1
                    ;;
            esac
            ;;
        mingw*|msys*|cygwin*)
            echo "windows-x86_64.exe"
            ;;
        *)
            log_error "Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

get_download_url() {
    local version="$1"
    local platform="$2"

    if [ "$version" = "latest" ]; then
        echo "https://github.com/crcn/devkit/releases/latest/download/devkit-${platform}"
    else
        echo "https://github.com/crcn/devkit/releases/download/${version}/devkit-${platform}"
    fi
}

# =============================================================================
# Dependency Checks
# =============================================================================

check_dependencies() {
    log_info "Checking dependencies..."

    # Required
    ensure_command curl
    ensure_command git

    # Recommended (git is likely to be available if we got here)
    local missing=()
    if ! check_command docker; then
        missing+=("docker")
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        log_warn "Missing recommended tools: ${missing[*]}"
        log_warn "devkit will have limited functionality without them."
        echo
    else
        log_success "All dependencies satisfied"
    fi
}

# =============================================================================
# Project Detection
# =============================================================================

detect_project_root() {
    # Look for git root or use current directory
    if git rev-parse --show-toplevel >/dev/null 2>&1; then
        git rev-parse --show-toplevel
    else
        pwd
    fi
}

detect_project_type() {
    local root="$1"
    local types=()

    [ -f "$root/Cargo.toml" ] && types+=("rust")
    [ -f "$root/package.json" ] && types+=("node")
    [ -f "$root/docker-compose.yml" ] && types+=("docker")
    [ -d "$root/.git" ] && types+=("git")

    echo "${types[@]}"
}

# =============================================================================
# Mode Selection
# =============================================================================

select_mode() {
    # Check if mode is set via environment variable
    if [ -n "${DEVKIT_MODE:-}" ]; then
        case "$DEVKIT_MODE" in
            kitchen-sink|1) echo "kitchen-sink" ;;
            custom|2) echo "custom" ;;
            *)
                log_warn "Invalid DEVKIT_MODE='$DEVKIT_MODE', defaulting to kitchen-sink"
                echo "kitchen-sink"
                ;;
        esac
        return
    fi

    # Check if running interactively (stdin is a terminal)
    if [ -t 0 ]; then
        echo
        log_info "Select installation mode:"
        echo
        echo "  1) Kitchen Sink (Recommended)"
        echo "     Batteries-included CLI with all features."
        echo "     Configure via .dev/config.toml - no Rust code needed!"
        echo "     Perfect for most projects."
        echo
        echo "  2) Custom CLI"
        echo "     Create your own CLI with dev/cli/ project."
        echo "     Full control - add only extensions you need."
        echo "     Best for complex projects with specific needs."
        echo
        read -p "Enter choice [1]: " mode
        mode=${mode:-1}

        case $mode in
            1) echo "kitchen-sink" ;;
            2) echo "custom" ;;
            *) echo "kitchen-sink" ;;
        esac
    else
        # Non-interactive mode (piped from curl) - default to kitchen-sink
        log_info "Non-interactive mode detected, using Kitchen Sink mode (recommended)"
        echo "kitchen-sink"
    fi
}

# =============================================================================
# Binary Installation
# =============================================================================

install_devkit_binary() {
    log_info "Installing devkit binary..."

    # Detect platform
    local platform=$(detect_platform)
    log_info "Detected platform: $platform"

    # Get download URL
    local url=$(get_download_url "$DEVKIT_VERSION" "$platform")
    log_info "Downloading from: $url"

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Download binary
    local binary_path="$INSTALL_DIR/devkit"
    if curl -fsSL "$url" -o "$binary_path"; then
        chmod +x "$binary_path"
        log_success "Installed devkit binary to $binary_path"
    else
        log_error "Failed to download devkit binary"
        log_error "You can build from source instead:"
        log_error "  git clone $REPO_URL && cd devkit && cargo build --release -p devkit-cli"
        exit 1
    fi

    # Check if install directory is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        log_warn "$INSTALL_DIR is not in your PATH"
        log_warn "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo
    fi
}

# =============================================================================
# Installation
# =============================================================================

install_wrapper_script() {
    local root="$1"
    local mode="$2"
    local script_path="$root/dev.sh"

    log_info "Installing dev.sh wrapper..."

    if [ -f "$script_path" ]; then
        log_warn "dev.sh already exists. Backing up to dev.sh.bak"
        mv "$script_path" "$script_path.bak"
    fi

    if [ "$mode" = "kitchen-sink" ]; then
        # Embed kitchen-sink script directly for reliability
        cat > "$script_path" << 'DEVSH_EOF'
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR"

need_cmd() { command -v "$1" >/dev/null 2>&1; }

export REPO_ROOT

# Kitchen Sink Mode: Use globally installed devkit binary
if need_cmd devkit; then
  exec devkit "$@"
fi

# If devkit not found, provide installation instructions
echo "❌ devkit not found in PATH"
echo
echo "Install devkit by running:"
echo
echo "  curl -fsSL https://raw.githubusercontent.com/crcn/devkit/main/install.sh | bash"
echo
echo "Or if you have devkit installed elsewhere, add it to your PATH:"
echo "  export PATH=\"/path/to/devkit:\$PATH\""
echo

exit 1
DEVSH_EOF
    else
        # Download custom CLI script
        curl -fsSL "$RAW_URL/templates/dev.sh" -o "$script_path" || {
            log_error "Failed to download custom CLI template"
            return 1
        }
    fi

    chmod +x "$script_path"

    log_success "Installed dev.sh ($mode mode)"
}

create_dev_cli() {
    local root="$1"
    local mode="$2"
    local cli_dir="$root/dev/cli"

    if [ "$mode" = "kitchen-sink" ]; then
        log_info "Kitchen sink mode: Using devkit-cli binary (no local CLI needed)"
        return
    fi

    log_info "Creating dev/cli project..."

    if [ -d "$cli_dir" ]; then
        log_warn "dev/cli already exists. Skipping."
        return
    fi

    mkdir -p "$cli_dir/src"

    # Download templates
    curl -fsSL "$RAW_URL/templates/cli/Cargo.toml" -o "$cli_dir/Cargo.toml"
    curl -fsSL "$RAW_URL/templates/cli/main.rs" -o "$cli_dir/src/main.rs"

    log_success "Created dev/cli"
}

create_config() {
    local root="$1"
    local config_dir="$root/.dev"
    local config_path="$config_dir/config.toml"

    log_info "Creating .dev/config.toml..."

    if [ -f "$config_path" ]; then
        log_warn ".dev/config.toml already exists. Skipping."
        return
    fi

    mkdir -p "$config_dir"

    # Detect project name (use directory name)
    local project_name=$(basename "$root")

    # Detect project type for defaults
    local types=$(detect_project_type "$root")

    # Download and customize template
    curl -fsSL "$RAW_URL/templates/config.toml" -o "$config_path"

    # Replace project name placeholder
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/name = \"my-project\"/name = \"$project_name\"/" "$config_path"
    else
        sed -i "s/name = \"my-project\"/name = \"$project_name\"/" "$config_path"
    fi

    log_success "Created .dev/config.toml"
}

create_example_package_config() {
    local root="$1"

    # Find a suitable package directory
    for pattern in "packages/*" "crates/*"; do
        for dir in $root/$pattern; do
            if [ -d "$dir" ] && [ ! -f "$dir/dev.toml" ]; then
                log_info "Creating example dev.toml in $(basename "$dir")..."

                # Detect if Rust or Node
                if [ -f "$dir/Cargo.toml" ]; then
                    curl -fsSL "$RAW_URL/templates/dev.toml.rust" -o "$dir/dev.toml"
                elif [ -f "$dir/package.json" ]; then
                    curl -fsSL "$RAW_URL/templates/dev.toml.node" -o "$dir/dev.toml"
                fi

                log_success "Created example dev.toml"
                return
            fi
        done
    done
}

add_to_gitignore() {
    local root="$1"
    local gitignore="$root/.gitignore"

    log_info "Updating .gitignore..."

    local entries=(
        "# devkit"
        "/target/"
        "/.env.*.local"
        "/dev/cli/target/"
    )

    if [ ! -f "$gitignore" ]; then
        touch "$gitignore"
    fi

    for entry in "${entries[@]}"; do
        if ! grep -qF "$entry" "$gitignore"; then
            echo "$entry" >> "$gitignore"
        fi
    done

    log_success "Updated .gitignore"
}

# =============================================================================
# Post-Install
# =============================================================================

print_next_steps() {
    local root="$1"
    local mode="$2"

    echo
    echo -e "${GREEN}${BOLD}✓ devkit installed successfully!${NC}"
    echo
    echo -e "${BOLD}Mode:${NC} ${CYAN}$mode${NC}"
    echo

    if [ "$mode" = "kitchen-sink" ]; then
        echo -e "${BOLD}Next steps:${NC}"
        echo
        echo -e "  ${CYAN}1.${NC} Review ${BOLD}.dev/config.toml${NC}"
        echo -e "     Enable/disable features in ${BLUE}[features]${NC} section"
        echo
        echo -e "  ${CYAN}2.${NC} Add commands to package ${BOLD}dev.toml${NC} files:"
        echo
        echo -e "     ${BLUE}[cmd]${NC}"
        echo -e "     ${BLUE}build${NC} = \"cargo build\""
        echo -e "     ${BLUE}test${NC} = \"cargo test\""
        echo
        echo -e "  ${CYAN}3.${NC} Run the CLI (${BOLD}devkit${NC} is now globally available!):"
        echo
        echo -e "     ${GREEN}\$ devkit${NC}               ${YELLOW}# Interactive menu${NC}"
        echo -e "     ${GREEN}\$ devkit start${NC}         ${YELLOW}# Start everything${NC}"
        echo -e "     ${GREEN}\$ devkit cmd build${NC}     ${YELLOW}# Run build commands${NC}"
        echo -e "     ${GREEN}\$ devkit status${NC}        ${YELLOW}# Show status${NC}"
        echo
        echo -e "     ${YELLOW}Or use the wrapper:${NC}"
        echo -e "     ${GREEN}\$ ./dev.sh${NC}             ${YELLOW}# Same as 'devkit'${NC}"
    else
        echo -e "${BOLD}Next steps:${NC}"
        echo
        echo -e "  ${CYAN}1.${NC} Review ${BOLD}.dev/config.toml${NC}"
        echo
        echo -e "  ${CYAN}2.${NC} Add commands to package ${BOLD}dev.toml${NC} files"
        echo
        echo -e "  ${CYAN}3.${NC} Customize ${BOLD}dev/cli/src/main.rs${NC} for your project"
        echo -e "     Add devkit extensions in ${BOLD}dev/cli/Cargo.toml${NC}"
        echo
        echo -e "  ${CYAN}4.${NC} Run the CLI:"
        echo
        echo -e "     ${GREEN}\$ ./dev.sh${NC}              ${YELLOW}# Interactive menu${NC}"
        echo -e "     ${GREEN}\$ ./dev.sh start${NC}        ${YELLOW}# Start development${NC}"
        echo -e "     ${GREEN}\$ ./dev.sh cmd build${NC}    ${YELLOW}# Run package commands${NC}"
    fi

    echo
    echo -e "${BOLD}Documentation:${NC}"
    echo -e "  ${BLUE}$REPO_URL${NC}"
    echo
}

# =============================================================================
# Main
# =============================================================================

main() {
    echo
    echo -e "${BOLD}${BLUE}devkit installer${NC}"
    echo

    # Check dependencies
    check_dependencies
    echo

    # Detect project root
    PROJECT_ROOT=$(detect_project_root)
    log_info "Installing to: $PROJECT_ROOT"

    # Select mode
    MODE=$(select_mode)
    echo

    # Install devkit binary
    install_devkit_binary
    echo

    # Install components
    install_wrapper_script "$PROJECT_ROOT" "$MODE"
    create_dev_cli "$PROJECT_ROOT" "$MODE"
    create_config "$PROJECT_ROOT"
    create_example_package_config "$PROJECT_ROOT"
    add_to_gitignore "$PROJECT_ROOT"

    # Print next steps
    print_next_steps "$PROJECT_ROOT" "$MODE"
}

main "$@"
