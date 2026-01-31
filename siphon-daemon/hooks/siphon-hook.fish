# Siphon Shell Hook for Fish
#
# This hook captures shell commands and sends them to the Siphon daemon.
# Events are sent asynchronously with zero impact on shell responsiveness.
#
# Installation:
#   Add to your ~/.config/fish/config.fish:
#   source /path/to/siphon-hook.fish
#
# The hook captures:
#   - Command string
#   - Exit code
#   - Duration (milliseconds)
#   - Working directory
#   - Git branch (if in a git repo)

# Configuration
set -gx SIPHON_API_URL (test -n "$SIPHON_API_URL"; and echo $SIPHON_API_URL; or echo "http://127.0.0.1:9847")
set -gx SIPHON_ENABLED (test -n "$SIPHON_ENABLED"; and echo $SIPHON_ENABLED; or echo "true")

# Internal state
set -g _siphon_cmd_start_time ""
set -g _siphon_last_cmd ""

# Get current time in milliseconds
function _siphon_get_time_ms
    if test (uname) = "Darwin"
        # macOS: use perl for millisecond precision
        perl -MTime::HiRes=time -e 'printf "%.3f\n", time'
    else
        # Linux: use date with nanoseconds
        date +%s.%N
    end
end

# Escape string for JSON
function _siphon_json_escape
    set -l str $argv[1]
    # Use sed to escape special characters
    echo -n $str | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g' | tr '\n' ' '
end

# Pre-execution hook - runs before each command
function _siphon_preexec --on-event fish_preexec
    if test "$SIPHON_ENABLED" != "true"
        return
    end

    set -g _siphon_cmd_start_time (_siphon_get_time_ms)
end

# Post-execution hook - runs after each command
function _siphon_postexec --on-event fish_postexec
    set -l exit_code $status

    if test "$SIPHON_ENABLED" != "true"
        return
    end

    set -l cmd_string $argv[1]

    # Skip empty commands
    if test -z "$cmd_string"
        return
    end

    # Avoid duplicate sends
    if test "$cmd_string" = "$_siphon_last_cmd"
        return
    end

    set -l end_time (_siphon_get_time_ms)
    set -l duration_ms 0

    # Calculate duration in milliseconds
    if test -n "$_siphon_cmd_start_time"
        set duration_ms (math "($end_time - $_siphon_cmd_start_time) * 1000" | cut -d'.' -f1)
    end

    set -l cwd (pwd)
    set -l git_branch ""

    # Get git branch if in a repo
    if test -d .git; or git rev-parse --git-dir >/dev/null 2>&1
        set git_branch (git symbolic-ref --short HEAD 2>/dev/null; or git rev-parse --short HEAD 2>/dev/null)
    end

    # Escape strings for JSON
    set -l escaped_cmd (_siphon_json_escape "$cmd_string")
    set -l escaped_cwd (_siphon_json_escape "$cwd")
    set -l escaped_branch (_siphon_json_escape "$git_branch")

    # Build JSON payload
    set -l payload "{
        \"command\": \"$escaped_cmd\",
        \"exit_code\": $exit_code,
        \"duration_ms\": $duration_ms,
        \"cwd\": \"$escaped_cwd\",
        \"git_branch\": \"$escaped_branch\",
        \"timestamp\": \"(date -u +%Y-%m-%dT%H:%M:%S.000Z)\"
    }"

    # Send to daemon in background, fail silently
    fish -c "curl -s -X POST \
        -H 'Content-Type: application/json' \
        -d '$payload' \
        '$SIPHON_API_URL/events/shell' \
        --connect-timeout 1 \
        --max-time 2 \
        >/dev/null 2>&1" &
    disown 2>/dev/null

    # Track last command
    set -g _siphon_last_cmd $cmd_string

    # Reset state
    set -g _siphon_cmd_start_time ""
end

# Utility functions

# Temporarily pause tracking
function siphon-pause
    set -gx SIPHON_ENABLED "false"
    echo "Siphon tracking paused. Run 'siphon-resume' to resume."
end

# Resume tracking
function siphon-resume
    set -gx SIPHON_ENABLED "true"
    echo "Siphon tracking resumed."
end

# Check if daemon is running
function siphon-status
    if curl -s "$SIPHON_API_URL/health" --connect-timeout 1 >/dev/null 2>&1
        echo "Siphon daemon is running at $SIPHON_API_URL"
    else
        echo "Siphon daemon is not running"
    end
end
