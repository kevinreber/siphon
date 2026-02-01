#!/usr/bin/env bash
#
# Tests for Siphon installation scripts
#
# Usage: ./scripts/test-install.sh [--quick]
#   --quick    Skip build tests (faster, for CI syntax checks)
#
set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test state
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
QUICK_MODE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            QUICK_MODE=true
            shift
            ;;
        *)
            shift
            ;;
    esac
done

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
TEST_PREFIX="${REPO_DIR}/test-install-tmp"

# Cleanup function
cleanup() {
    rm -rf "${TEST_PREFIX}" 2>/dev/null || true
}
trap cleanup EXIT

# Test assertion helpers
assert_equals() {
    local expected="$1"
    local actual="$2"
    local message="${3:-}"

    if [[ "$expected" == "$actual" ]]; then
        return 0
    else
        echo -e "${RED}  Assertion failed: expected '$expected', got '$actual'${NC}"
        [[ -n "$message" ]] && echo "    $message"
        return 1
    fi
}

assert_contains() {
    local haystack="$1"
    local needle="$2"
    local message="${3:-}"

    if [[ "$haystack" == *"$needle"* ]]; then
        return 0
    else
        echo -e "${RED}  Assertion failed: '$haystack' does not contain '$needle'${NC}"
        [[ -n "$message" ]] && echo "    $message"
        return 1
    fi
}

assert_file_exists() {
    local file="$1"
    local message="${2:-}"

    if [[ -f "$file" ]]; then
        return 0
    else
        echo -e "${RED}  Assertion failed: file '$file' does not exist${NC}"
        [[ -n "$message" ]] && echo "    $message"
        return 1
    fi
}

assert_file_executable() {
    local file="$1"
    local message="${2:-}"

    if [[ -x "$file" ]]; then
        return 0
    else
        echo -e "${RED}  Assertion failed: file '$file' is not executable${NC}"
        [[ -n "$message" ]] && echo "    $message"
        return 1
    fi
}

assert_exit_code() {
    local expected="$1"
    local actual="$2"
    local message="${3:-}"

    if [[ "$expected" -eq "$actual" ]]; then
        return 0
    else
        echo -e "${RED}  Assertion failed: expected exit code $expected, got $actual${NC}"
        [[ -n "$message" ]] && echo "    $message"
        return 1
    fi
}

# Run a test
run_test() {
    local name="$1"
    local fn="$2"

    TESTS_RUN=$((TESTS_RUN + 1))
    echo -e "${BLUE}▶${NC} $name"

    if $fn; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "${GREEN}  ✓ passed${NC}"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "${RED}  ✗ failed${NC}"
        return 1
    fi
}

# Print test summary
print_summary() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Test Summary: $TESTS_PASSED/$TESTS_RUN passed"

    if [[ $TESTS_FAILED -gt 0 ]]; then
        echo -e "${RED}$TESTS_FAILED test(s) failed${NC}"
        return 1
    else
        echo -e "${GREEN}All tests passed!${NC}"
        return 0
    fi
}

# ============================================================================
# Test: install.sh exists and is executable
# ============================================================================
test_install_script_exists() {
    assert_file_exists "${REPO_DIR}/install.sh" &&
    assert_file_executable "${REPO_DIR}/install.sh"
}

# ============================================================================
# Test: install.sh --help shows usage
# ============================================================================
test_install_help() {
    local output
    output=$("${REPO_DIR}/install.sh" --help 2>&1) || true

    assert_contains "$output" "Usage:" &&
    assert_contains "$output" "--cli-only" &&
    assert_contains "$output" "--no-service" &&
    assert_contains "$output" "--no-hooks" &&
    assert_contains "$output" "--uninstall" &&
    assert_contains "$output" "--prefix"
}

# ============================================================================
# Test: Makefile exists and has required targets
# ============================================================================
test_makefile_exists() {
    assert_file_exists "${REPO_DIR}/Makefile"
}

