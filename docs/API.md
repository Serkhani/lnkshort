# API Specification & Reference

This document provides complete documentation for the HTTP API exposed by **lnkshort**.

---

## Base URL

When running locally:

```http
http://localhost:3000
```

---

## Authentication

Administrative and mutating endpoints require an API key passed via the HTTP header:

```http
x-api-key: <YOUR_API_KEY>
```

If the header is missing or the provided key is invalid, the server responds with:

```http
HTTP/1.1 401 Unauthorized
content-type: text/plain; charset=utf-8

Unauthorised
```

---

## Endpoints

### 1. Health Check

Verifies server liveness for load balancers and orchestrators.

- **URL**: `/health`
- **Method**: `GET`
- **Auth Required**: No

#### Response

- **Status**: `200 OK`
- **Body**: `OK`

```bash
curl -i http://localhost:3000/health
```

---

### 2. Create Short Link

Creates a new shortened link pointing to the specified destination URL.

- **URL**: `/create`
- **Method**: `POST`
- **Auth Required**: Yes (`x-api-key`)
- **Headers**:
  - `Content-Type: application/json`
  - `x-api-key: <API_KEY>`

#### Request Body 1

```json
{
  "targetUrl": "https://example.com/very/long/path/with?query=params"
}
```

#### Successful Response (`200 OK`) 1

```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "id": "1840294829104",
  "targetUrl": "https://example.com/very/long/path/with?query=params"
}
```

#### Error Responses 1

- **`409 Conflict`**: Target URL is malformed or uses an unsupported scheme.
  
  ```http
  HTTP/1.1 409 Conflict
  Content-Type: text/plain; charset=utf-8

  url malformed
  ```

- **`500 Internal Server Error`**: All collision retry attempts exhausted.
  
  ```http
  HTTP/1.1 500 Internal Server Error

  Internal server error
  ```

#### Example cURL 1

```bash
curl -i -X POST http://localhost:3000/create \
  -H "Content-Type: application/json" \
  -H "x-api-key: DEFAULT_KEY" \
  -d '{"targetUrl": "https://rust-lang.org"}'
```

---

### 3. Redirect to Target URL

Resolves a short link ID, registers a click event, and redirects the client to the destination.

- **URL**: `/{id}`
- **Method**: `GET`
- **Auth Required**: No
- **Path Parameters**:
  - `id` (string, required): Short link identifier.

#### Successful Response (`307 Temporary Redirect`) 2

```http
HTTP/1.1 307 Temporary Redirect
Location: https://rust-lang.org
Cache-Control: public, max-age=300, s-maxage=300, state-while-revalidate=300, stale-if-error=300
```

#### Error Response 2

- **`404 Not Found`**: Link ID does not exist.
  
  ```http
  HTTP/1.1 404 Not Found
  Content-Type: text/plain; charset=utf-8

  Not found
  ```

#### Example cURL 2

```bash
curl -i http://localhost:3000/1840294829104
```

---

### 4. Update Short Link

Updates the destination URL of an existing short link.

- **URL**: `/{id}`
- **Method**: `PUT`
- **Auth Required**: Yes (`x-api-key`)
- **Path Parameters**:
  - `id` (string, required): Short link identifier to update.
- **Headers**:
  - `Content-Type: application/json`
  - `x-api-key: <API_KEY>`

#### Request Body 2

```json
{
  "targetUrl": "https://new-destination.example.com"
}
```

#### Successful Response (`200 OK`) 3

```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "id": "1840294829104",
  "targetUrl": "https://new-destination.example.com"
}
```

#### Error Responses 3

- **`404 Not Found`**: The specified link ID does not exist.
- **`409 Conflict`**: Target URL is malformed or uses an unsupported scheme.

#### Example cURL 3

```bash
curl -i -X PUT http://localhost:3000/1840294829104 \
  -H "Content-Type: application/json" \
  -H "x-api-key: DEFAULT_KEY" \
  -d '{"targetUrl": "https://new-destination.example.com"}'
```

---

### 5. Get Link Analytics / Statistics

Retrieves aggregated visitor statistics for a specific link, grouped by Referer and User-Agent.

- **URL**: `/{id}/stats`
- **Method**: `GET`
- **Auth Required**: Yes (`x-api-key`)
- **Path Parameters**:
  - `id` (string, required): Short link identifier.
- **Headers**:
  - `x-api-key: <API_KEY>`

#### Successful Response (`200 OK`) 4

```http
HTTP/1.1 200 OK
Content-Type: application/json

[
  {
    "amount": 14,
    "referer": "https://news.ycombinator.com/",
    "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36..."
  },
  {
    "amount": 3,
    "referer": null,
    "userAgent": "curl/8.7.1"
  }
]
```

#### Example cURL 4

```bash
curl -i http://localhost:3000/1840294829104/stats \
  -H "x-api-key: DEFAULT_KEY"
```

---

### 6. Prometheus Telemetry

Renders Prometheus text-formatted metrics for scraping.

- **URL**: `/metrics`
- **Method**: `GET`
- **Auth Required**: No

#### Sample Metrics Exposed

```http
# HELP unauthenticated_calls_count Total unauthenticated API attempts
# TYPE unauthenticated_calls_count counter
unauthenticated_calls_count{uri="/create!"} 3

# HELP exhausting_unique_entries Count of exhausted ID retry collisions
# TYPE exhausting_unique_entries counter
exhausting_unique_entries 0

# HELP request_error Errors encountered during request processing
# TYPE request_error counter
request_error{error="PoolTimedOut!"} 1

# Standard Axum HTTP metrics
axum_http_requests_total{method="GET",status="307"} 1542
axum_http_requests_duration_seconds_bucket{le="0.005"} 1500

# Rate limiting metrics
# HELP rate_limited_requests_count Total rate-limited requests rejected with HTTP 429
# TYPE rate_limited_requests_count counter
rate_limited_requests_count 4
```

#### Example cURL 5

```bash
curl -i http://localhost:3000/metrics
```

---

## Rate Limiting & Throttling

Application routes (`POST /create`, `GET /{id}`, `PUT /{id}`, `GET /{id}/stats`) are protected by a token-bucket rate limiter.

When a client IP exceeds the configured burst capacity or refill rate:

### Response

- **Status**: `429 Too Many Requests`
- **Headers**:
  - `Content-Type: application/json`
  - `Retry-After: <seconds>` (estimated seconds until quota is replenished)

```http
HTTP/1.1 429 Too Many Requests
content-type: application/json
retry-after: 2

{"error":"Too Many Requests"}
```

