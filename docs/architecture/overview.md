# FileHub Architecture Overview

## System Architecture

FileHub is built as a Rust multi-crate workspace with 16 crates following
clean architecture principles (domain-driven design).

### Crate Layers

┌─────────────────────────────────────────┐
│ Presentation Layer │
│ filehub-api, filehub-cli, filehub-webdav│
├─────────────────────────────────────────┤
│ Application Layer │
│ filehub-service, filehub-worker │
│ filehub-realtime, filehub-plugin │
├─────────────────────────────────────────┤
│ Domain Layer │
│ filehub-core, filehub-entity │
├─────────────────────────────────────────┤
│ Infrastructure Layer │
│ filehub-database, filehub-cache │
│ filehub-storage, filehub-auth │
└─────────────────────────────────────────┘

### Key Design Decisions

1. **Trait-based abstractions** — Core traits defined in `filehub-core`, implemented in infrastructure crates
2. **Newtype IDs** — All entity IDs use newtype wrappers around `uuid::Uuid`
3. **Event-driven** — Domain events flow through the plugin hook system
4. **Multi-storage** — Pluggable storage backends (local, S3, WebDAV, SMB)
5. **Seat-based licensing** — FlexNet integration with atomic seat allocation
6. **Real-time** — WebSocket engine with typed channels and presence tracking

### Data Flow

HTTP Request → Auth Middleware → RBAC Check → Handler
→ Service (ACL check) → Repository → Database
→ Event → Hook Dispatcher → Plugins
→ Notification → WebSocket / Persist

### Authentication Flow

Login → Validate Credentials → Check Session Limits
→ Allocate Seat → FlexNet Checkout → Create Session
→ Generate JWT → Return Tokens

### Background Processing

- Cron scheduler enqueues periodic jobs
- Worker runner polls queues with priority ordering
- Semaphore-based concurrency control
- Retry with exponential backoff for transient failures
