![LibreSpeed Logo](https://github.com/librespeed/speedtest/blob/master/.logo/logo3.png?raw=true)

# speedtest-rust

No Flash, No Java, No WebSocket, No Bullshit.

## Try it
[Take a speed test](https://librespeed-rs.ir)

## Compatibility
Compatible with all librespeed clients :

- Web Client
- [Command line client](https://github.com/librespeed/speedtest-cli)
- [Android client](https://github.com/librespeed/speedtest-android)
- [Desktop client](https://github.com/librespeed/speedtest-desktop)

## Attributes
- Memory safety (uses `#![forbid(unsafe_code)]` to ensure everything is implemented in 100% safe Rust.)
- Lightweight & Low overhead
- Low level networking
- Based on tokio-rs (asynchronous)
- No database, telemetry or statistics: nothing is stored

## Features
- Download
- Upload
- Ping
- Jitter
- IP Address, ISP (ISP lookup requires an [ipinfo.io](https://ipinfo.io) API key)

## Run with Podman
Linux only, `amd64` (x86_64) and `arm64` (aarch64) are supported.

```sh
podman build -f Dockerfile.alpine -t speedtest-rust .
podman run --rm -p 8080:8080 speedtest-rust
```

Then open <http://localhost:8080>. To use your own configuration:

```sh
podman run --rm -p 8080:8080 -v ./configs.toml:/usr/local/bin/configs.toml:ro,Z speedtest-rust
```

To build for another architecture:

```sh
podman build --platform linux/arm64 -f Dockerfile.alpine -t speedtest-rust:arm64 .
```

## Build from source
```sh
cargo build --release
./target/release/librespeed-rs
```

## Configuration
Settings are read from `configs.toml` (or the file given with `-c`) and can be overridden with command line flags (see `--help`):

| Setting | Description |
|---|---|
| `bind_address`, `listen_port` | Address and port to listen on |
| `worker_threads` | Number of worker threads, or `"auto"` |
| `base_url` | Prefix of the API routes (`/{base_url}/empty`, `garbage`, `getIP`) |
| `ipinfo_api_key` | Optional ipinfo.io API key to show ISP information |
| `assets_path` | Directory of the web frontend; empty serves the embedded one |
| `enable_tls`, `tls_cert_file`, `tls_key_file` | Serve over TLS |

## Note :
This project can be much better.\
Therefore, your PRs are accepted to improve and solve problems

## License
Copyright (C) 2016-2024 Federico Dossena\
Copyright (C) 2024-2026 Sudo Dios

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Lesser General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU Lesser General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/lgpl>.
