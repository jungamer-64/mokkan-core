// src/presentation/http/middleware/rate_limit.rs
use ::governor::middleware::NoOpMiddleware;
use axum::body::Body;
use std::sync::OnceLock;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};

pub fn rate_limit_layer() -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, Body> {
    static RATE_LIMITER: OnceLock<GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, Body>> =
        OnceLock::new();

    RATE_LIMITER
        .get_or_init(|| {
            let mut builder = GovernorConfigBuilder::default();
            builder.per_second(10);
            builder.burst_size(20);
            let config = builder
                .key_extractor(SmartIpKeyExtractor)
                .finish()
                .expect("valid rate limit configuration");

            GovernorLayer::new(config)
        })
        .clone()
}
