#!/bin/bash
# Deployment script

# TODO: Add error checking
cargo build --release
cp target/release/app /usr/local/bin/

echo "extract_pattern: Deployment complete"
