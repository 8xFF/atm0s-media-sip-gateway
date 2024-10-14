```markdown:README.md
# SIP Gateway for atm0s-media-server

## Overview

This project is a SIP (Session Initiation Protocol) gateway designed for the atm0s media server. It facilitates the handling of SIP calls, including incoming and outgoing call management, media handling, and integration with an address book for phone number synchronization. The gateway is built using Rust and leverages asynchronous programming with the Tokio runtime.

## Features

- **SIP Call Management**: Supports both incoming and outgoing SIP calls.
- **Media Handling**: Integrates with media servers for RTP (Real-time Transport Protocol) handling.
- **Address Book Synchronization**: Syncs phone numbers from a specified source.
- **Secure Context**: Utilizes secure tokens for authentication and authorization.
- **WebSocket Support**: Provides WebSocket endpoints for real-time communication.
- **Incoming Call Handling**: Allows receiving incoming calls with WebSocket.

## Getting Started

### Prerequisites

- Rust (version 1.56 or higher)
- Cargo (Rust package manager)
- A compatible media server

### Installation from docker, prebuilt

TODO

### Installation from source

1. Clone the repository:
2. Build the project:

   ```bash
   cargo build --release
   ```

3. Run the server:

   ```bash
   cargo run --release
   ```

### Configuration

The server can be configured using command-line arguments or environment variables. The following parameters are available:

- `--http-addr`: Address for the HTTP server (default: `0.0.0.0:8008`)
- `--http-public`: Public URL for the HTTP server (default: `http://127.0.0.1:8008`)
- `--sip-addr`: Address for the SIP server (default: `0.0.0.0:5060`)
- `--secret`: Secret for the gateway (default: `insecure`)
- `--phone-numbers-sync`: Address for phone book synchronization (optional)
- `--phone-numbers-sync-interval-ms`: Interval for phone book synchronization in milliseconds (default: `30000`)
- `--http-hook-queues`: Number of HTTP hook queues (default: `20`)
- `--media-gateway`: Address for the media server gateway (required)
- `--media-app-sync`: Address for media server apps synchronization (optional)

### Example Usage

To start the server with custom configurations, you can run:

```bash
cargo run --release -- --http 0.0.0.0:8080 --sip 0.0.0.0:5070 --secret mysecret --media-gateway http://media-server
```

## API Documentation

The project uses the `poem_openapi` crate to provide API documentation. You can access the API documentation at the following endpoint:

```
http://<your-server-address>/docs
```

## Contributing

Contributions are welcome! If you have suggestions for improvements or new features, please open an issue or submit a pull request.

1. Fork the repository.
2. Create a new branch for your feature or bug fix.
3. Make your changes and commit them.
4. Push to your branch and create a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Thanks to the Rust community for their support and contributions.
- Special thanks to the maintainers of the libraries used in this project.

---

Feel free to customize this README further based on your project's specific needs and details!