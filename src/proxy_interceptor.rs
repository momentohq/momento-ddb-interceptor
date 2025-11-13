use std::time::Duration;

/// Extension trait for DynamoDB config builder to add Momento accelerator support.
pub trait MomentoAccelerator {
    /// Configure the DynamoDB client to route requests through a Momento accelerator.
    ///
    /// See [accelerator_config] for a builder-style configuration.
    ///
    /// # Example
    /// ```
    /// use aws_sdk_dynamodb::config::Builder as DynamoDbConfigBuilder;
    /// use momento_ddb_interceptor::{MomentoAccelerator, accelerator_config};
    /// use std::time::Duration;
    ///
    /// let dynamodb_config = DynamoDbConfigBuilder::default()
    ///     .with_momento_accelerator(
    ///         accelerator_config()
    ///             .cache_name("my-dynamo-cache")
    ///             .momento_hostname("api.cache.cell-us-west-2-1.prod.a.momentohq.com")
    ///             .auth_token("my-momento-auth-token")
    ///             .ttl(Duration::from_secs(60))
    ///     ).build();
    /// ```
    fn with_momento_accelerator(self, config: AcceleratorConfig) -> Self;
}

impl MomentoAccelerator for aws_sdk_dynamodb::config::Builder {
    fn with_momento_accelerator(
        self,
        AcceleratorConfig {
            uri,
            auth_token,
            ttl,
        }: AcceleratorConfig,
    ) -> Self {
        let interceptor = ProxyInterceptor::new(uri, auth_token, ttl);
        self.interceptor(interceptor)
    }
}

/// Build a Momento accelerator configuration
pub fn accelerator_config() -> AcceleratorConfigBuilder<WantsCacheName> {
    AcceleratorConfigBuilder(WantsCacheName)
}

/// Configuration builder for Momento Accelerator
pub struct AcceleratorConfigBuilder<T>(T);

/// MomentoAcceleratorConfig state: wants cache name
pub struct WantsCacheName;
impl AcceleratorConfigBuilder<WantsCacheName> {
    /// Set the Momento cache name. This is the cache that will store your DynamoDB items.
    pub fn cache_name(
        self,
        cache_name: impl Into<String>,
    ) -> AcceleratorConfigBuilder<WantsMomentoHost> {
        let cache_name = cache_name.into();
        AcceleratorConfigBuilder(WantsMomentoHost { cache_name })
    }
}

/// MomentoAcceleratorConfig state: wants URI
pub struct WantsMomentoHost {
    cache_name: String,
}
impl AcceleratorConfigBuilder<WantsMomentoHost> {
    /// Set the Momento accelerator hostname.
    /// Something like "api.cache.cell-us-west-2-1.prod.a.momentohq.com"
    pub fn momento_hostname(
        self,
        uri: impl Into<String>,
    ) -> AcceleratorConfigBuilder<WantsAuthToken> {
        let uri = uri.into();
        let cache_name = self.0.cache_name;
        AcceleratorConfigBuilder(WantsAuthToken {
            uri: format!("https://{uri}/ddb/{cache_name}/cache"),
        })
    }
}

/// MomentoAcceleratorConfig state: wants auth token
pub struct WantsAuthToken {
    uri: String,
}
impl AcceleratorConfigBuilder<WantsAuthToken> {
    /// Set your Momento auth token.
    pub fn auth_token(self, auth_token: impl Into<String>) -> AcceleratorConfigBuilder<WantsTtl> {
        AcceleratorConfigBuilder(WantsTtl {
            uri: self.0.uri,
            auth_token: auth_token.into(),
        })
    }
}

/// MomentoAcceleratorConfig state: wants TTL
pub struct WantsTtl {
    uri: String,
    auth_token: String,
}
impl AcceleratorConfigBuilder<WantsTtl> {
    /// Set the TTL for DynamoDB items stored in the Momento cache.
    pub fn ttl(self, ttl: Duration) -> AcceleratorConfig {
        AcceleratorConfig {
            uri: self.0.uri,
            auth_token: self.0.auth_token,
            ttl,
        }
    }
}

/// A configuration for Momento accelerator
pub struct AcceleratorConfig {
    uri: String,
    auth_token: String,
    ttl: Duration,
}

/// Post-signature interceptor that routes GetItem requests through a Momento proxy
#[derive(Debug)]
pub struct ProxyInterceptor {
    proxy_uri: String,
    auth_token: String,
    ttl: String,
}

impl ProxyInterceptor {
    fn new(proxy_uri: impl Into<String>, auth_token: impl Into<String>, ttl: Duration) -> Self {
        Self {
            proxy_uri: proxy_uri.into(),
            auth_token: auth_token.into(),
            // Pre-convert to a header-friendly string
            ttl: ttl.as_millis().min(u32::MAX as u128).to_string(),
        }
    }
}

impl aws_sdk_dynamodb::config::Intercept for ProxyInterceptor {
    fn name(&self) -> &'static str {
        "MomentoProxy"
    }

    fn modify_before_transmit(
        &self,
        context: &mut aws_sdk_dynamodb::config::interceptors::BeforeTransmitInterceptorContextMut<
            '_,
        >,
        _runtime_components: &aws_sdk_dynamodb::config::RuntimeComponents,
        _cfg: &mut aws_sdk_dynamodb::config::ConfigBag,
    ) -> Result<(), aws_sdk_dynamodb::error::BoxError> {
        let requested = context.request().uri().to_string();
        log::trace!("replacing {requested} with {proxy}", proxy = self.proxy_uri);
        // Set the request uri to the proxy uri. This is after the request is signed, so this request
        // is proxyable and secure against modification. Make sure you trust the proxy to make this request!
        // Smithy does not export this `Uri` symbol so you have to take this expect in each request path rather
        // than once in the new()...
        *context.request_mut().uri_mut() = self
            .proxy_uri
            .clone()
            .try_into()
            .expect("must be a valid uri");

        // Proxy uses x-uri to replace the original uri when it needs to forward the request
        context
            .request_mut()
            .headers_mut()
            .insert("x-uri", requested);

        // Include the auth header for the proxy
        context
            .request_mut()
            .headers_mut()
            .insert("x-momento-authorization", self.auth_token.clone());

        // Include the auth header for the proxy
        context
            .request_mut()
            .headers_mut()
            .insert("x-ttl-millis", self.ttl.clone());

        Ok(())
    }
}
