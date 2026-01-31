# Siphon Shell Hook for Bash
#
# This hook captures shell commands and sends them to the Siphon daemon.
# Events are sent asynchronously with zero impact on shell responsiveness.
#
# Installation:
#   Add to your ~/.bashrc:
#   source /path/to/siphon-hook.bash
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
_siphon_last_cmd=""

# Get current time in milliseconds (portable)
_siphon_get_time_ms() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS: use perl for millisecond precision
        perl -MTime::HiRes=time -e 'printf "%.3f\n", time'
    else
        # Linux: use date with nanoseconds
        date +%s.%N
    fi
}

# Escape string for JSON (simple implementation)
_siphon_json_escape() {
    local str="$1"
    # Escape backslashes first, then quotes, then newlines/tabs
    str="${str//\\/\\\\}"
    str="${str//\"/\\\"}"
    str="${str//$'\n'/\\n}"
    str="${str//$'\r'/\\r}"
    str="${str//$'\t'/\\t}"
    printf '%s' "$str"
}

# Runs before each command execution (via DEBUG trap)
_siphon_preexec() {
    [[ "$SIPHON_ENABLED" != "true" ]] && return

    # Avoid running for PROMPT_COMMAND itself
    [[ "$BASH_COMMAND" == "$PROMPT_COMMAND" ]] && return
    [[ "$BASH_COMMAND" == "_siphon_precmd" ]] && return
    [[ "$BASH_COMMAND" == "_siphon_preexec" ]] && return

    # Only capture the first command (not subcommands in pipes)
    if [[ -z "$_siphon_cmd_start_time" ]]; then
        _siphon_cmd_start_time=$(_siphon_get_time_ms)
        _siphon_cmd_string="$BASH_COMMAND"
    fi
}

# Runs after each command completes (via PROMPT_COMMAND)
_siphon_precmd() {
    local exit_code=$?

    [[ "$SIPHON_ENABLED" != "true" ]] && return
    [[ -z "$_siphon_cmd_string" ]] && return

    # Avoid duplicate sends for the same command
    [[ "$_siphon_cmd_string" == "$_siphon_last_cmd" ]] && return

    local end_time
    end_time=$(_siphon_get_time_ms)
    local duration_ms=0

    # Calculate duration in milliseconds
    if [[ -n "$_siphon_cmd_start_time" ]]; then
        # Use awk for floating point arithmetic (bash doesn't support it)
        duration_ms=$(awk "BEGIN {printf \"%d\", ($end_time - $_siphon_cmd_start_time) * 1000}")
    fi

    local cwd="$PWD"
    local git_branch=""

    # Get git branch if in a repo (fast check)
    if [[ -d .git ]] || git rev-parse --git-dir &>/dev/null 2>&1; then
        git_branch=$(git symbolic-ref --short HEAD 2>/dev/null || git rev-parse --short HEAD 2>/dev/null)
    fi

    # Build JSON payload
    local escaped_cmd escaped_cwd escaped_branch
    escaped_cmd=$(_siphon_json_escape "$_siphon_cmd_string")
    escaped_cwd=$(_siphon_json_escape "$cwd")
    escaped_branch=$(_siphon_json_escape "$git_branch")

    local payload
    payload=$(cat <<EOF
{
    "command": "$escaped_cmd",
    "exit_code": $exit_code,
    "duration_ms": $duration_ms,
    "cwd": "$escaped_cwd",
    "git_branch": "$escaped_branch",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%S.000Z)"
}
EOF
)

    # Send to daemon in background, fail silently
    (curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$payload" \
        "${SIPHON_API_URL}/events/shell" \
        --connect-timeout 1 \
        --max-time 2 \
        >/dev/null 2>&1) &
    disown 2>/dev/null

    # Track last command to avoid duplicates
    _siphon_last_cmd="$_siphon_cmd_string"

    # Reset state
    _siphon_cmd_start_time=""
    _siphon_cmd_string=""
}

# Register hooks
trap '_siphon_preexec' DEBUG

# Append to PROMPT_COMMAND (preserve existing)
if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="_siphon_precmd"
elif [[ "$PROMPT_COMMAND" != *"_siphon_precmd"* ]]; then
    PROMPT_COMMAND="_siphon_precmd;$PROMPT_COMMAND"
fi

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
