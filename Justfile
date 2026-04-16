# Default recipe to list all available tasks
default:
    @just --list

# Run all tests (clippy, unit tests, and example fuzzer verification)
@test duration="5": clippy test-unit (test-fuzzer duration)

# Check code formatting and lints
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Format codebase (Rust, TOML, YAML, Markdown, Justfile)
fmt:
    cargo fmt --all
    taplo format
    npx prettier --write "**/*.yml" "**/*.yaml" "**/*.md"
    just --fmt --unstable

# Check codebase formatting (Rust, TOML, YAML, Markdown, Justfile)
fmt-check:
    cargo fmt --all -- --check
    taplo format --check
    npx prettier --check "**/*.yml" "**/*.yaml" "**/*.md"
    just --fmt --check --unstable

# Run unit tests across the workspace
test-unit:
    cargo test --workspace

# Build the example target server
build-target:
    cd examples/example_target && make

# Build the example fuzzer
build-fuzzer:
    cd examples/example_fuzzer && cargo build

# Run the example fuzzer for a specified duration to ensure it runs successfully without crashing
@test-fuzzer duration="5": build-target build-fuzzer
    #!/usr/bin/env bash
    set -euo pipefail

    echo "Starting example target server..."
    ./examples/example_target/server &
    SERVER_PID=$!

    # Ensure server is cleaned up on exit
    trap 'kill $SERVER_PID 2>/dev/null || true' EXIT

    # Wait for server to start listening
    sleep 1

    echo "Running example fuzzer for {{ duration }} seconds..."
    # Run the fuzzer binary directly to avoid cargo overhead and accurately capture PID
    ./examples/target/debug/example_fuzzer &
    FUZZER_PID=$!

    # Sleep for the specified duration
    sleep {{ duration }}

    echo "Checking if fuzzer is still running..."
    if kill -0 $FUZZER_PID 2>/dev/null; then
        echo "Fuzzer is running fine. Stopping it gracefully..."
        kill -INT $FUZZER_PID
        wait $FUZZER_PID || true
        echo "Fuzzer ran successfully for {{ duration }} seconds."
    else
        echo "Error: Fuzzer crashed or stopped prematurely!"
        # Check if server crashed
        if ! kill -0 $SERVER_PID 2>/dev/null; then
            echo "Error: Server crashed!"
        fi
        exit 1
    fi
