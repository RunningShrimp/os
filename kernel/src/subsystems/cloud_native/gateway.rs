// API Gateway Implementation
//
// API网关实现
// 提供统一API入口、路由转发、协议转换、认证授权和流量控制

extern crate alloc;

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::sync::atomic::AtomicU64;
use spin::Mutex;

use crate::reliability::{EIO, ENOENT};

/// API Gateway配置
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Gateway名称
    pub gateway_name: String,
    /// 监听地址
    pub listen_address: String,
    /// HTTP端口
    pub http_port: u16,
    /// HTTPS端口
    pub https_port: u16,
    /// GRPC端口
    pub grpc_port: u16,
    /// 是否启用TLS
    pub enable_tls: bool,
    /// TLS证书
    pub tls_cert: Option<String>,
    /// TLS私钥
    pub tls_key: Option<String>,
    /// 认证配置
    pub auth_config: AuthConfig,
    /// 限流配置
    pub rate_limit_config: RateLimitConfig,
    /// 超时配置
    pub timeout_config: TimeoutConfig,
    /// API版本管理
    pub versioning: ApiVersioningConfig,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            gateway_name: "default-gateway".to_string(),
            listen_address: "0.0.0.0".to_string(),
            http_port: 8080,
            https_port: 8443,
            grpc_port: 9090,
            enable_tls: false,
            tls_cert: None,
            tls_key: None,
            auth_config: AuthConfig::default(),
            rate_limit_config: RateLimitConfig::default(),
            timeout_config: TimeoutConfig::default(),
            versioning: ApiVersioningConfig::default(),
        }
    }
}

/// 认证配置
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// 认证提供者
    pub providers: Vec<AuthProvider>,
    /// 默认认证策略
    pub default_policy: AuthPolicy,
    /// JWT配置
    pub jwt_config: Option<JwtConfig>,
    /// OAuth2配置
    pub oauth2_config: Option<OAuth2Config>,
    /// API Key配置
    pub api_key_config: Option<ApiKeyConfig>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            default_policy: AuthPolicy::AuthenticationRequired,
            jwt_config: None,
            oauth2_config: None,
            api_key_config: None,
        }
    }
}

/// 认证提供者
#[derive(Debug, Clone)]
pub struct AuthProvider {
    /// 提供者名称
    pub name: String,
    /// 提供者类型
    pub provider_type: AuthProviderType,
    /// 提供者配置
    pub config: BTreeMap<String, String>,
    /// 优先级
    pub priority: u32,
}

/// 认证提供者类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthProviderType {
    /// JWT
    Jwt,
    /// OAuth2
    OAuth2,
    /// API Key
    ApiKey,
    /// Basic Auth
    Basic,
    /// mTLS
    MutualTls,
    /// LDAP
    Ldap,
    /// OIDC
    Oidc,
}

/// 认证策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthPolicy {
    /// 无需认证
    None,
    /// 需要认证
    AuthenticationRequired,
    /// 可选认证
    AuthenticationOptional,
}

/// JWT配置
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Issuer
    pub issuer: String,
    /// Audience
    pub audience: String,
    /// 公钥(PEM格式)
    pub public_key: String,
    /// 算法
    pub algorithm: String,
    /// 过期容忍(秒)
    pub clock_skew_sec: u64,
}

/// OAuth2配置
#[derive(Debug, Clone)]
pub struct OAuth2Config {
    /// 授权端点
    pub authorization_endpoint: String,
    /// Token端点
    pub token_endpoint: String,
    /// 客户端ID
    pub client_id: String,
    /// Scopes
    pub scopes: Vec<String>,
    /// 重定向URI
    pub redirect_uri: String,
}

/// API Key配置
#[derive(Debug, Clone)]
pub struct ApiKeyConfig {
    /// API Key位置
    pub key_location: ApiKeyLocation,
    /// Key名称
    pub key_name: String,
    /// 验证端点
    pub validation_endpoint: Option<String>,
}

