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

## Build and Run
### Run in compose
```yml
services:
  simple-proxy:
    image: kaiv-dev/simple_proxy:latest
    ports:
      - "443:443"
    volumes:
      - "./certs:/app/certs"
      - "./proxy.toml:/app/proxy.toml"
    env_file:
      - .cfg
```

### From source
Install Rust and the required toolchain, then:

```sh
git clone https://github.com/kaiv-dev/simple_proxy.git
cd simple_proxy
cargo build --release

# Copy binary
cp ./target/release/simple_proxy .

# Run
./simple-proxy
```

---
# Configuration
## .cfg 
Default values if unset:
```sh
CERT_PATH="./certs"                                 # Folder containing fullchain.pem and privkey.pem
CONFIG_PATH="./proxy.toml"                          # Path to proxy config
LISTEN_ADDR="0.0.0.0:443"                           # Proxy listen address
HTTPS="false"                                       # If false, CERT_PATH will be ignored
# Pingora on windows instantly begin graceful shutdown on start, so it set to u64::MAX as default
GRACE_PERIOD="18446744073709551615"                 # Grace period in seconds
GRACEFUL_SHUTDOWN_TIMEOUT="18446744073709551615"    # Graceful shutdown timeout in seconds
```
## proxy.toml
List of services. Current version has http and dir entries. 
If request will not match any of rules - 404 Not Found will be returned.
If upstream is unaccessible - 502 Bad Gateway will be returned.
Order of same domain rules matters!
```toml
# Serves static files from a directory.
# Only direct file paths are accessible (no directory listing).
# If file not found → 404 is returned.
# Directory rules are matched before HTTP rules:
# If domain + route matches a dir, it will serve files instead of proxying.
[[dir]]
domain = "files.example.com"
route  = "/files"                 # Route prefix to match
listen = "127.0.0.1:4000"         # Local address for Axum file service
path   = "/static"                # Absolute or relative path to serve

# Proxies HTTP requests for the given domain to an upstream server.
[[http]]
domain = "app.example.com"
routes = ["/abc"]                 # Optional, redirect only if route is match. If unset - everything will be redirected.
strip_route = false               # Optional, default = false
https  = false                    # Optional, default = false (⚠️ experimental, untested)
upstream = "127.0.0.1:1"
proxy_ports_from_prefix = [3000]  # Optional list of ports to forward from first entry of path from route
                                  # For example, app.example.com/3000/abc?q=v will be redirected to 127.0.0.1:3000/abc?q=v
```