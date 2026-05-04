const MS_PER_HOUR: f64 = 3_600_000.0;

pub struct TokenBucket {
    pub tokens: f64,
    pub capacity: f64,
    pub refill_per_hour: f64,
    pub refilled_at_ms: i64,
}

impl TokenBucket {
    pub fn new(capacity: f64, refill_per_hour: f64, now_ms: i64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_per_hour,
            refilled_at_ms: now_ms,
        }
    }

    fn refill(&mut self, now_ms: i64) {
        let elapsed = (now_ms - self.refilled_at_ms).max(0) as f64;
        if elapsed <= 0.0 {
            return;
        }
        let added = elapsed * (self.refill_per_hour / MS_PER_HOUR);
        self.tokens = (self.tokens + added).min(self.capacity);
        self.refilled_at_ms = now_ms;
    }

    pub fn try_take(&mut self, now_ms: i64) -> Result<(), u64> {
        self.refill(now_ms);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            return Ok(());
        }
        let needed = 1.0 - self.tokens;
        let ms_to_one = needed * (MS_PER_HOUR / self.refill_per_hour);
        Err(ms_to_one.ceil() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_consumes_until_empty() {
        let mut bucket = TokenBucket::new(3.0, 30.0, 0);
        assert!(bucket.try_take(0).is_ok());
        assert!(bucket.try_take(0).is_ok());
        assert!(bucket.try_take(0).is_ok());
        let err = bucket.try_take(0).unwrap_err();
        assert!(err > 0);
    }

    #[test]
    fn bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(3.0, 3600.0, 0);
        bucket.try_take(0).unwrap();
        bucket.try_take(0).unwrap();
        bucket.try_take(0).unwrap();
        assert!(bucket.try_take(0).is_err());
        // 3600/h = 1/sec; after 1000ms one token returns.
        assert!(bucket.try_take(1000).is_ok());
    }

    #[test]
    fn bucket_caps_at_capacity() {
        let mut bucket = TokenBucket::new(3.0, 3600.0, 0);
        bucket.try_take(0).unwrap();
        bucket.try_take(0).unwrap();
        bucket.try_take(0).unwrap();
        // Wait 10s → would refill 10 tokens but capped at 3.
        bucket.try_take(10_000).unwrap();
        bucket.try_take(10_000).unwrap();
        bucket.try_take(10_000).unwrap();
        assert!(bucket.try_take(10_000).is_err());
    }
}
