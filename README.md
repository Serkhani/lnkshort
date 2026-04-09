# lnkshort

A high-performance, production-grade URL shortening and redirection service written in Rust using [Axum](https://github.com/tokio-rs/axum), [SeaORM](https://www.sea-ql.org/SeaORM/), and PostgreSQL.

Built for low latency, resilience under load, and maintainability through a clean layered architecture.

---

## Table of Contents

- [lnkshort](#lnkshort)
  - [Table of Contents](#table-of-contents)
  - [Features](#features)
  - [Architecture \& Design Rationale](#architecture--design-rationale)
    - [Why this architecture?](#why-this-architecture)
  - [Quick Start](#quick-start)
    - [Prerequisites](#prerequisites)
    - [1. Start Database](#1-start-database)
    - [2. Apply Migrations](#2-apply-migrations)
    - [3. Configure Environment](#3-configure-environment)
    - [4. Run the Server](#4-run-the-server)
  - [API Reference](#api-reference)
    - [Authentication](#authentication)
    - [Endpoints](#endpoints)
    - [Sample Requests](#sample-requests)
      - [Health Check](#health-check)
      - [Create a Short Link (Protected)](#create-a-short-link-protected)
      - [Redirect to Destination (Public)](#redirect-to-destination-public)
      - [Get Link Statistics (Protected)](#get-link-statistics-protected)
  - [Configuration](#configuration)
  - [Observability \& Metrics](#observability--metrics)
  - [Resilience \& Performance Mechanisms](#resilience--performance-mechanisms)
    - [1. Collision Handling with Retries](#1-collision-handling-with-retries)
    - [2. Isolated Analytics Recording](#2-isolated-analytics-recording)
    - [3. Constant-Time Authentication](#3-constant-time-authentication)
    - [4. Edge Caching Headers](#4-edge-caching-headers)
    - [5. Token-Bucket Rate Limiting](#5-token-bucket-rate-limiting)
  - [Testing \& Quality Assurance](#testing--quality-assurance)
  - [Future Expansion Guide](#future-expansion-guide)

---

## Features

- **Fast Redirects**: Temporary redirects (`307 Temporary Redirect`) with customizable `Cache-Control` headers for edge/CDN caching.
- **Layered Clean Architecture**: Strict separation of concerns between presentation (handlers/middleware), business logic (services), data access (repositories), and domain models.
- **Collision-Resistant ID Generation**: Non-deterministic URL-safe IDs with automatic retry loop and collision telemetry.
- **Resilient Analytics Tracking**: Request analytics (User-Agent, Referer) recorded with query timeouts so redirection latency remains unaffected by database delays.
- **Constant-Time Authentication**: SHA3-256 API key verification with constant-time byte comparison against timing attacks.
- **URL Scheme Validation**: Defense against open-redirect XSS exploits (e.g. `javascript:` or `data:` schemes).
- **Token-Bucket Rate Limiting**: Built-in IP-based rate limiting via `tower_governor` with `SmartIpKeyExtractor` (header-aware proxy support) protecting all application routes with HTTP 429 and `Retry-After` headers.
- **Graceful Shutdown**: Native support for `SIGINT` (Ctrl+C) and `SIGTERM` signals for clean drain in Kubernetes/Docker environments.
- **Prometheus Telemetry**: Integrated metrics endpoint (`/metrics`) tracking request counts, unauthenticated calls, error counts, rate limited requests, and collision events.

---

## Architecture & Design Rationale

The codebase follows a modular, dependency-injected design:

```code
src/
├── lib.rs              # Library root: re-usable router, state, and modules
├── main.rs             # Application runner: loads config, sets up DB pool & signals
├── config.rs           # Strongly-typed environment configuration
├── error.rs            # Centralized AppError enum with IntoResponse & metrics
├── state.rs            # AppState container (DB pool, config, injected services)
├── telemetry.rs        # Tracing subscriber and Prometheus metrics setup
├── models/             # Domain entities and DTOs (Link, LinkStats, Setting)
├── repository/         # Data access layer (SeaORM queries isolated from domain logic)
├── services/           # Business logic (retries, ID generation, validation, crypto)
└── api/                # HTTP presentation layer (Axum routers, handlers, middleware)
```

### Why this architecture?

1. **Decoupling Transports from Storage**: Database operations are encapsulated inside `repository/` using SeaORM entities and active models. The service layer does not know whether data comes from PostgreSQL, an in-memory cache, or an external KV store.
2. **Testability**: Splitting into a library (`lib.rs`) and binary (`main.rs`) allows comprehensive integration tests against Axum routers without spawning external processes.
3. **Dependency Injection**: `AppState` holds cloned service handles and repository instances, allowing easy replacement or mocking during tests.
4. **Resilience by Default**: All database interactions are wrapped with configurable timeouts (`QUERY_TIMEOUT_MS`) to prevent connection pool exhaustion during database slowdowns.

Detailed architectural deep-dive is available in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/) (1.80+ or 2024 edition)
- [Docker](https://www.docker.com/) & Docker Compose
- [sqlx-cli](https://github.com/launchbadge/sqlx) (optional, for running migrations directly)

### 1. Start Database

Start PostgreSQL using the provided `docker-compose.yml`:

```bash
docker compose up -d
```

This spins up PostgreSQL 16 on port `5454`.

### 2. Apply Migrations

Run migrations using `sqlx-cli`:

```bash
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run
```

Or connect directly to PostgreSQL and run the SQL scripts located in `migrations/`:

- `migrations/20261024175749_init.up.sql`
- `migrations/20261026162953_linkstats.up.sql`
- `migrations/20261026230432_settings.up.sql`

### 3. Configure Environment

Create or edit `.env` in the project root:

```env
DATABASE_URL=postgres://POSTGRES_USER:POSTGRES_PASSWORD@localhost:5454/POSTGRES_DB
HOST=0.0.0.0
PORT=3000
RUST_LOG=lnkshort=debug,tower_http=debug
DB_MAX_CONNECTIONS=12
DB_MIN_CONNECTIONS=2
QUERY_TIMEOUT_MS=300
LINK_MAX_RETRIES=3
```

### 4. Run the Server

```bash
cargo run
```

Output:

```text
INFO lnkshort: Initializing database connection pool...
INFO lnkshort: Server running on: http://0.0.0.0:3000
```

---

## API Reference

### Authentication

Protected routes require the `x-api-key` HTTP header.

The API key is hashed using **SHA3-256** and compared against the hash stored in the `settings` database table.

To generate a SHA3-256 hash for a new key:

```bash
# Python
python3 -c "import hashlib; print(hashlib.sha3_256(b'YOUR_API_KEY').hexdigest())"

# OpenSSL
echo -n "YOUR_API_KEY" | openssl dgst -sha3-256
```

### Endpoints

| Method | Path | Auth Required | Description |
| --- | --- | --- | --- |
| `GET` | `/health` | No | Health check endpoint |
| `POST` | `/create` | **Yes** | Create a new shortened link |
| `GET` | `/{id}` | No | Redirect to the target URL (records click) |
| `PUT` | `/{id}` | **Yes** | Update destination URL for an existing link |
| `GET` | `/{id}/stats` | **Yes** | Fetch aggregated visit stats for a link |
| `GET` | `/metrics` | No | Prometheus telemetry metrics |

### Sample Requests

#### Health Check

```bash
curl -i http://localhost:3000/health
```

```http
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8

OK
```

#### Create a Short Link (Protected)

```bash
curl -i -X POST http://localhost:3000/create \
  -H "Content-Type: application/json" \
  -H "x-api-key: YOUR_API_KEY" \
  -d '{"targetUrl": "https://github.com/tokio-rs/axum"}'
```

```http
HTTP/1.1 200 OK
content-type: application/json

{"id":"wE1z9Aa","targetUrl":"https://github.com/tokio-rs/axum"}
```

#### Redirect to Destination (Public)

```bash
curl -i http://localhost:3000/wE1z9Aa
```

```http
HTTP/1.1 307 Temporary Redirect
Location: https://github.com/tokio-rs/axum
Cache-Control: public, max-age=300, s-maxage=300, state-while-revalidate=300, stale-if-error=300
```

#### Get Link Statistics (Protected)

```bash
curl -i http://localhost:3000/wE1z9Aa/stats \
  -H "x-api-key: YOUR_API_KEY"
```

```http
HTTP/1.1 200 OK
content-type: application/json

[
  {
    "amount": 42,
    "referer": "https://twitter.com",
    "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)..."
  }
]
```

Full request/response examples and error formats are in [docs/API.md](docs/API.md).

---

## Configuration

All configuration settings are loaded via environment variables or fall back to sensible production defaults:

| Variable | Default | Description |
| --- | --- | --- |
| `DATABASE_URL` | *Required* | PostgreSQL connection string |
| `HOST` | `0.0.0.0` | Network interface to bind server |
| `PORT` | `3000` | Port to bind server |
| `DB_MAX_CONNECTIONS` | `12` | Maximum SQL connection pool size |
| `DB_MIN_CONNECTIONS` | `2` | Minimum idle connections retained |
| `QUERY_TIMEOUT_MS` | `300` | Timeout in milliseconds per database operation |
| `LINK_MAX_RETRIES` | `3` | Maximum attempts to generate a non-colliding ID |
| `DEFAULT_CACHE_CONTROL` | `public, max-age=300...` | Cache-Control header applied to 307 redirects |
| `RATE_LIMIT_ENABLED` | `true` | Enable or disable token-bucket rate limiting |
| `RATE_LIMIT_PER_SECOND` | `50` | Refill rate in requests per second per IP |
| `RATE_LIMIT_BURST` | `100` | Maximum burst capacity per IP |
| `RUST_LOG` | `lnkshort=debug` | Tracing log filter expression |

---

## Observability & Metrics

Prometheus metrics are collected via `axum-prometheus` and exposed at `GET /metrics`.

Key domain metrics:

- `unauthenticated_calls_count`: Incremented when an unauthorized request is made (labeled by `uri`).
- `rate_limited_requests_count`: Incremented when incoming requests exceed token-bucket rate limits (HTTP 429).
- `exhausting_unique_entries`: Incremented when all unique short link collision retry attempts are exhausted.
- `request_error`: Incremented on unexpected errors or database failures (labeled by `error`).
- `axum_http_requests_total`: Total HTTP requests served.
- `axum_http_requests_duration_seconds`: Request latency histogram.

---

## Resilience & Performance Mechanisms

### 1. Collision Handling with Retries

Short IDs are generated using random 64-bit entropy encoded as URL-safe base64. When a duplicate key collision occurs on the `links` primary key:

- The error is trapped by the `ErrorKind::UniqueViolation` match.
- A warning is logged and a fresh ID is generated up to `LINK_MAX_RETRIES` times.
- If all retries fail, `exhausting_unique_entries` metric fires and an error is returned.

### 2. Isolated Analytics Recording

When redirecting users, link lookups are prioritized. Analytics insertion (`lnkstats`) runs with a query timeout and will not fail the redirect if the analytics write times out or encounters an error.

### 3. Constant-Time Authentication

To mitigate side-channel timing attacks where an attacker infers key characters based on string comparison duration, `AuthService::constant_time_compare` evaluates all bytes in constant time.

### 4. Edge Caching Headers

Redirect responses include `Cache-Control: public, max-age=300, s-maxage=300, stale-while-revalidate=300`. Downstream reverse proxies, CDNs (e.g. Cloudflare), and browsers cache the redirect, drastically reducing load on the database for viral links.

### 5. Token-Bucket Rate Limiting

The application embeds an in-memory token-bucket rate limiter via `tower_governor`:

- **Smart IP Extraction**: Evaluates `Forwarded`, `X-Forwarded-For`, and `X-Real-IP` proxy headers, falling back gracefully to peer socket addresses (`ConnectInfo<SocketAddr>`).
- **Standardized Responses**: Rejections emit `HTTP 429 Too Many Requests` along with standard `Retry-After` headers and JSON error payloads.
- **Operational Route Immunity**: Telemetry (`/metrics`) and health check (`/health`) endpoints bypass rate limits to avoid disrupting Prometheus scrapers and Kubernetes probes.

---

## Testing & Quality Assurance

Run unit tests:

```bash
cargo test
```

Run linter checks:

```bash
cargo clippy -- -D warnings
```

Format code:

```bash
cargo fmt --check
```

---

## Future Expansion Guide

The refactored modular structure allows painless expansion for enterprise workloads:

- **Adding a Redis Cache**: Wrap `LinkRepository` with a caching repository implementation (or implement a `LinkCache` trait) to serve hot redirects straight from memory.
- **Async Batch Analytics**: Replace direct SQL stat inserts with an in-memory queue (`tokio::sync::mpsc`) and flush batches periodically to PostgreSQL or ClickHouse.
- **Distributed Rate Limiting**: Swap in-memory governor state with Redis Cell or Redis Token Bucket across multi-region server clusters.✅

For complete details, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
