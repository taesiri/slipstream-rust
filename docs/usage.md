# Usage

This page documents the CLI surface for the Rust client and server binaries.

## slipstream-client

Required flags:

- --domain <DOMAIN>
- --resolver <IP:PORT> (repeatable; at least one required)

Common flags:

- --tcp-listen-port <PORT> (default: 5201)
- --congestion-control <bbr|dcubic> (default: dcubic)
- --gso (currently not implemented in the Rust loop; prints a warning)
- --keep-alive-interval <SECONDS> (default: 400)

Example:

```
./target/release/slipstream-client \
  --tcp-listen-port 7000 \
  --resolver 127.0.0.1:8853 \
  --domain example.com
```

Notes:

- Resolver addresses must all be IPv4 or all be IPv6; mixed families are rejected.
- IPv6 resolvers must be bracketed, for example: [2001:db8::1]:53.

## slipstream-server

Required flags:

- --domain <DOMAIN>

Common flags:

- --dns-listen-port <PORT> (default: 53)
- --dns-listen-ipv6 (default: false)
- --target-address <HOST:PORT> (default: 127.0.0.1:5201)
- --cert <PATH>
- --key <PATH>

Example:

```
./target/release/slipstream-server \
  --dns-listen-port 8853 \
  --target-address 127.0.0.1:5201 \
  --domain example.com \
  --cert ./cert.pem \
  --key ./key.pem
```

## Local testing

For a local smoke test, the Rust to Rust interop script spins up a UDP proxy and TCP echo:

```
./scripts/interop/run_rust_rust.sh
```

See docs/interop.md for full details and C interop variants.
