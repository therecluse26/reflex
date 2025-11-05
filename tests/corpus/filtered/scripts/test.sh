#!/bin/bash
# Test runner script

set -e

# TODO: Add coverage reporting
cargo test --all
cargo test test_extract --verbose

echo "All tests passed!"
