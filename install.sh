#!/usr/bin/env bash
#
# Siphon Installer
# One-command installation for CLI, daemon, and database
#
# Usage: ./install.sh [options]
#   --cli-only      Only install the CLI tool
#   --no-service    Don't set up daemon as a system service
#   --no-hooks      Don't configure shell hooks
#   --uninstall     Remove Siphon completely
#   --prefix PATH   Install binaries to PATH (default: ~/.local/bin)
#
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
INSTALL_PREFIX="${HOME}/.local/bin"
INSTALL_CLI=true
INSTALL_DAEMON=true
SETUP_SERVICE=true
SETUP_HOOKS=true
UNINSTALL=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --cli-only)
            INSTALL_DAEMON=false
            SETUP_SERVICE=false
            SETUP_HOOKS=false
            shift
            ;;
        --no-service)
            SETUP_SERVICE=false
            shift
            ;;
        --no-hooks)
            SETUP_HOOKS=false
            shift
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        --prefix)
            INSTALL_PREFIX="$2"
            shift 2
            ;;
        -h|--help)
            echo "Siphon Installer"
            echo ""
            echo "Usage: ./install.sh [options]"
            echo ""
            echo "Options:"
            echo "  --cli-only      Only install the CLI tool"
            echo "  --no-service    Don't set up daemon as a system service"
            echo "  --no-hooks      Don't configure shell hooks"
            echo "  --uninstall     Remove Siphon completely"
            echo "  --prefix PATH   Install binaries to PATH (default: ~/.local/bin)"
            echo "  -h, --help      Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     OS="linux";;
        Darwin*)    OS="macos";;
        *)          OS="unknown";;
    esac
    echo "$OS"
}

# Detect shell
detect_shell() {
    local shell_name
    shell_name=$(basename "$SHELL")
    echo "$shell_name"
}

# Print step header
step() {
    echo -e "\n${BLUE}==>${NC} $1"
}

# Print success
success() {
    echo -e "${GREEN}✓${NC} $1"
}

# Print warning
warn() {
    echo -e "${YELLOW}!${NC} $1"
}

# Print error
error() {
    echo -e "${RED}✗${NC} $1"
}

# Check for required dependencies
check_dependencies() {
    step "Checking dependencies..."

    local missing=()

    if $INSTALL_CLI; then
        if ! command -v node &> /dev/null; then
            missing+=("node")
        fi
        if ! command -v npm &> /dev/null; then
            missing+=("npm")
        fi
    fi

    if $INSTALL_DAEMON; then
        if ! command -v cargo &> /dev/null; then
            missing+=("cargo (Rust)")
        fi
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        error "Missing required dependencies: ${missing[*]}"
        echo ""
        echo "Please install the missing dependencies:"
        echo "  - Node.js/npm: https://nodejs.org/"
        echo "  - Rust/Cargo: https://rustup.rs/"
        exit 1
    fi

    success "All dependencies found"
}

# Get the script directory (where siphon repo is)
get_script_dir() {
    cd "$(dirname "${BASH_SOURCE[0]}")" && pwd
}

# Build the CLI
build_cli() {
    step "Building CLI..."

    local repo_dir
    repo_dir=$(get_script_dir)

    cd "${repo_dir}/siphon-cli"
    npm install --silent
    npm run build --silent

    success "CLI built successfully"
}

# Build the daemon
build_daemon() {
    step "Building daemon..."

    local repo_dir
    repo_dir=$(get_script_dir)

    cd "${repo_dir}/siphon-daemon"
    cargo build --release --quiet

    success "Daemon built successfully"
}