/// API Key位置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyLocation {
    /// 头部
    Header,
    /// 查询参数
    Query,
    /// Cookie
    Cookie,
}

/// 限流配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 全局限流
    pub global_limits: RateLimit,
    /// 按服务的限流
    pub service_limits: BTreeMap<String, RateLimit>,
    /// 按API的限流
    pub api_limits: BTreeMap<String, RateLimit>,
    /// 限流算法
    pub algorithm: RateLimitAlgorithm,
    /// 限流窗口
    pub window_size_sec: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_limits: RateLimit {
                requests_per_second: 10000,
                burst: 20000,
            },
            service_limits: BTreeMap::new(),
            api_limits: BTreeMap::new(),
            algorithm: RateLimitAlgorithm::TokenBucket,
            window_size_sec: 60,
        }
    }
}

/// 限流规则
#[derive(Debug, Clone)]
pub struct RateLimit {
    /// 每秒请求数
    pub requests_per_second: u64,
    /// 突发容量
    pub burst: u64,
}

/// 限流算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitAlgorithm {
    /// 令牌桶
    TokenBucket,
    /// 漏桶
    LeakyBucket,
    /// 固定窗口
    FixedWindow,
    /// 滑动窗口
    SlidingWindow,
}

/// 超时配置
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// 默认超时(毫秒)
    pub default_timeout_ms: u64,
    /// 按服务的超时
    pub service_timeouts: BTreeMap<String, u64>,
    /// 按API的超时
    pub api_timeouts: BTreeMap<String, u64>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 30000, // 30秒
            service_timeouts: BTreeMap::new(),
            api_timeouts: BTreeMap::new(),
        }
    }
}

/// API版本管理配置
#[derive(Debug, Clone)]
pub struct ApiVersioningConfig {
    /// 版本策略
    pub versioning_strategy: VersioningStrategy,
    /// 默认版本
    pub default_version: String,
    /// 支持的版本
    pub supported_versions: Vec<String>,
    /// 废弃的版本
    pub deprecated_versions: Vec<String>,
    /// 版本过期时间
    pub version_expiration: BTreeMap<String, u64>,
}

impl Default for ApiVersioningConfig {
    fn default() -> Self {
        Self {
            versioning_strategy: VersioningStrategy::UrlPath,
            default_version: "v1".to_string(),
            supported_versions: vec!["v1".to_string(), "v2".to_string()],
            deprecated_versions: Vec::new(),
            version_expiration: BTreeMap::new(),
        }
    }
}

/// 版本管理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersioningStrategy {
    /// URL路径
    UrlPath,
    /// 头部
    Header,
    /// 查询参数
    Query,
    /// 内容类型
    ContentType,
}

/// API Gateway
pub struct ApiGateway {
    /// Gateway配置
    config: GatewayConfig,
    /// 路由表
    routes: BTreeMap<String, Route>,
    /// API定义
    apis: BTreeMap<String, ApiDefinition>,
    /// 插件列表
    plugins: Vec<Box<dyn GatewayPlugin>>,
    /// 限流器
    rate_limiter: Arc<Mutex<RateLimiter>>,
    /// 认证器
    authenticator: Arc<Mutex<Authenticator>>,
    /// 统计信息
    stats: Arc<Mutex<GatewayStats>>,
    /// 下一个API ID
    next_api_id: AtomicU64,
}

/// 路由
#[derive(Debug, Clone)]
pub struct Route {
    /// 路由ID
    pub id: String,
    /// 路由路径
    pub path: String,
    /// 路径匹配类型
    pub path_match_type: PathMatchType,
    /// HTTP方法
    pub methods: Vec<HttpMethod>,
    /// 目标服务
    pub target_service: String,
    /// 目标端口
    pub target_port: u16,
    /// 超时(毫秒)
    pub timeout_ms: u64,
    /// 重试策略
    pub retry_policy: Option<RetryPolicy>,
    /// 插件
    pub plugins: Vec<String>,
    /// 是否启用
    pub enabled: bool,
    /// 元数据
    pub metadata: BTreeMap<String, String>,
}

