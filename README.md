# Nova-MCP: Minimal MCP Server for OpenAI Responses

Nova-MCP is a minimal Model Context Protocol (MCP) server designed to work with OpenAI's built-in MCP tool via the Responses API. It exposes GeckoTerminal tools that call public APIs (no keys required) and returns results through MCP-compliant JSON-RPC.

## Features

### ✅ Example Tools (No API Keys)
- get_gecko_networks: List available networks from GeckoTerminal
- get_gecko_token: Fetch token info from GeckoTerminal
- get_gecko_pool: Fetch pool info from GeckoTerminal
- get_trending_pools: Fetch trending DEX pools from GeckoTerminal
- search_pools: Search DEX pools on GeckoTerminal
- get_new_pools: Fetch newest DEX pools from GeckoTerminal

## Quick Start

### Prerequisites
- Rust 1.70+ 
- Tokio async runtime

### Installation

1. Clone the repository:
```bash
git clone https://github.com/your-org/nova-mcp.git
cd nova-mcp
```

2. Build the project:
```bash
cargo build --release
```

3. Run the server on stdio (default):
```bash
cargo run --bin nova-mcp-stdio
```

4. Optional: Run HTTP JSON-RPC on port 8080:
```bash
export NOVA_MCP_TRANSPORT=http
export NOVA_MCP_PORT=8080
# Optional: enable API key auth for HTTP
export NOVA_MCP_AUTH_ENABLED=true
export NOVA_MCP_API_KEYS=devkey123
cargo run --bin nova-mcp-stdio
```

### Docker

Build the image and run the server over HTTP on port 8080.

1) Docker CLI
```bash
# From repo root
docker build -t nova-mcp -f docker/Dockerfile .

# Run with HTTP JSON-RPC on port 8080
docker run --rm -it \
  -p 8080:8080 \
  -e NOVA_MCP_TRANSPORT=http \
  -e NOVA_MCP_PORT=8080 \
  --name nova-mcp \
  nova-mcp

# Verify
curl -s -X POST http://localhost:8080/rpc \
  -H 'Content-Type: application/json' \
  -H 'x-api-key: devkey123' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

2) Docker Compose
```bash
# From repo root
# Start services (build + start in background)
docker compose -f docker/docker-compose.yml up --build -d

# Check logs
docker compose -f docker/docker-compose.yml logs -f

# Check service status
docker compose -f docker/docker-compose.yml ps

# Test tools with curl
echo "Testing tools/list..."
curl -s -X POST http://localhost:8080/rpc \
  -H 'Content-Type: application/json' \
  -H 'x-api-key: devkey123' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'

echo -e "\n\nTesting get_gecko_networks..."
curl -s -X POST http://localhost:8080/rpc \
  -H 'Content-Type: application/json' \
  -H 'x-api-key: devkey123' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_gecko_networks","arguments":{}}}'

# Stop and remove containers
docker compose -f docker/docker-compose.yml down

# Stop and remove containers, networks, and volumes
docker compose -f docker/docker-compose.yml down -v
```

### Configuration

Nova-MCP can be configured via environment variables:

```bash
# Server configuration
export NOVA_MCP_PORT=8080
export NOVA_MCP_LOG_LEVEL=info
export NOVA_MCP_TRANSPORT=stdio   # or "http"
export NOVA_MCP_AUTH_ENABLED=false # true to require x-api-key on HTTP
export NOVA_MCP_API_KEYS="key1,key2" # allowed API keys (HTTP)
export NOVA_MCP_AUTH_HEADER=x-api-key # override header name if needed

# API keys (optional)
export UNISWAP_API_KEY=your_uniswap_key
export COINGECKO_API_KEY=your_coingecko_key
export DEXSCREENER_API_KEY=your_dexscreener_key
export GECKO_TERMINAL_BASE_URL=https://api.geckoterminal.com/api/v2
```

Or create a `config.toml` file:

```toml
[server]
port = 8080
log_level = "info"
transport = "stdio"  # or "http"

[apis]
uniswap_api_key = "your_key_here"
coingecko_api_key = "your_key_here"
dexscreener_api_key = "your_key_here"
rate_limit_per_minute = 60

[cache]
ttl_seconds = 300
max_entries = 1000

[auth]
enabled = false
allowed_keys = []
header_name = "x-api-key"
```

## Use with OpenAI Responses (MCP Tool)

Two common integration patterns:

1) stdio “command” mode (recommended for local dev):
   - Configure the OpenAI client’s MCP tool to run the command:
     `cargo run --bin nova-mcp-stdio`

2) HTTP “url” mode:
   - Start server with `NOVA_MCP_TRANSPORT=http` and provide the URL: `http://localhost:8080/rpc`

