//! # filehub-api
//!
//! HTTP API layer for FileHub built on Axum.
//!
//! Provides all REST endpoints, WebSocket upgrade, middleware (auth, RBAC,
//! rate limiting, CORS, logging), extractors, DTOs, and error mapping.

pub mod app;
pub mod dto;
pub mod extractors;
pub mod handlers;
pub mod middleware;
pub mod router;
pub mod state;

pub use app::build_app;
pub use state::AppState;