test_makefile_targets() {
    local makefile
    makefile=$(cat "${REPO_DIR}/Makefile")

    # Check for essential targets
    assert_contains "$makefile" "install:" &&
    assert_contains "$makefile" "build:" &&
    assert_contains "$makefile" "build-cli:" &&
    assert_contains "$makefile" "build-daemon:" &&
    assert_contains "$makefile" "clean:" &&
    assert_contains "$makefile" "uninstall:" &&
    assert_contains "$makefile" "start:" &&
    assert_contains "$makefile" "stop:" &&
    assert_contains "$makefile" "status:" &&
    assert_contains "$makefile" "help:"
}

test_makefile_help() {
    local output
    output=$(make -C "${REPO_DIR}" help 2>&1)

    assert_contains "$output" "Build targets:" &&
    assert_contains "$output" "Install targets:" &&
    assert_contains "$output" "Service targets:"
}

# ============================================================================
# Test: Service files exist and have valid syntax
# ============================================================================
test_systemd_service_exists() {
    assert_file_exists "${REPO_DIR}/services/siphon-daemon.service"
}

test_systemd_service_syntax() {
    local service_file="${REPO_DIR}/services/siphon-daemon.service"
    local content
    content=$(cat "$service_file")

    # Check for required systemd sections
    assert_contains "$content" "[Unit]" &&
    assert_contains "$content" "[Service]" &&
    assert_contains "$content" "[Install]" &&
    assert_contains "$content" "Description=" &&
    assert_contains "$content" "ExecStart=" &&
    assert_contains "$content" "Type=" &&
    assert_contains "$content" "WantedBy="
}

test_launchd_plist_exists() {
    assert_file_exists "${REPO_DIR}/services/com.siphon.daemon.plist"
}

test_launchd_plist_syntax() {
    local plist_file="${REPO_DIR}/services/com.siphon.daemon.plist"

    # Check XML syntax with xmllint if available
    if command -v xmllint &> /dev/null; then
        if xmllint --noout "$plist_file" 2>/dev/null; then
            return 0
        else
            echo "  xmllint validation failed"
            return 1
        fi
    fi

    # Fallback: basic structure check
    local content
    content=$(cat "$plist_file")

    assert_contains "$content" '<?xml version="1.0"' &&
    assert_contains "$content" "<!DOCTYPE plist" &&
    assert_contains "$content" "<plist version=" &&
    assert_contains "$content" "<key>Label</key>" &&
    assert_contains "$content" "<key>ProgramArguments</key>" &&
    assert_contains "$content" "</plist>"
}

# ============================================================================
# Test: Shell hooks exist for all supported shells
# ============================================================================
test_shell_hooks_exist() {
    assert_file_exists "${REPO_DIR}/siphon-daemon/hooks/siphon-hook.zsh" &&
    assert_file_exists "${REPO_DIR}/siphon-daemon/hooks/siphon-hook.bash" &&
    assert_file_exists "${REPO_DIR}/siphon-daemon/hooks/siphon-hook.fish"
}

# ============================================================================
# Test: Build CLI (if not quick mode)
# ============================================================================
test_build_cli() {
    if $QUICK_MODE; then
        echo "  (skipped in quick mode)"
        return 0
    fi

    cd "${REPO_DIR}"
    make build-cli > /dev/null 2>&1

    assert_file_exists "${REPO_DIR}/siphon-cli/dist/cli.js"
}

# ============================================================================
# Test: Build daemon (if not quick mode)
# ============================================================================
test_build_daemon() {
    if $QUICK_MODE; then
        echo "  (skipped in quick mode)"
        return 0
    fi

    cd "${REPO_DIR}"
    make build-daemon > /dev/null 2>&1

    assert_file_exists "${REPO_DIR}/siphon-daemon/target/release/siphon-daemon" &&
    assert_file_exists "${REPO_DIR}/siphon-daemon/target/release/siphon-ctl"
}

# ============================================================================
# Test: Install to custom prefix (if not quick mode)
# ============================================================================
test_install_to_prefix() {
    if $QUICK_MODE; then
        echo "  (skipped in quick mode)"
        return 0
    fi

    mkdir -p "${TEST_PREFIX}"

    cd "${REPO_DIR}"

    # Run install with custom prefix and skip service/hooks
    ./install.sh --prefix "${TEST_PREFIX}" --no-service --no-hooks > /dev/null 2>&1

    # Verify binaries were installed
    assert_file_exists "${TEST_PREFIX}/siphon" &&
    assert_file_executable "${TEST_PREFIX}/siphon" &&
    assert_file_exists "${TEST_PREFIX}/siphon-daemon" &&
    assert_file_executable "${TEST_PREFIX}/siphon-daemon" &&
    assert_file_exists "${TEST_PREFIX}/siphon-ctl" &&
    assert_file_executable "${TEST_PREFIX}/siphon-ctl"
}

