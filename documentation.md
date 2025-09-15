# Nova-MCP Documentation

## Overview

- Purpose: Minimal MCP server exposing GeckoTerminal-powered DEX tools via JSON-RPC for OpenAI Responses’ MCP tool.
- Transports: `stdio` (default) and `http` JSON-RPC.
- Auth: Optional API key required for HTTP, disabled for stdio.
- Extras: Simple plugin registry with enablement per user/group (dev/demo), sled-backed state, basic rate limiting.

## Architecture

```
src/
├── main.rs                 # Entrypoint; selects transport (stdio/http)
├── server.rs               # Server object; tool registry; PluginManager wiring
├── mcp/
│   ├── dto.rs              # JSON-RPC types for MCP
│   └── handler.rs          # Implements initialize, tools/list, tools/call, ping
├── http.rs                 # HTTP transport (/rpc + /plugins/* + health)
├── auth.rs                 # API key header validation
├── config.rs               # Env/TOML-driven config (serde defaulted)
├── plugins/
│   ├── dto.rs              # Plugin metadata + enablement records
│   ├── handler.rs          # REST handlers (register/update/list/invoke/enable)
│   ├── helpers.rs          # Auth + rate limiting integration for plugins
│   └── manager.rs          # In-memory registry + sled-backed enablement
└── tools/
    ├── mod.rs              # Public re-exports for tools
    └── gecko_terminal/
        ├── helpers.rs
        ├── implementation.rs   # Shared reqwest client + base URL
        ├── networks/           # get_gecko_networks
        │   ├── dto.rs
        │   └── handler.rs
        ├── token/              # get_gecko_token
        │   ├── dto.rs
        │   └── handler.rs
        ├── pool/               # get_gecko_pool
        │   ├── dto.rs
        │   └── handler.rs
        ├── trending_pools/     # get_trending_pools
        │   ├── dto.rs
        │   ├── handler.rs
        │   └── implementation.rs
        ├── search_pools/       # search_pools
        │   ├── dto.rs
        │   ├── handler.rs
        │   └── implementation.rs
        └── new_pools/          # get_new_pools
            ├── dto.rs
            ├── handler.rs
            └── implementation.rs
```

## Tools

- get_gecko_networks: Lists available networks.
- get_gecko_token: Returns token info on a network/address.
- get_gecko_pool: Returns pool info on a network/address.
- get_trending_pools: Lists trending pools with pagination and duration.
- search_pools: Searches pools by query, optional network.
- get_new_pools: Lists newest pools with pagination.

Schemas are defined in `src/server.rs:get_tools()` and inputs/outputs live in the module `dto.rs` files.

## MCP JSON-RPC

- initialize: Returns protocol version and server info.
- tools/list: Returns tools with name/description/input_schema.
- tools/call: Executes the tool by name and `arguments` object.

Example request/response for tools/list:

```
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
```

```
{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"get_gecko_networks", ...}]}}
```

## HTTP Transport

- Endpoint: `POST /rpc` with JSON body as `McpRequest`.
- Auth: Header name defaults to `x-api-key` when enabled. Configure header/key(s) via env.
- Health: `GET /healthz` and `GET /readyz`.
- Rate limit: Simple per-key counter with a minute bucket and TTL cleanup.

## Plugin Registry (Dev)

- Register: `POST /plugins/register` -> `PluginMetadata`.
- Update: `PUT /plugins/:plugin_id` -> `PluginMetadata`.
- Unregister: `DELETE /plugins/:plugin_id`.
- List: `GET /plugins` -> `PluginMetadata[]`.
- Enablement: `POST /plugins/enable` -> `PluginEnablementStatus` for user or group.
- Invoke: `POST /plugins/:plugin_id/call` with context and arguments.

Enablement is stored in sled (`user_plugins`, `group_plugins` trees). This is a demonstration scaffold; swap out for your production policy store.

## Configuration

Environment variables:

```
# Server
NOVA_MCP_TRANSPORT=stdio|http
NOVA_MCP_PORT=8080
NOVA_MCP_LOG_LEVEL=info

# HTTP auth
NOVA_MCP_AUTH_ENABLED=true|false
NOVA_MCP_API_KEYS="key1,key2"
NOVA_MCP_AUTH_HEADER=x-api-key

# External APIs
GECKO_TERMINAL_BASE_URL=https://api.geckoterminal.com/api/v2
UNISWAP_API_KEY=...
COINGECKO_API_KEY=...
DEXSCREENER_API_KEY=...

# Limits/cache
# (rate_limit_per_minute, ttl_seconds, etc., when using config file)
```

TOML config (`config.toml`) mirrors `NovaConfig` (see README for example). You can load from file using `NovaConfig::from_file(path)` or rely on `from_env()`.

## Running

- Stdio: `cargo run --bin nova-mcp-stdio`
- HTTP: set `NOVA_MCP_TRANSPORT=http` and `NOVA_MCP_PORT`, then run the same command.
- Docker: see README for full Compose and CLI examples.

## Testing

- Unit/integration: `cargo test`
- Live API tests (ignored): `cargo test -- --ignored`

## Adding a Tool

1. Create a `your_tool/` directory under `src/tools/gecko_terminal/` with `dto.rs`, `handler.rs`, and optional `implementation.rs`.
2. Re-export it in `src/tools/gecko_terminal/mod.rs` (and in `src/tools/mod.rs` if you want top-level re-exports).
3. Register the tool schema in `src/server.rs:get_tools()`.
4. Add a `match` branch in `src/mcp/handler.rs:handle_tool_call()` for the tool name.
5. Add tests under `tests/` and, optionally, live tests under `tests/` with `#[ignore]`.

## Error Handling

- Internal errors are surfaced as `McpError` with code `-32603` in JSON-RPC and appropriate HTTP codes in the HTTP transport and plugin routes.
- Common validation errors return concise messages (e.g., missing required params).

## Security Notes

- HTTP auth uses raw API keys for demo; consider a proper identity layer with hashed secrets and scoped tokens in production.
- Rate limiting is in-memory per-process; use a shared limiter (Redis) for multi-instance deployments.
- Sled storage is local; replace with a managed DB for production needs.

## Troubleshooting

- “Unauthorized” on HTTP: Make sure `NOVA_MCP_AUTH_ENABLED=true` and include the header (`x-api-key: your_key`).
- Rate limit exceeded: Increase `rate_limit_per_minute` or reduce request frequency.
- Live API flakiness: GeckoTerminal is public; expect occasional rate limits or transient errors; retry or add caching if needed.

