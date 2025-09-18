# Nova-MCP Documentation

## Overview

- Purpose: Minimal MCP server exposing GeckoTerminal-powered DEX tools via JSON-RPC for OpenAI Responses’ MCP tool.
- Transports: `stdio` (default) and `http` JSON-RPC.
- Auth: Optional API key required for HTTP, disabled for stdio.
- Extras: Simple plugin registry with enablement per user/group (dev/demo), sled-backed state, basic rate limiting.

## Multi-Tenant Tool Platform Upgrade

Nova-MCP is evolving from a single-tenant demo into a context-aware, multi-tenant tool platform. The key goals of this upgrade are to let Telegram users and group chats own their own tools, surface only the tools that match the current caller, and maintain backward compatibility with the existing built-in GeckoTerminal tools and legacy plug-in APIs. The sections below capture the requirements and system changes that drive this transformation.

### 1. Introduction & Scope

- Built-in tools remain defined in Rust via `Tool { name, description, input_schema }` and are assembled in `NovaServer::get_tools()`.
- Users or groups will be able to register their own tools. Registered tools must only be returned to the AI when the requester’s context matches the owning context.
- A single API key still authenticates HTTP requests. Identity is now derived from new context headers so that the same key can serve multiple tenants.

### 2. Current Architecture Snapshot

- Built-in tools are hard-coded Rust structs.
- The existing plug-in registry allows `/plugins/register` followed by `PluginEnableRequest` per user or group (Telegram IDs). Enablement uses sled-backed state.
- Authentication currently relies on only a global API key. The upgrade must preserve existing behaviour while adding multi-tenant awareness.

### 3. Revised Requirements

- **Context-aware tools:** Every tool is owned by `(context_type, context_id)` where `context_type ∈ {User, Group}` and `context_id` is the Telegram identifier (negative for groups).
- **Single API key:** HTTP callers still provide one shared key but must include `x-nova-context-type` and `x-nova-context-id` headers. JSON-RPC over stdio accepts optional `context_type` and `context_id` fields.
- **Web dashboard:** A browser UI will manage schemas, registrations, updates, and enablement workflows for both individuals and groups.
- **Backward compatibility:** Built-in tools and the legacy plug-in APIs continue to function.
- **Security & isolation:** Nova validates request payloads against declared schemas and limits tool invocation to enabled contexts.

### 4. Data Model Changes

#### 4.1 Tool Definition (Revised)

`PluginRegistrationRequest` gains additional metadata so Nova can invoke third-party endpoints while injecting the caller context itself:

| Field | Type | Description |
| --- | --- | --- |
| `input_schema` | `serde_json::Value` | JSON schema describing tool arguments. |
| `output_schema` | `Option<serde_json::Value>` | Optional JSON schema for the returned payload. |
| `version` | `u32` | Tool definition version. |
| `endpoint_url` | `String` | HTTPS endpoint Nova will call. Nova includes caller context and arguments in the request body. |

Historically plug-in authors provided `context_type` and `context_id` during registration. The upgrade removes that requirement—Nova now injects the caller context at runtime. An internal `owner_id` can still represent the third-party account separate from Telegram identifiers.

#### 4.2 Tool Namespacing

Nova derives a fully qualified name (FQN) to avoid collisions across users and groups:

```rust
if context_type == User {
    format!("user_{}_{}_v{}", context_id, name, version)
} else {
    format!("group_{}_{}_v{}", context_id, name, version)
}
```

The MCP `tools/list` response returns this FQN in `Tool.name`, while descriptions can present user-friendly information.

### 5. API & Protocol Changes

#### 5.1 Context Identification

- **Headers:** Every HTTP request must include `x-api-key`, `x-nova-context-type` (`user` or `group`), and `x-nova-context-id` (Telegram ID). Missing or invalid context yields an auth error.
- **JSON-RPC:** Stdio requests may include `context_type`/`context_id` at the root of the payload.

#### 5.2 Tool Registration

- New endpoint `POST /tools/register` accepts the fields from §4.1.
- The server validates schemas, ensures the context matches the caller, prevents name collisions, persists metadata (including `endpoint_url`), auto-enables the tool for its owner, and returns `{ plugin_id, fq_name, version }`.
- Updates via `PUT /tools/:plugin_id` bump the version; older versions remain callable for compatibility.

#### 5.3 Tool Listing

- `tools/list` extracts the caller context, loads built-in tools, queries plug-ins matching `(context_type, context_id)`, and returns the combined list with FQNs.

#### 5.4 Tool Invocation

1. Parse the tool name. Built-ins continue to dispatch via existing handlers.
2. For plug-ins, parse the FQN back into `(context_type, context_id, base_name, version)`.
3. Ensure the caller context matches and the tool is enabled via `PluginEnableRequest` rules.
4. Validate arguments against `input_schema`.
5. Invoke `endpoint_url` with a `PluginInvocationRequest` containing context and arguments.
6. Optionally validate responses against `output_schema`.

#### 5.5 Legacy Plug-in Endpoints

Existing `/plugins/*` routes remain for internal administration but `/tools/*` will be the primary surface for multi-tenant scenarios.

### 6. Server Implementation Changes

- **Auth middleware (`auth.rs`):** Require the new headers, validate context types, ensure IDs are numeric (negative for groups), and store the resolved context in request scope.
- **Plug-in manager:** Extend `PluginMetadata` with context ownership, schemas, versioning, and `endpoint_url`. Add helpers such as `list_plugins_for_context` and `get_plugin_by_fq_name`, and automatically enable the owning context on registration.
- **MCP handlers:** `NovaServer::get_tools()` now receives context, merges built-in and context-specific tools, and `handle_tool_call()` parses FQNs, enforces context checks, and routes to plug-in invocation logic with clear error messages.

### 7. Web Dashboard

- **Authentication:** UI collects the API key and context (user or group) and sends them with every request. Users can switch contexts from the dashboard.
- **Tool creation workflow:** Select context, define tool metadata, build schemas with validation, provide an HTTPS `endpoint_url`, register via `/tools/register`, and optionally enable the tool for other contexts. Successful registration displays the FQN.
- **Tool management:** Show tools for the active context (base name, version, status, created at). Allow enable/disable, cross-context enablement, edits (create new versions), deletion, and optional invocation logs.
- **Implementation notes:** React SPA or standalone app backed by Nova API, environment-configurable base URLs, JSON editors for advanced schema editing, and version bumping when editing.

### 8. Security & Governance

- **Rate limiting:** Enforce per-context sliding window limits for registrations and invocations since all clients share one API key.
- **Schema sanitisation:** Accept only valid, recognised JSON Schema keywords to avoid expensive validation or malicious payloads.
- **Ownership enforcement:** Only the owner context may update/delete tools; group admin policies can extend permissions.
- **Audit logs:** Track registration, edits, enablement, timestamps, context IDs, and IPs with admin visibility.
- **Future extensibility:** Preserve the user/group distinction but leave room for additional context types (e.g., organisations).

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