/// 路径匹配类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathMatchType {
    /// 精确匹配
    Exact,
    /// 前缀匹配
    Prefix,
    /// 正则匹配
    Regex,
}

/// HTTP方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
}

/// API定义
#[derive(Debug, Clone)]
pub struct ApiDefinition {
    /// API ID
    pub id: String,
    /// API名称
    pub name: String,
    /// API版本
    pub version: String,
    /// 基础路径
    pub base_path: String,
    /// 路由列表
    pub routes: Vec<String>,
    /// 认证要求
    pub auth_required: bool,
    /// 限流配置
    pub rate_limit: Option<RateLimit>,
    /// 超时配置
    pub timeout_ms: Option<u64>,
    /// CORS配置
    pub cors_config: Option<CorsConfig>,
    /// 缓存配置
    pub cache_config: Option<CacheConfig>,
    /// 描述
    pub description: Option<String>,
    /// 标签
    pub tags: Vec<String>,
}

/// CORS配置
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// 允许的源
    pub allow_origins: Vec<String>,
    /// 允许的方法
    pub allow_methods: Vec<HttpMethod>,
    /// 允许的头部
    pub allow_headers: Vec<String>,
    /// 暴露的头部
    pub expose_headers: Vec<String>,
    /// 是否允许凭证
    pub allow_credentials: bool,
    /// 最大缓存时间(秒)
    pub max_age: Option<u64>,
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 缓存时间(秒)
    pub ttl_sec: u64,
    /// 缓存键
    pub cache_key_headers: Vec<String>,
    /// 是否允许按方法缓存
    pub cacheable_methods: Vec<HttpMethod>,
}

/// 重试策略
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数
    pub max_retries: usize,
    /// 重试间隔(毫秒)
    pub backoff_ms: u64,
    /// 可重试的状态码
    pub retryable_status_codes: Vec<u16>,
}

/// Gateway插件trait
pub trait GatewayPlugin {
    /// 插件名称
    fn name(&self) -> &str;

    /// 请求前处理
    fn on_request(&mut self, context: &mut RequestContext) -> Result<(), i32>;

    /// 响应后处理
    fn on_response(&mut self, context: &mut ResponseContext) -> Result<(), i32>;
}

/// 请求上下文
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// 请求ID
    pub request_id: String,
    /// 源地址
    pub source_address: String,
    /// HTTP方法
    pub method: HttpMethod,
    /// 路径
    pub path: String,
    /// 查询参数
    pub query_params: BTreeMap<String, String>,
    /// 头部
    pub headers: BTreeMap<String, String>,
    /// 主体
    pub body: Option<Vec<u8>>,
    /// API版本
    pub api_version: Option<String>,
    /// 认证信息
    pub auth_info: Option<AuthInfo>,
    /// 开始时间
    pub start_time: u64,
    /// 元数据
    pub metadata: BTreeMap<String, String>,
}

/// 响应上下文
#[derive(Debug, Clone)]
pub struct ResponseContext {
    /// 请求ID
    pub request_id: String,
    /// 状态码
    pub status_code: u16,
    /// 头部
    pub headers: BTreeMap<String, String>,
    /// 主体
    pub body: Option<Vec<u8>>,
    /// 延迟(毫秒)
    pub latency_ms: u64,
    /// 上游地址
    pub upstream_address: Option<String>,
    /// 错误信息
    pub error: Option<String>,
}

/// 认证信息
#[derive(Debug, Clone)]
pub struct AuthInfo {
    /// 认证类型
    pub auth_type: AuthProviderType,
    /// 用户ID
    pub user_id: String,
    /// 用户名
    pub username: Option<String>,
    /// 权限
    pub scopes: Vec<String>,
    /// 额外信息
    pub claims: BTreeMap<String, String>,
}

