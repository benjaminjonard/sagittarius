# Build stage
FROM rust:1.92-slim AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build the real application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/sagittarius-server /usr/local/bin/sagittarius

# Create directory for database
RUN mkdir -p /data

# Expose port
EXPOSE 3000

# Volume for database persistence
VOLUME ["/data"]

# Run the application
CMD ["sagittarius"]