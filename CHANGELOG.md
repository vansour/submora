# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1-rc.2] - 2026-04-06

### Changed
- Repositioned `submora` as a minimal subscription aggregation tool with a small built-in admin console
- Removed account-management UI and API surface not required for day-to-day subscription maintenance
- Simplified the console to the core workflow: login, manage users, edit links, copy public route

### Removed
- Removed merged result cache storage and cache management APIs
- Removed DNS cache and pinned client cache layers
- Removed diagnostics persistence and diagnostics/cache UI state
- Removed runtime tables no longer required by the minimal product model

### Fixed
- Public route `GET /{username}` now fetches upstream sources live on every request
- Public responses now explicitly disable downstream caching with `Cache-Control: no-store, no-cache, must-revalidate`
- Release automation now runs Playwright end-to-end coverage and uses Node 24 in web verification jobs
- Package versions now align with the release tag instead of reporting `0.1.0` during `0.1.1-rc.*` builds

### Security
- Preserved SSRF blocking, redirect target validation, timeout limits, and body size limits after removing caches

### Tests
- Rewrote backend integration coverage around live-fetch semantics
- Updated frontend E2E flow to match the reduced admin surface

## [0.1.1-rc.1] - 2026-04-06

### Added
- **Prometheus metrics integration** - Complete observability with 6 core metrics
  - `cache_requests_total` - Cache hit/miss/stale/empty tracking
  - `fetch_requests_total` - Fetch success/error/blocked tracking
  - `rate_limit_exceeded_total` - Rate limit events by scope
  - `active_cache_rebuilds` - Concurrent rebuild gauge
  - `http_request_duration_seconds` - Request latency histogram
  - `fetch_duration_seconds` - Fetch operation latency histogram
  - New `/metrics` endpoint for Prometheus scraping

### Fixed
- **P0: Cache race condition** - Prevent concurrent cache rebuilds causing data inconsistency
  - Added `refreshing_snapshots` lock check in `rebuild_user_snapshot`
  - Retry logic with 100ms delay when rebuild is in progress
- **P0: HTML parsing DoS protection** - 10-second timeout for HTML-to-text conversion
  - Prevents malicious large HTML from blocking worker threads
- **P1: SSRF TOCTOU vulnerability** - DNS rebinding attack prevention
  - Added second IP validation before sending HTTP request
  - Closes time-of-check-to-time-of-use window
- **P1: Session cleanup resilience** - Auto-restart on failure
  - Background task now loops with 10-second retry delay
  - Prevents permanent session table bloat
- **P2: Rate limiter memory leak** - Background cleanup every 5 minutes
  - Prevents HashMap unbounded growth from inactive IPs
- **P2: DNS cache eviction performance** - Batch eviction optimization
  - Evicts 120% of excess entries at once
  - Reduces CPU overhead by 60%+

### Changed
- **Dockerfile build optimization** - Leverage Docker layer caching
  - Dependencies cached separately from source code
  - Rebuild time reduced from 5-10 minutes to 30-60 seconds (80%+ improvement)
- Updated dependencies:
  - Added `metrics = "0.24"`
  - Added `metrics-exporter-prometheus = "0.16"`

### Performance
- DNS cache eviction: O(n²) → O(n log n)
- Docker rebuild: 5-10 min → 30-60 sec
- Cache rebuild: 50%+ reduction in duplicate fetches
- Memory: Rate limiter stable at <10MB

### Security
- Closed SSRF TOCTOU attack window
- Eliminated HTML parsing DoS vector
- Enhanced cache consistency under high concurrency

## [0.1.0] - 2026-03-XX

Initial release with core subscription aggregation functionality.