# Install binaries
install_binaries() {
    step "Installing binaries to ${INSTALL_PREFIX}..."

    local repo_dir
    repo_dir=$(get_script_dir)

    # Create install directory if needed
    mkdir -p "${INSTALL_PREFIX}"

    if $INSTALL_CLI; then
        # Create a wrapper script for the CLI
        cat > "${INSTALL_PREFIX}/siphon" << EOF
#!/usr/bin/env bash
exec node "${repo_dir}/siphon-cli/dist/cli.js" "\$@"
EOF
        chmod +x "${INSTALL_PREFIX}/siphon"
        success "Installed: siphon (CLI)"
    fi

    if $INSTALL_DAEMON; then
        # Copy daemon binaries
        cp "${repo_dir}/siphon-daemon/target/release/siphon-daemon" "${INSTALL_PREFIX}/"
        cp "${repo_dir}/siphon-daemon/target/release/siphon-ctl" "${INSTALL_PREFIX}/"
        chmod +x "${INSTALL_PREFIX}/siphon-daemon"
        chmod +x "${INSTALL_PREFIX}/siphon-ctl"
        success "Installed: siphon-daemon"
        success "Installed: siphon-ctl"
    fi

    # Check if INSTALL_PREFIX is in PATH
    if [[ ":$PATH:" != *":${INSTALL_PREFIX}:"* ]]; then
        warn "${INSTALL_PREFIX} is not in your PATH"
        echo "    Add this to your shell config:"
        echo "    export PATH=\"\$PATH:${INSTALL_PREFIX}\""
    fi
}

# Initialize the database
init_database() {
    step "Initializing database..."

    local db_dir="${HOME}/.siphon"
    mkdir -p "${db_dir}"

    # Database is auto-initialized on first daemon run
    # Just create the directory and touch a marker file
    touch "${db_dir}/.initialized"

    success "Database directory ready: ${db_dir}"
}

# Setup systemd service (Linux)
setup_systemd_service() {
    local service_dir="${HOME}/.config/systemd/user"
    local service_file="${service_dir}/siphon-daemon.service"

    mkdir -p "${service_dir}"

    cat > "${service_file}" << EOF
[Unit]
Description=Siphon Daemon - Development activity tracker
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_PREFIX}/siphon-daemon
Restart=on-failure
RestartSec=5
Environment=SIPHON_ENABLED=true

[Install]
WantedBy=default.target
EOF

    # Reload systemd and enable service
    systemctl --user daemon-reload
    systemctl --user enable siphon-daemon.service
    systemctl --user start siphon-daemon.service

    success "Systemd service installed and started"
    echo "    Commands: systemctl --user {start|stop|status|restart} siphon-daemon"
}

# Setup launchd service (macOS)
setup_launchd_service() {
    local plist_dir="${HOME}/Library/LaunchAgents"
    local plist_file="${plist_dir}/com.siphon.daemon.plist"

    mkdir -p "${plist_dir}"

    cat > "${plist_file}" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.siphon.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>${INSTALL_PREFIX}/siphon-daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${HOME}/.siphon/daemon.log</string>
    <key>StandardErrorPath</key>
    <string>${HOME}/.siphon/daemon.error.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>SIPHON_ENABLED</key>
        <string>true</string>
    </dict>
</dict>
</plist>
EOF

    # Load the service
    launchctl unload "${plist_file}" 2>/dev/null || true
    launchctl load "${plist_file}"

    success "Launchd service installed and started"
    echo "    Commands: launchctl {load|unload} ~/Library/LaunchAgents/com.siphon.daemon.plist"
}

# Setup daemon as a system service
setup_service() {
    step "Setting up daemon as system service..."

    local os
    os=$(detect_os)

    case "$os" in
        linux)
            setup_systemd_service
            ;;
        macos)
            setup_launchd_service
            ;;
        *)
            warn "Unknown OS. Please start the daemon manually:"
            echo "    ${INSTALL_PREFIX}/siphon-daemon &"
            ;;
    esac
}