/// 限流器
pub struct RateLimiter {
    /// 全局限流器
    global_limiter: TokenBucket,
    /// 服务限流器
    service_limiters: BTreeMap<String, TokenBucket>,
    /// API限流器
    api_limiters: BTreeMap<String, TokenBucket>,
    /// 算法
    algorithm: RateLimitAlgorithm,
}

/// 令牌桶
pub struct TokenBucket {
    /// 容量
    pub capacity: u64,
    /// 令牌数
    pub tokens: u64,
    /// 填充速率(令牌/秒)
    pub refill_rate: u64,
    /// 最后填充时间
    pub last_refill: u64,
}

/// 认证器
pub struct Authenticator {
    /// 认证提供者
    pub providers: Vec<AuthProvider>,
    /// 默认策略
    pub default_policy: AuthPolicy,
    /// JWT验证器
    pub jwt_validator: Option<JwtValidator>,
    /// API Key验证器
    pub api_key_validator: Option<ApiKeyValidator>,
}

/// JWT验证器
pub struct JwtValidator {
    /// 公钥
    pub public_key: String,
    /// Issuer
    pub issuer: String,
    /// Audience
    pub audience: String,
}

/// API Key验证器
pub struct ApiKeyValidator {
    /// API Keys
    pub api_keys: BTreeMap<String, ApiKeyInfo>,
    /// Key位置
    pub key_location: ApiKeyLocation,
    /// Key名称
    pub key_name: String,
}

/// API Key信息
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    /// Key名称
    pub name: String,
    /// 关联的服务
    pub service: String,
    /// 权限
    pub scopes: Vec<String>,
    /// 速率限制
    pub rate_limit: Option<RateLimit>,
    /// 过期时间
    pub expires_at: Option<u64>,
}

/// Gateway统计信息
#[derive(Debug, Clone)]
pub struct GatewayStats {
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 被拒绝的请求数(认证/限流)
    pub rejected_requests: u64,
    /// 平均延迟(毫秒)
    pub average_latency_ms: u64,
    /// P99延迟(毫秒)
    pub p99_latency_ms: u64,
    /// 当前连接数
    pub active_connections: u64,
    /// 字节数
    pub bytes_sent: u64,
    pub bytes_received: u64,
    /// 按状态码统计
    pub status_code_counts: BTreeMap<u16, u64>,
    /// 按API统计
    pub api_stats: BTreeMap<String, ApiStats>,
}

/// API统计
#[derive(Debug, Clone)]
pub struct ApiStats {
    /// API名称
    pub api_name: String,
    /// 请求数
    pub request_count: u64,
    /// 成功数
    pub success_count: u64,
    /// 失败数
    pub error_count: u64,
    /// 平均延迟(毫秒)
    pub avg_latency_ms: u64,
    /// P99延迟(毫秒)
    pub p99_latency_ms: u64,
}

