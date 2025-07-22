# Plasmic Image Optimizer 🚀

[![CI/CD](https://github.com/fgribreau/plasmic-img-optimizer/workflows/CI%2FCD/badge.svg)](https://github.com/fgribreau/plasmic-img-optimizer/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Deploy to Cloudflare Workers](https://img.shields.io/badge/Deploy%20to-Cloudflare%20Workers-f38020?logo=cloudflare&logoColor=white)](https://workers.cloudflare.com)

A high-performance, API-compatible Rust implementation of the img.plasmic.app image optimization service. Designed for blazing-fast image processing with automatic caching, format conversion, and quality optimization.

## ✨ Features

- 🖼️ **Smart Image Processing**
  - Resize images with configurable width (up to 3840px)
  - Automatic format conversion (JPEG, PNG, WebP)
  - Quality optimization (1-100, default 75)
  - SVG pass-through with redirects
  
- ⚡ **Performance**
  - Written in Rust for maximum performance
  - Zero-copy streaming for memory efficiency
  - File-based caching system
  - Async/await with Tokio runtime
  
- 🌐 **Deployment Options**
  - Run as standalone HTTP server
  - Deploy to Cloudflare Workers
  - Docker container support
  - Automatic CI/CD with GitHub Actions

- 🛡️ **Production Ready**
  - Comprehensive error handling with RFC7807 Problem Details
  - CORS support for cross-origin requests
  - Health check endpoint
  - Structured logging

## 🚀 Quick Start

### Running Locally

```bash
# Clone the repository
git clone https://github.com/fgribreau/plasmic-img-optimizer.git
cd plasmic-img-optimizer

# Run the server (default port: 3000)
cargo run --release

# Or specify a custom port
PORT=8080 cargo run --release
```

### Using Docker

```bash
# Build the Docker image
docker build -t plasmic-img-optimizer .

# Run the container
docker run -p 3000:3000 plasmic-img-optimizer
```

### API Usage

```bash
# Basic image optimization
curl "http://localhost:3000/img-optimizer/v1/img?src=https://example.com/image.jpg"

# Resize to specific width
curl "http://localhost:3000/img-optimizer/v1/img?src=https://example.com/image.jpg&w=800"

# Convert format and adjust quality
curl "http://localhost:3000/img-optimizer/v1/img?src=https://example.com/image.png&f=webp&q=85"

# Health check
curl "http://localhost:3000/health"

# List all possible errors
curl "http://localhost:3000/errors"
```

## 📚 API Reference

### Endpoints

#### `GET /img-optimizer/v1/img`

Optimize and transform images on-the-fly.

**Query Parameters:**
- `src` (required): Source image URL
- `w` (optional): Target width in pixels (1-3840)
- `q` (optional): Quality (1-100, default: 75)
- `f` (optional): Output format (`jpeg`, `jpg`, `png`, `webp`)

**Example:**
```
/img-optimizer/v1/img?src=https://example.com/photo.jpg&w=1200&q=90&f=webp
```

#### `GET /health`

Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "service": "img-optimizer"
}
```

#### `GET /errors`

List all possible error codes and descriptions.

**Response:**
```json
{
  "errors": [
    "IMG_001: Invalid image URL - The provided URL is not valid",
    "VAL_001: Invalid width - Width must be between 1 and 3840, got {width}",
    ...
  ],
  "total": 11
}
```

### Error Handling

All errors follow RFC7807 Problem Details standard:

```json
{
  "type": "https://github.com/fgribreau/plasmic-img-optimizer/errors/IMG_001",
  "title": "Bad Request",
  "status": 400,
  "detail": "IMG_001: Invalid image URL - The provided URL is not valid",
  "errorCode": "IMG_001",
  "howToFix": "Provide a valid URL starting with http:// or https://",
  "moreInfo": "https://github.com/fgribreau/plasmic-img-optimizer#error-img_001"
}
```

## 🚢 Deployment

### Cloudflare Workers

This service can be deployed to Cloudflare Workers for global edge deployment.

#### Prerequisites

1. Create a Cloudflare account and get your API token
2. Set up environment variables:

```bash
# Add to your GitHub repository secrets
CLOUDFLARE_API_TOKEN=your_cloudflare_api_token
```

#### Configuration

1. Update `wrangler.toml` with your settings:

```toml
[env.production]
name = "your-worker-name"
route = { pattern = "img-optimizer.yourdomain.com/*", zone_name = "yourdomain.com" }

[[kv_namespaces]]
binding = "IMAGE_CACHE"
id = "your_kv_namespace_id"
```

2. Create KV namespace:

```bash
wrangler kv:namespace create "IMAGE_CACHE"
```

#### Automatic Deployment

Push to the `main` branch to trigger automatic deployment via GitHub Actions:

```bash
git push origin main
```

The CI/CD pipeline will:
1. Run tests
2. Check code formatting and linting
3. Build for WebAssembly
4. Deploy to Cloudflare Workers

#### Manual Deployment

```bash
# Install wrangler
npm install -g wrangler

# Build and deploy
wrangler deploy --env production
```

## 🛠️ Development

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Node.js 18+ (for Cloudflare Workers deployment)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/fgribreau/plasmic-img-optimizer.git
cd plasmic-img-optimizer

# Build debug version
cargo build

# Build release version with optimizations
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=info cargo run
```

### Project Structure

```
plasmic-img-optimizer/
├── src/
│   ├── main.rs           # Entry point for native binary
│   ├── lib.rs            # Core library with shared logic
│   ├── error.rs          # Unified error handling
│   ├── image_processor.rs # Image processing logic
│   ├── cache.rs          # Caching implementation
│   └── worker.rs         # Cloudflare Workers entry point
├── tests/
│   └── integration_tests.rs # Comprehensive test suite
├── .github/
│   └── workflows/
│       └── ci-cd.yml     # GitHub Actions workflow
├── Cargo.toml            # Rust dependencies
├── wrangler.toml         # Cloudflare Workers config
└── README.md
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_image_optimization

```


## 🔧 Configuration

### Environment Variables

- `PORT`: HTTP server port (default: 3000)
- `RUST_LOG`: Log level (`error`, `warn`, `info`, `debug`, `trace`)
- `CACHE_TTL`: Cache time-to-live in seconds (Workers only, default: 86400)

### Cache Configuration

The service uses file-based caching for the native version and KV storage for Cloudflare Workers. Cache keys are generated using SHA256 hash of:
- Source URL
- Width parameter
- Quality parameter
- Format parameter

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Write tests for new features
- Follow Rust conventions and idioms
- Run `cargo fmt` and `cargo clippy` before committing
- Update documentation as needed

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by [img.plasmic.app](https://img.plasmic.app)
- Built with [Actix Web](https://actix.rs/) and [image-rs](https://github.com/image-rs/image)
- Deployed on [Cloudflare Workers](https://workers.cloudflare.com)

## 📞 Support

- 🐛 [Report bugs](https://github.com/fgribreau/plasmic-img-optimizer/issues)
- 💡 [Request features](https://github.com/fgribreau/plasmic-img-optimizer/issues)
- 📖 [Read the docs](#api-reference)
- ⭐ Star this repo if you find it useful!

---

Made with ❤️ by [@FGRibreau](https://github.com/fgribreau)