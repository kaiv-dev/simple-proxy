# Simple Proxy
A lightweight reverse proxy and static file server written in Rust.
> 🚧 It is in development, so config structure and api may change 🚧

---

## Features
- Serve static files from a directory.
- Proxy HTTP requests to upstream servers.
- Simple configuration using `toml`.
- Optional HTTPS support (TLS certificates).

---

Current configuration structure:

## Build and Run
Install Rust and the required toolchain, then:

```sh
git clone https://github.com/kaiva-morphin/simple-proxy.git
cd simple-proxy
cargo build --release

# Copy binary
cp ./target/release/simple-proxy .

# Run
./simple-proxy
```

---
# Configuration
## .cfg 
Default values if unset:
```sh
CERT_PATH="./certs"        # Folder containing fullchain.pem and privkey.pem
CONFIG_PATH="./proxy.toml" # Path to proxy config
LISTEN_ADDR="0.0.0.0:443"  # Proxy listen address
HTTPS="false"              # If false, CERT_PATH will be ignored
```
## proxy.toml
List of services. Current version has http and dir entries. 
If request will not match any of rules - 404 Not Found will be returned.
If upstream is unaccessible - 502 Bad Gateway will be returned.
```toml

# dir will serve files in dir
# It will not show dir at /, only direct file paths.
# If file not found - 404 will be returned
# dirs are matching before http, so if domain is same with http record and beginning of route match dir - dir will be served instead of proxying to http upstream even if file will not found
[[dir]]
domain = "files.example.com"
route  = "/files"                 # Route prefix to match
listen = "127.0.0.1:4000"         # Local address for Axum file service
path   = "/static"                # Absolute or relative path to serve



# http will proxy requests with domain in header to upstream.
[[http]]
domain = "app.example.com"
https  = false                    # Optional, default = false (⚠️ experimental)
upstream = "127.0.0.1:3000"
proxy_ports_from_prefix = [3000]  # Optional list of ports to forward
```