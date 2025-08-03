# CrustyBallz

A blazing-fast, highly optimized backend implementation of [agar.io](https://agar.io) written in Rust. Designed for minimal compute usage and maximum fidelity to the original game, CrustyBallz leverages modern protocols and efficient data structures to deliver a seamless, real-time multiplayer experience.

---

## 🚀 Features

- **High Performance:** Utilizes spatial partitioning (QuadTrees) and binary packet serialization for ultra-low latency and efficient compute usage.
- **Modern Protocols:** Built on QUIC (via WebTransport) for fast, reliable, and secure real-time communication.
- **Real-Time Multiplayer:** Handles hundreds of concurrent players with smooth gameplay and accurate collision detection.
- **Faithful to Original:** Closely mimics the original agar.io mechanics, including splitting, merging, viruses, and mass food.
- **Optimized Networking:** Binary packets and async networking for minimal overhead and maximum throughput.

---

## 🛠️ Technology Stack

| Technology         | Purpose                                                      |
|-------------------|--------------------------------------------------------------|
| **Rust**          | Systems programming language for safety and performance       |
| **Tokio**         | Asynchronous runtime for fast, non-blocking IO                |
| **Axum**          | Web framework for HTTP server and routing                     |
| **Tower / Tower HTTP** | Middleware (CORS, compression, etc.)                    |
| **QUIC / WebTransport** | Modern transport protocol for low-latency networking   |
| **wtransport**    | QUIC/WebTransport implementation in Rust                      |
| **Socket.IO**     | Real-time communication (via `socketioxide`, `rust_socketio`) |
| **Serde**         | Serialization/deserialization of packets and data             |
| **Rustls**        | TLS encryption for secure connections                         |
| **dotenv**        | Environment variable management                               |
| **fern, log, tracing** | Structured logging and diagnostics                      |
| **uuid, rand, chrono, clap** | Utilities for unique IDs, randomness, time, CLI   |

---

## 📦 Getting Started

### Prerequisites
- Rust (edition 2021)
- [QUIC/WebTransport-compatible client](https://developer.chrome.com/articles/webtransport/)

### Build & Run
```bash
# Clone the repository
$ git clone https://github.com/yourusername/crustyballz.git
$ cd crustyballz

# Set up environment variables (see .env.example)
$ cp .env.example .env

# Build and run
$ cargo run --release
```

### Configuration
- Edit `config.rs` or use environment variables to tweak server settings.
- TLS certificates are required for QUIC/WebTransport in production (see `axum-server` + `rustls`).

---

## 📐 Architecture Highlights
- **QuadTree**: Efficient spatial partitioning for collision and visibility checks.
- **Binary Packets**: Custom serialization for minimal bandwidth usage.
- **Async Everything**: All networking and game logic is fully asynchronous.
- **Modular Managers**: Separate modules for food, viruses, players, and more.

---

## 🤝 Contributing
Pull requests and issues are welcome! Please open an issue to discuss major changes.

## 📄 License
MIT License. See [LICENSE](LICENSE) for details.

---

> Made with Rust — by Neoseiki & Lucas Campos