Example tool calls (JSON-RPC):

List tools:
```json
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
```

Call get_gecko_networks:
```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_gecko_networks","arguments":{}}}
```

## Testing

- Run unit and integration tests: `cargo test --all`
- Run network-dependent tests (ignored by default): `cargo test -- --ignored`

The ignored tests hit real public APIs and may be flaky or rate-limited; they are executed separately in CI with `continue-on-error`.

```rust
use open_ai_rust_responses_by_sshift::{Client, Request, Model, Tool};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;

    // Define the Nova-MCP server tool
    let nova_mcp_tool = Tool::mcp(
        "nova-mcp",
        "http://localhost:8080", // Or your server URL
        None, // No auth headers needed for local server
    );

    // Use in your request
    let request = Request::builder()
        .model(Model::GPT4oMini)
        .input("Search for WETH pools on Uniswap with minimum $1M liquidity")
        .tools(vec![nova_mcp_tool])
        .max_output_tokens(500)
        .build();

    let response = client.responses.create(request).await?;
    println!("Response: {}", response.output_text());
    Ok(())
}
```

## Available Tools

- get_gecko_networks
- get_gecko_token
- get_gecko_pool
- get_trending_pools
- search_pools
- get_new_pools

## Architecture

```
nova-mcp/
├── src/
│   ├── main.rs               # Server entry point (stdio/http)
│   ├── server.rs             # Server core (tools registry, state)
│   ├── mcp/
│   │   ├── dto.rs            # MCP DTOs (requests, tools, responses)
│   │   └── handler.rs        # MCP method handlers (list/call/initialize)
│   ├── http.rs               # HTTP JSON-RPC transport (+auth, health, plugins)
│   ├── auth.rs               # API key auth (dev; replace for prod)
│   ├── tools/
│   │   ├── mod.rs            # Public re-exports for tools
│   │   └── gecko_terminal/
│   │       ├── helpers.rs
│   │       ├── implementation.rs   # Shared HTTP client + base URL
│   │       ├── networks/           # get_gecko_networks
│   │       │   ├── dto.rs
│   │       │   └── handler.rs
│   │       ├── token/              # get_gecko_token
│   │       │   ├── dto.rs
│   │       │   └── handler.rs
│   │       ├── pool/               # get_gecko_pool
│   │       │   ├── dto.rs
│   │       │   └── handler.rs
│   │       ├── trending_pools/     # get_trending_pools
│   │       │   ├── dto.rs
│   │       │   ├── handler.rs
│   │       │   └── implementation.rs
│   │       ├── search_pools/       # search_pools
│   │       │   ├── dto.rs
│   │       │   ├── handler.rs
│   │       │   └── implementation.rs
│   │       └── new_pools/          # get_new_pools
│   │           ├── dto.rs
│   │           ├── handler.rs
│   │           └── implementation.rs
│   └── config.rs             # Configuration management
```

## Development

### Building
```bash
cargo build
```

### Running Tests
```bash
cargo test

# Optional: run live API integration tests
cargo test -- --ignored
```

### Running with Debug Logging
```bash
RUST_LOG=debug cargo run
```

## Docker Commands

- Build image:
  - `docker build -t nova-mcp -f docker/Dockerfile .`
- Run with Compose (build + start in background):
  - `docker compose -f docker/docker-compose.yml up --build -d`
- View logs:
  - `docker compose -f docker/docker-compose.yml logs -f`
- Check service status:
  - `docker compose -f docker/docker-compose.yml ps`
- Stop and remove containers:
  - `docker compose -f docker/docker-compose.yml down`
- Stop and remove containers, networks, and volumes:
  - `docker compose -f docker/docker-compose.yml down -v`

### Adding New Tools

1) Create a module under `src/tools/gecko_terminal/your_tool/` with:
   - `dto.rs` for input/output structs
   - `handler.rs` for the public async function used by MCP
   - `implementation.rs` if it needs its own HTTP logic (or reuse `implementation.rs` at the parent)

2) Re-export in `src/tools/gecko_terminal/mod.rs` and, if you want top-level access, in `src/tools/mod.rs`.

3) Register the tool in `src/server.rs:get_tools()` with its JSON schema.

4) Route the call in `src/mcp/handler.rs:handle_tool_call()` by name, deserializing the input.

5) Add tests under `tests/` or a module test alongside the code.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Support

For issues and questions:
- Create an issue on GitHub
- Check the documentation
- Review the example implementations

## Roadmap

- [ ] Additional public example tools
- [ ] SSE/WebSocket transport for streaming
- [ ] Input validation improvements
- [ ] Caching and retries for flaky APIs
