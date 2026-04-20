use governor::clock::DefaultClock;
use governor::state::keyed::DashMapStateStore;
use governor::{Quota, RateLimiter};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

pub type RegisterRateLimiter = Arc<RateLimiter<IpAddr, DashMapStateStore<IpAddr>, DefaultClock>>;

pub fn new_register_rate_limiter() -> RegisterRateLimiter {
    // 100 requests per 60 seconds / ip
    let quota = Quota::per_minute(NonZeroU32::new(100).unwrap());

    Arc::new(RateLimiter::keyed(quota))
}