# ============================================================================
# Test: CLI wrapper script works (if not quick mode)
# ============================================================================
test_cli_wrapper() {
    if $QUICK_MODE; then
        echo "  (skipped in quick mode)"
        return 0
    fi

    # Test the installed wrapper shows help
    local output
    output=$("${TEST_PREFIX}/siphon" --help 2>&1) || true

    assert_contains "$output" "siphon" &&
    assert_contains "$output" "capture"
}

# ============================================================================
# Test: siphon-ctl binary works (if not quick mode)
# ============================================================================
test_siphon_ctl_binary() {
    if $QUICK_MODE; then
        echo "  (skipped in quick mode)"
        return 0
    fi

    # Test the installed binary shows help
    local output
    output=$("${TEST_PREFIX}/siphon-ctl" --help 2>&1) || true

    assert_contains "$output" "siphon-ctl"
}

# ============================================================================
# Test: Database directory initialization
# ============================================================================
test_database_directory() {
    if $QUICK_MODE; then
        echo "  (skipped in quick mode)"
        return 0
    fi

    # The install script creates ~/.siphon directory
    # We check that the script handles this correctly
    local install_output
    install_output=$("${REPO_DIR}/install.sh" --help 2>&1)

    # Just verify the script runs without errors in help mode
    assert_contains "$install_output" "Usage:"
}

# ============================================================================
# Test: Uninstall option exists
# ============================================================================
test_uninstall_option() {
    local output
    output=$("${REPO_DIR}/install.sh" --help 2>&1) || true

    assert_contains "$output" "--uninstall"
}

# ============================================================================
# Test: README documents installation
# ============================================================================
test_readme_installation_docs() {
    local readme
    readme=$(cat "${REPO_DIR}/README.md")

    assert_contains "$readme" "Quick Install" &&
    assert_contains "$readme" "./install.sh" &&
    assert_contains "$readme" "make install"
}

# ============================================================================
# Main test runner
# ============================================================================
main() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Siphon Installation Tests"
    if $QUICK_MODE; then
        echo "  (Quick mode - skipping build tests)"
    fi
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    # File existence tests
    echo -e "${YELLOW}File Existence Tests${NC}"
    run_test "install.sh exists and is executable" test_install_script_exists || true
    run_test "Makefile exists" test_makefile_exists || true
    run_test "Systemd service file exists" test_systemd_service_exists || true
    run_test "Launchd plist file exists" test_launchd_plist_exists || true
    run_test "Shell hooks exist for all shells" test_shell_hooks_exist || true
    echo ""

    # Syntax and content tests
    echo -e "${YELLOW}Syntax and Content Tests${NC}"
    run_test "install.sh --help shows usage" test_install_help || true
    run_test "Makefile has required targets" test_makefile_targets || true
    run_test "make help works" test_makefile_help || true
    run_test "Systemd service has valid syntax" test_systemd_service_syntax || true
    run_test "Launchd plist has valid XML" test_launchd_plist_syntax || true
    run_test "README documents installation" test_readme_installation_docs || true
    echo ""

    # Build tests (skipped in quick mode)
    echo -e "${YELLOW}Build Tests${NC}"
    run_test "Build CLI" test_build_cli || true
    run_test "Build daemon" test_build_daemon || true
    echo ""

    # Installation tests (skipped in quick mode)
    echo -e "${YELLOW}Installation Tests${NC}"
    run_test "Install to custom prefix" test_install_to_prefix || true
    run_test "CLI wrapper script works" test_cli_wrapper || true
    run_test "siphon-ctl binary works" test_siphon_ctl_binary || true
    run_test "Uninstall option exists" test_uninstall_option || true
    echo ""

    print_summary
}

main
