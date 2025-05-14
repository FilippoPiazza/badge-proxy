# Build stage
FROM rust:1.82-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev pkgconfig openssl-libs-static

# Create a new empty project
WORKDIR /app
COPY . .

# Remove Cargo.lock to avoid version compatibility issues
RUN rm -f Cargo.lock

# Build the application with release optimizations
RUN cargo build --release

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Copy the binary from the build stage
COPY --from=builder /app/target/release/badge-proxy /usr/local/bin/

# Expose the port the server listens on
EXPOSE 3000

# Define environment variables that should be provided at runtime
# These are intentionally left empty and should be provided when running the container
# Example: docker run -e URL_UPDATE_PASSWORD="your_password" -e DEFAULT_URL="https://example.com" badge-proxy
ENV URL_UPDATE_PASSWORD=""
ENV DEFAULT_URL=""

# Run the binary
CMD ["badge-proxy"]