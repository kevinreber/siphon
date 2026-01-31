# Siphon Shell Hook for Zsh
#
# This hook captures shell commands and sends them to the Siphon daemon.
# Events are sent asynchronously with zero impact on shell responsiveness.
#
# Installation:
#   Add to your ~/.zshrc:
#   source /path/to/siphon-hook.zsh
#
# The hook captures:
#   - Command string
#   - Exit code
#   - Duration (milliseconds)
#   - Working directory
#   - Git branch (if in a git repo)

# Configuration
SIPHON_API_URL="${SIPHON_API_URL:-http://127.0.0.1:9847}"
SIPHON_ENABLED="${SIPHON_ENABLED:-true}"

# Internal state
_siphon_cmd_start_time=""
_siphon_cmd_string=""

# Runs before each command execution
_siphon_preexec() {
    [[ "$SIPHON_ENABLED" != "true" ]] && return

    # Record start time with millisecond precision
    _siphon_cmd_start_time="${EPOCHREALTIME:-$(date +%s.%N)}"
    _siphon_cmd_string="$1"
}

# Runs after each command completes (before prompt)
_siphon_precmd() {
    local exit_code=$?

    [[ "$SIPHON_ENABLED" != "true" ]] && return
    [[ -z "$_siphon_cmd_string" ]] && return

    local end_time="${EPOCHREALTIME:-$(date +%s.%N)}"
    local duration_ms

    # Calculate duration in milliseconds
    if [[ -n "$_siphon_cmd_start_time" ]]; then
        # Use zsh arithmetic for floating point
        duration_ms=$(( (end_time - _siphon_cmd_start_time) * 1000 ))
        duration_ms=${duration_ms%.*}  # Truncate to integer
    else
        duration_ms=0
    fi

    local cwd="$PWD"
    local git_branch=""

    # Get git branch if in a repo (fast check)
    if [[ -d .git ]] || git rev-parse --git-dir &>/dev/null 2>&1; then
        git_branch=$(git symbolic-ref --short HEAD 2>/dev/null || git rev-parse --short HEAD 2>/dev/null)
    fi

    # Build JSON payload
    local payload
    payload=$(cat <<EOF
{
    "command": $(printf '%s' "$_siphon_cmd_string" | jq -Rs .),
    "exit_code": $exit_code,
    "duration_ms": $duration_ms,
    "cwd": $(printf '%s' "$cwd" | jq -Rs .),
    "git_branch": $(printf '%s' "$git_branch" | jq -Rs .),
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%S.000Z)"
}
EOF
)

    # Send to daemon in background, fail silently
    # Using &! to disown immediately (zsh-specific)
    (curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$payload" \
        "${SIPHON_API_URL}/events/shell" \
        --connect-timeout 1 \
        --max-time 2 \
        >/dev/null 2>&1) &!

    # Reset state
    _siphon_cmd_start_time=""
    _siphon_cmd_string=""
}

# Register hooks
autoload -Uz add-zsh-hook
add-zsh-hook preexec _siphon_preexec
add-zsh-hook precmd _siphon_precmd

# Utility functions

# Temporarily pause tracking
siphon-pause() {
    export SIPHON_ENABLED="false"
    echo "Siphon tracking paused. Run 'siphon-resume' to resume."
}

# Resume tracking
siphon-resume() {
    export SIPHON_ENABLED="true"
    echo "Siphon tracking resumed."
}

# Check if daemon is running
siphon-status() {
    if curl -s "${SIPHON_API_URL}/health" --connect-timeout 1 >/dev/null 2>&1; then
        echo "Siphon daemon is running at ${SIPHON_API_URL}"
    else
        echo "Siphon daemon is not running"
    fi
}
