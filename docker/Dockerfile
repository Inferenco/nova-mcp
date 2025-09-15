# Nova-MCP Server Dockerfile
FROM rust:1.89-slim AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release --bin nova-mcp-stdio

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false nova

# Create app directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/nova-mcp-stdio ./nova-mcp-stdio

# Change ownership of the entire app directory to nova user
RUN chown -R nova:nova /app
RUN chmod +x nova-mcp-stdio

# Switch to app user
USER nova

# Expose port (for future HTTP transport)
EXPOSE 8080

# Set environment variables
ENV NOVA_MCP_LOG_LEVEL=info
ENV NOVA_MCP_TRANSPORT=http
ENV NOVA_MCP_PORT=8080

# Run the server
CMD ["./nova-mcp-stdio"]
