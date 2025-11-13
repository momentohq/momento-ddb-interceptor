#![deny(missing_docs)]

//! Momento DynamoDB Proxy Interceptor
//!
//! This crate provides a DynamoDB client interceptor that routes signed requests through a Momento cache proxy.
//! It allows you to accelerate DynamoDB read operations by caching frequently accessed items in Momento.
//!
//! # Example
//! ```rust,no_run
//! use momento_ddb_interceptor::{MomentoAccelerator, accelerator_config};
//! use std::time::Duration;
//!
//! let dynamodb_config = aws_sdk_dynamodb::config::Builder::default()
//!     .with_momento_accelerator(
//!         accelerator_config()
//!             .cache_name("my-dynamo-cache")
//!             .momento_hostname("api.cache.cell-us-west-2-1.prod.a.momentohq.com")
//!             .auth_token("my-momento-auth-token")
//!             .ttl(Duration::from_secs(60))
//!     ).build();
//!
//! let dynamodb_client = aws_sdk_dynamodb::Client::from_conf(dynamodb_config);
//! ```

mod proxy_interceptor;

pub use proxy_interceptor::{
    AcceleratorConfig, AcceleratorConfigBuilder, MomentoAccelerator, ProxyInterceptor,
    accelerator_config,
};
