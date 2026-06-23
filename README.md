# Chronosys

[![License: GPL v2](https://img.shields.io/badge/License-GPL_v2-blue.svg)](https://www.gnu.org/licenses/old-licenses/gpl-2.0.html)

`chronosys` is a light-weight, highly performant Linux System Monitor Recorder that uses async system data ingestion and persistent storage to provide real-time and sub-second historical system metrics. `chronosys` captures both global system and process-specific metrics.

---

## Features

`chronosys` implements multi-tier architecture based on Rust concurrency and async primatives to ensure data is captured without blocking.

1. **Metrics Scraper:** scraper writes raw bytes directly into a circular buffer. If the buffer fills up, it simply overwrites the oldest data.
2. **In-Memory Time-Series Cache:** Asynchronounsly drain ring-buffer and store in optimised in-memory cache.
3. **Persistent Storage:** Periodically serialise and flush cached data into an embedded database.

## Getting Started

### Prerequisites

* Docker
* Rust

### Installation

Step-by-step instructions to get a local development environment running.

#### Pull Chronosys

```bash
git clone https://github.com/tomcmead/chronosys.git
cd chronosys
```

#### Docker Commands

```bash
# dev is hot-reload, not required to re-run 'docker compose build dev' if only ./src files changed
docker compose build dev
docker compose up dev

# Re-run 'docker compose build release' before 'up' if any ./src files changed
docker compose build release
docker compose up release
```

#### Cargo Commands

```bash
# Compile optimised binary target
cargo build --release
# Compile binary target and run executable
cargo run
# Execute all unit and integration tests
cargo test
```

## License

This program is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation; either version 2 of the License, or (at your option) any later version.

See the [LICENSE](LICENSE) file for the full license text.