impl ApiGateway {
    /// 创建新的API Gateway
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            routes: BTreeMap::new(),
            apis: BTreeMap::new(),
            plugins: Vec::new(),
            rate_limiter: Arc::new(Mutex::new(RateLimiter {
                global_limiter: TokenBucket {
                    capacity: 20000,
                    tokens: 20000,
                    refill_rate: 10000,
                    last_refill: 0,
                },
                service_limiters: BTreeMap::new(),
                api_limiters: BTreeMap::new(),
                algorithm: RateLimitAlgorithm::TokenBucket,
            })),
            authenticator: Arc::new(Mutex::new(Authenticator {
                providers: Vec::new(),
                default_policy: AuthPolicy::AuthenticationRequired,
                jwt_validator: None,
                api_key_validator: None,
            })),
            stats: Arc::new(Mutex::new(GatewayStats {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                rejected_requests: 0,
                average_latency_ms: 0,
                p99_latency_ms: 0,
                active_connections: 0,
                bytes_sent: 0,
                bytes_received: 0,
                status_code_counts: BTreeMap::new(),
                api_stats: BTreeMap::new(),
            })),
            next_api_id: AtomicU64::new(1),
        }
    }

    /// 初始化Gateway
    pub fn initialize(&mut self) -> Result<(), i32> {
        crate::println!("[gateway] Initializing API gateway: {}", self.config.gateway_name);

        // 初始化认证器
        self.initialize_authenticator()?;

        // 初始化限流器
        self.initialize_rate_limiter()?;

        crate::println!("[gateway] API gateway initialized successfully");
        Ok(())
    }

    /// 初始化认证器
    fn initialize_authenticator(&mut self) -> Result<(), i32> {
        let mut auth = self.authenticator.lock();
        auth.providers = self.config.auth_config.providers.clone();
        auth.default_policy = self.config.auth_config.default_policy;

        // 初始化JWT验证器
        if let Some(ref jwt_config) = self.config.auth_config.jwt_config {
            auth.jwt_validator = Some(JwtValidator {
                public_key: jwt_config.public_key.clone(),
                issuer: jwt_config.issuer.clone(),
                audience: jwt_config.audience.clone(),
            });
        }

        // 初始化API Key验证器
        if let Some(ref api_key_config) = self.config.auth_config.api_key_config {
            auth.api_key_validator = Some(ApiKeyValidator {
                api_keys: BTreeMap::new(),
                key_location: api_key_config.key_location,
                key_name: api_key_config.key_name.clone(),
            });
        }

        crate::println!("[gateway] Authenticator initialized");
        Ok(())
    }

    /// 初始化限流器
    fn initialize_rate_limiter(&mut self) -> Result<(), i32> {
        let mut rate_limiter = self.rate_limiter.lock();
        rate_limiter.algorithm = self.config.rate_limit_config.algorithm;

        // 初始化服务限流器
        for (service, limit) in &self.config.rate_limit_config.service_limits {
            rate_limiter.service_limiters.insert(
                service.clone(),
                TokenBucket {
                    capacity: limit.burst,
                    tokens: limit.burst,
                    refill_rate: limit.requests_per_second,
                    last_refill: self.get_current_time_ms(),
                },
            );
        }

        crate::println!("[gateway] Rate limiter initialized");
        Ok(())
    }

    /// 注册API
    pub fn register_api(&mut self, api: ApiDefinition) -> Result<String, i32> {
        let api_id = api.id.clone();
        self.apis.insert(api_id.clone(), api);

        crate::println!("[gateway] Registered API: {}", api_id);
        Ok(api_id)
    }

    /// 添加路由
    pub fn add_route(&mut self, route: Route) -> Result<(), i32> {
        let route_id = route.id.clone();
        self.routes.insert(route_id.clone(), route);

        crate::println!("[gateway] Added route: {}", route_id);
        Ok(())
    }

    /// 处理请求
    pub fn handle_request(&mut self, mut context: RequestContext) -> Result<ResponseContext, i32> {
        let start_time = self.get_current_time_ms();
        context.start_time = start_time;

        // 更新统计信息
        {
            let mut stats = self.stats.lock();
            stats.total_requests += 1;
            stats.active_connections += 1;
        }

        // 1. 限流检查
        self.check_rate_limit(&context)?;

        // 2. 认证检查
        self.authenticate(&mut context)?;

        // 3. 提取API版本
        self.extract_api_version(&mut context);

        // 4. 路由匹配
        let route = self.match_route(&context)?;

        // 5. 执行请求插件
        for plugin in &mut self.plugins {
            plugin.on_request(&mut context)?;
        }

        // 6. 转发请求到后端服务
        let response = self.forward_request(&context, &route)?;

        // 7. 执行响应插件
        let mut response_context = ResponseContext {
            request_id: context.request_id.clone(),
            status_code: response.status_code,
            headers: response.headers,
            body: response.body,
            latency_ms: self.get_current_time_ms() - start_time,
            upstream_address: Some(format!("{}:{}", route.target_service, route.target_port)),
            error: None,
        };

        for plugin in &mut self.plugins {
            plugin.on_response(&mut response_context)?;
        }

        // 更新统计信息
        self.update_stats(&context, &response_context);

        {
            let mut stats = self.stats.lock();
            stats.active_connections -= 1;
        }

        Ok(response_context)
    }

    /// 检查限流
    fn check_rate_limit(&self, _context: &RequestContext) -> Result<(), i32> {
        let mut rate_limiter = self.rate_limiter.lock();

        // 检查全局限流
        if !self.consume_tokens(&mut rate_limiter.global_limiter, 1) {
            crate::println!("[gateway] Global rate limit exceeded");
            return Err(EIO); // 返回429
        }

        // 检查服务限流(简化实现)
        // 在实际实现中,应该从路由中提取服务名并检查对应限流器

        // 检查API限流(简化实现)
        // 在实际实现中,应该从路由中提取API名并检查对应限流器

        Ok(())
    }

    /// 消费令牌
    fn consume_tokens(&self, bucket: &mut TokenBucket, tokens: u64) -> bool {
        let now = self.get_current_time_ms();

        // 填充令牌
        let elapsed = now - bucket.last_refill;
        if elapsed >= 1000 {
            let refill = (elapsed / 1000) * bucket.refill_rate;
            bucket.tokens = (bucket.tokens + refill).min(bucket.capacity);
            bucket.last_refill = now;
        }

        // 消费令牌
        if bucket.tokens >= tokens {
            bucket.tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// 认证检查
    fn authenticate(&self, context: &mut RequestContext) -> Result<(), i32> {
        let auth = self.authenticator.lock();

        // 如果无需认证,直接通过
        if auth.default_policy == AuthPolicy::None {
            return Ok(());
        }

        // 尝试JWT认证
        if let Some(ref jwt_validator) = auth.jwt_validator {
            if self.authenticate_jwt(jwt_validator, context).is_ok() {
                return Ok(());
            }
        }

        // 尝试API Key认证
        if let Some(ref api_key_validator) = auth.api_key_validator {
            if self.authenticate_api_key(api_key_validator, context).is_ok() {
                return Ok(());
            }
        }

        // 如果是可选认证,也通过
        if auth.default_policy == AuthPolicy::AuthenticationOptional {
            return Ok(());
        }

        crate::println!("[gateway] Authentication failed");
        Err(EIO) // 返回401
    }

    /// JWT认证
    fn authenticate_jwt(&self, _validator: &JwtValidator, _context: &mut RequestContext) -> Result<(), i32> {
        // 简化实现
        // 在实际实现中,应该解析JWT token并验证签名和claims
        Ok(())
    }

    /// API Key认证
    fn authenticate_api_key(
        &self,
        _validator: &ApiKeyValidator,
        _context: &mut RequestContext,
    ) -> Result<(), i32> {
        // 简化实现
        // 在实际实现中,应该从请求中提取API Key并验证
        Ok(())
    }

    /// 提取API版本
    fn extract_api_version(&self, context: &mut RequestContext) {
        match self.config.versioning.versioning_strategy {
            VersioningStrategy::UrlPath => {
                // 从路径中提取版本
                if let Some(version) = context.path.split('/').nth(1) {
                    if version.starts_with('v') {
                        context.api_version = Some(version.to_string());
                    }
                }
            },
            VersioningStrategy::Header => {
                // 从头部提取版本
                if let Some(version) = context.headers.get("X-API-Version") {
                    context.api_version = Some(version.clone());
                }
            },
            VersioningStrategy::Query => {
                // 从查询参数提取版本
                if let Some(version) = context.query_params.get("version") {
                    context.api_version = Some(version.clone());
                }
            },
            VersioningStrategy::ContentType => {
                // 从Content-Type提取版本
                if let Some(ct) = context.headers.get("Content-Type") {
                    if ct.contains("v=") {
                        if let Some(version) = ct.split("v=").nth(1) {
                            context.api_version = Some(version.split(';').next().unwrap_or("v1").to_string());
                        }
                    }
                }
            },
        }

        // 如果没有提取到版本,使用默认版本
        if context.api_version.is_none() {
            context.api_version = Some(self.config.versioning.default_version.clone());
        }
    }

    /// 路由匹配
    fn match_route(&self, context: &RequestContext) -> Result<Route, i32> {
        for route in self.routes.values() {
            if !route.enabled {
                continue;
            }

            // 检查方法
            if !route.methods.contains(&context.method) {
                continue;
            }

            // 检查路径
            let path_matches = match route.path_match_type {
                PathMatchType::Exact => context.path == route.path,
                PathMatchType::Prefix => context.path.starts_with(&route.path),
                PathMatchType::Regex => {
                    // 简化实现:使用前缀匹配代替正则
                    context.path.starts_with(&route.path)
                },
            };

            if path_matches {
                return Ok(route.clone());
            }
        }

        crate::println!("[gateway] No matching route found for: {}", context.path);
        Err(ENOENT) // 返回404
    }

    /// 转发请求
    fn forward_request(&self, _context: &RequestContext, _route: &Route) -> Result<Response, i32> {
        // 简化实现
        // 在实际实现中,应该建立到后端服务的连接并发送请求
        Ok(Response {
            status_code: 200,
            headers: {
                let mut headers = BTreeMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers
            },
            body: Some(b"{\"status\":\"ok\"}".to_vec()),
        })
    }

    /// 更新统计信息
    fn update_stats(&self, context: &RequestContext, response: &ResponseContext) {
        let mut stats = self.stats.lock();

        if response.status_code < 400 {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        // 更新状态码统计
        *stats.status_code_counts.entry(response.status_code).or_insert(0) += 1;

        // 更新平均延迟
        stats.average_latency_ms =
            (stats.average_latency_ms * (stats.total_requests - 1) + response.latency_ms) / stats.total_requests;

        // 更新字节数
        if let Some(ref body) = context.body {
            stats.bytes_received += body.len() as u64;
        }
        if let Some(ref body) = response.body {
            stats.bytes_sent += body.len() as u64;
        }

        // 更新API统计
        // 在实际实现中,应该根据路由更新对应API的统计
    }

    /// 添加插件
    pub fn add_plugin(&mut self, plugin: Box<dyn GatewayPlugin>) {
        crate::println!("[gateway] Added plugin: {}", plugin.name());
        self.plugins.push(plugin);
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> GatewayStats {
        self.stats.lock().clone()
    }

    /// 获取当前时间(毫秒)
    fn get_current_time_ms(&self) -> u64 {
        crate::subsystems::time::rdtsc() as u64 / 1000000
    }
}

/// 响应
#[derive(Debug, Clone)]
pub struct Response {
    /// 状态码
    pub status_code: u16,
    /// 头部
    pub headers: BTreeMap<String, String>,
    /// 主体
    pub body: Option<Vec<u8>>,
}

/// 全局API Gateway实例
static mut API_GATEWAY: Option<ApiGateway> = None;
static mut API_GATEWAY_INITIALIZED: bool = false;

/// 初始化API Gateway
pub fn init_api_gateway(config: GatewayConfig) -> Result<(), i32> {
    if unsafe { API_GATEWAY_INITIALIZED } {
        return Ok(());
    }

    let mut gateway = ApiGateway::new(config);
    gateway.initialize()?;

    unsafe {
        API_GATEWAY = Some(gateway);
        API_GATEWAY_INITIALIZED = true;
    }

    crate::println!("[gateway] API gateway initialized");
    Ok(())
}

/// 获取API Gateway引用
pub fn get_api_gateway() -> Option<&'static mut ApiGateway> {
    unsafe { API_GATEWAY.as_mut() }
}
