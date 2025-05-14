# badge-proxy

A simple HTTP server that stores a URL and proxies requests to it. It's designed specifically for proxying badge images from shields.io or similar services.

## Features

- Store a URL that can be updated via POST requests
- Proxy requests to the stored URL when users access the root path (`/`)
- Return badge images directly to the client without redirecting
- Optional password protection for URL updates
- Configurable via environment variables

## API Endpoints

- `GET /`: Proxies the request to the stored URL and returns the content directly
- `POST /` or `POST /url`: Updates the stored URL (requires authentication if password is set)

## Environment Variables

- `DEFAULT_URL`: Optional default URL to use on startup
- `URL_UPDATE_PASSWORD`: Optional password for updating the URL

## Running Locally

### Using Cargo

```bash
# Build the project
cargo build --release

# Run the server
URL_UPDATE_PASSWORD=your_password DEFAULT_URL=https://example.com ./target/release/badge-proxy
```

### Using Docker

```bash
# Build the Docker image
docker build -t badge-proxy .

# Run the container
docker run -p 3000:3000 -e URL_UPDATE_PASSWORD=your_password -e DEFAULT_URL=https://example.com badge-proxy
```

## Using the GitHub Packages Container

This project is automatically built and published to GitHub Packages. You can use the pre-built container image:

```bash
# Pull the image
docker pull ghcr.io/OWNER/badge-proxy:latest

# Run the container
docker run -p 3000:3000 -e URL_UPDATE_PASSWORD=your_password -e DEFAULT_URL=https://example.com ghcr.io/OWNER/badge-proxy:latest
```

Replace `OWNER` with your GitHub username or organization name.

## Usage Examples

### Setting a URL

```bash
# Without password protection
curl -X POST -d "https://example.com/badge.svg" http://localhost:3000/

# With password protection
curl -X POST -H "Authorization: Bearer your_password" -d "https://example.com/badge.svg" http://localhost:3000/
```

### Accessing the URL

```bash
# This will fetch the badge image from the stored URL and return it directly
curl http://localhost:3000/

# You can use this URL directly in HTML or Markdown
# <img src="http://localhost:3000/" alt="Badge">
# ![Badge](http://localhost:3000/)
```

## GitHub Actions Workflow

This repository includes a GitHub Actions workflow that:

1. Builds the Rust application
2. Packages it in an Alpine container
3. Publishes it to GitHub Packages

The workflow runs on pushes to the main branch and when tags are created.