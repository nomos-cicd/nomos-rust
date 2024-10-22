# Stage 1: Build dependencies
FROM rust:1.75 as deps-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Create dummy lib.rs and main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {println!(\"dummy\");}" > src/main.rs && \
    echo "pub fn dummy() {println!(\"dummy\");}" > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Stage 2: Build the application
FROM rust:1.75 as app-builder
WORKDIR /app
# Copy the dependencies build artifacts
COPY --from=deps-builder /app/target target
COPY --from=deps-builder /app/Cargo.toml /app/Cargo.lock ./
# Copy the actual source code
COPY src src
# Build the application
RUN cargo build --release

# Stage 3: Create the final image
FROM docker:25.0.5-dind

# Install any necessary runtime dependencies
RUN apk add --no-cache \
    openssl \
    ca-certificates \
    tzdata

# Create a non-root user
RUN addgroup -S appgroup && adduser -S appuser -G appgroup

# Set working directory
WORKDIR /app

# Copy the binary from builder
COPY --from=app-builder /app/target/release/nomos-rust .

# Change ownership of the binary
RUN chown -R appuser:appgroup /app

# Switch to non-root user
USER appuser

# Set the entrypoint
ENTRYPOINT ["./nomos-rust"]
