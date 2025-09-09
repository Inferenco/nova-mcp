#!/bin/bash

# Nova-MCP Build Script

set -e

echo "Building Nova-MCP Server..."
echo "=========================="

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

# Build the project
echo "Building release binary..."
cargo build --release --bin nova-mcp-stdio

# Check if build was successful
if [ -f "target/release/nova-mcp-stdio" ]; then
    echo "✅ Build successful!"
    echo "Binary location: target/release/nova-mcp-stdio"
    
    # Show binary info
    echo ""
    echo "Binary information:"
    ls -lh target/release/nova-mcp-stdio
    
    echo ""
    echo "To run the server:"
    echo "  ./target/release/nova-mcp-stdio"
    echo ""
    echo "Or use cargo:"
    echo "  cargo run --bin nova-mcp-stdio"
else
    echo "❌ Build failed!"
    exit 1
fi

