//! # lnkshort
//!
//! A high-performance, production-grade URL shortening and redirection microservice.
//!
//! ## Architecture Overview
//!
//! The application is structured in layers for maintainability and scalability:
//! - [`api`]: HTTP transport layer containing Axum handlers and routing.
//! - [`services`]: Domain business logic, collision handling, and URL validation.
//! - [`repository`]: Isolated database access layer using SQLx.
//! - [`models`]: Strongly-typed entities and JSON Data Transfer Objects (DTOs).
//! - [`config`]: Application configuration loaded from environment variables.
//! - [`error`]: Centralized error types and HTTP response conversion.
//! - [`state`]: Shared dependency-injected application state container.
//! - [`telemetry`]: Tracing and Prometheus metric exporters.

pub mod api;
pub mod auth;
pub mod config;
pub mod error;
pub mod models;
pub mod repository;
pub mod routes;
pub mod services;
pub mod state;
pub mod telemetry;
pub mod utils;

pub use api::create_router;
pub use config::AppConfig;
pub use error::AppError;
pub use state::AppState;
