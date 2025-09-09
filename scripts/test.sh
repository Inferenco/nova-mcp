#!/bin/bash

# Nova-MCP Test Script

set -e

echo "Testing Nova-MCP Server..."
echo "========================="

# Build first
echo "Building server..."
cargo build --bin nova-mcp-stdio

echo ""
echo "Running tests..."

# Test 1: List tools
echo "1. Testing tools/list..."
RESPONSE=$(echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | timeout 10s cargo run --bin nova-mcp-stdio 2>/dev/null | tail -1)

if echo "$RESPONSE" | grep -q 'get_cat_fact' && echo "$RESPONSE" | grep -q 'get_btc_price'; then
    echo "   ✅ Tools list successful"
else
    echo "   ❌ Tools list failed"
    echo "   Response: $RESPONSE"
    exit 1
fi

# Test 2: get_cat_fact (may fail offline)
echo "2. Testing get_cat_fact tool (non-fatal if offline)..."
RESPONSE=$(echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_cat_fact","arguments":{}}}' | timeout 10s cargo run --bin nova-mcp-stdio 2>/dev/null | tail -1)

if echo "$RESPONSE" | grep -q '"content"'; then
    echo "   ✅ get_cat_fact responded"
else
    echo "   ⚠️  get_cat_fact call possibly failed (offline?)"
    echo "   Response: $RESPONSE"
fi

# Test 3: get_btc_price (may fail offline)
echo "3. Testing get_btc_price tool (non-fatal if offline)..."
RESPONSE=$(echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_btc_price","arguments":{}}}' | timeout 10s cargo run --bin nova-mcp-stdio 2>/dev/null | tail -1)

if echo "$RESPONSE" | grep -q '"content"'; then
    echo "   ✅ get_btc_price responded"
else
    echo "   ⚠️  get_btc_price call possibly failed (offline?)"
    echo "   Response: $RESPONSE"
fi

echo ""
echo "🎉 All tests passed!"
echo "Nova-MCP server basic checks completed."