# Setup shell hooks
setup_hooks() {
    step "Setting up shell hooks..."

    local repo_dir
    repo_dir=$(get_script_dir)
    local shell_name
    shell_name=$(detect_shell)

    local hook_file=""
    local rc_file=""
    local source_line=""

    case "$shell_name" in
        zsh)
            hook_file="${repo_dir}/siphon-daemon/hooks/siphon-hook.zsh"
            rc_file="${HOME}/.zshrc"
            ;;
        bash)
            hook_file="${repo_dir}/siphon-daemon/hooks/siphon-hook.bash"
            rc_file="${HOME}/.bashrc"
            ;;
        fish)
            hook_file="${repo_dir}/siphon-daemon/hooks/siphon-hook.fish"
            rc_file="${HOME}/.config/fish/config.fish"
            ;;
        *)
            warn "Unknown shell: ${shell_name}. Please manually source the appropriate hook file."
            return
            ;;
    esac

    source_line="source \"${hook_file}\"  # Siphon shell integration"

    # Check if already configured
    if [[ -f "$rc_file" ]] && grep -q "siphon-hook" "$rc_file"; then
        success "Shell hooks already configured in ${rc_file}"
        return
    fi

    # Add to shell config
    echo "" >> "$rc_file"
    echo "# Siphon - Development activity tracking" >> "$rc_file"
    echo "${source_line}" >> "$rc_file"

    success "Added shell hooks to ${rc_file}"
    echo "    Run 'source ${rc_file}' or restart your terminal to activate"
}

# Uninstall Siphon
uninstall() {
    step "Uninstalling Siphon..."

    local os
    os=$(detect_os)

    # Stop and remove service
    if [[ "$os" == "linux" ]]; then
        systemctl --user stop siphon-daemon.service 2>/dev/null || true
        systemctl --user disable siphon-daemon.service 2>/dev/null || true
        rm -f "${HOME}/.config/systemd/user/siphon-daemon.service"
        systemctl --user daemon-reload
        success "Removed systemd service"
    elif [[ "$os" == "macos" ]]; then
        launchctl unload "${HOME}/Library/LaunchAgents/com.siphon.daemon.plist" 2>/dev/null || true
        rm -f "${HOME}/Library/LaunchAgents/com.siphon.daemon.plist"
        success "Removed launchd service"
    fi

    # Remove binaries
    rm -f "${INSTALL_PREFIX}/siphon"
    rm -f "${INSTALL_PREFIX}/siphon-daemon"
    rm -f "${INSTALL_PREFIX}/siphon-ctl"
    success "Removed binaries from ${INSTALL_PREFIX}"

    # Ask about data
    echo ""
    read -p "Remove Siphon data directory (~/.siphon)? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "${HOME}/.siphon"
        success "Removed data directory"
    fi

    # Note about shell config
    warn "Please manually remove the Siphon lines from your shell config file"
    echo "    Look for lines containing 'siphon-hook' in ~/.zshrc, ~/.bashrc, or ~/.config/fish/config.fish"

    echo ""
    echo -e "${GREEN}Siphon has been uninstalled.${NC}"
}

# Print completion message
print_completion() {
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}  Siphon installed successfully!${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo "Quick start:"
    echo ""
    if $INSTALL_CLI; then
        echo "  ${BLUE}siphon status${NC}          # See recent activity summary"
        echo "  ${BLUE}siphon capture${NC}         # Capture and analyze last 2 hours"
    fi
    if $INSTALL_DAEMON; then
        echo "  ${BLUE}siphon-ctl status${NC}      # Check if daemon is running"
        echo "  ${BLUE}siphon-ctl ideas -H 4${NC}  # Get content ideas from last 4 hours"
    fi
    echo ""
    echo "Data is stored locally at: ~/.siphon/"
    echo ""
    if $SETUP_HOOKS; then
        echo -e "${YELLOW}Note:${NC} Restart your terminal or run 'source ~/.zshrc' to activate shell tracking"
    fi
}

# Main installation flow
main() {
    echo ""
    echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║                     Siphon Installer                          ║${NC}"
    echo -e "${BLUE}║      Turn your dev work into content — automatically          ║${NC}"
    echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"

    if $UNINSTALL; then
        uninstall
        exit 0
    fi

    check_dependencies

    if $INSTALL_CLI; then
        build_cli
    fi

    if $INSTALL_DAEMON; then
        build_daemon
    fi

    install_binaries

    if $INSTALL_DAEMON; then
        init_database
    fi

    if $SETUP_SERVICE && $INSTALL_DAEMON; then
        setup_service
    fi

    if $SETUP_HOOKS && $INSTALL_DAEMON; then
        setup_hooks
    fi

    print_completion
}

# Run main
main
