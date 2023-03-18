use std::{
    thread,
    time::{self, Instant},
};

fn main() -> anyhow::Result<()> {
    let mut limiter = TokenBucket::new(time::Duration::from_secs(3600), 60);
    for _ in 0..=10 {
        thread::sleep(time::Duration::from_secs(1));
        println!("{}", limiter.allowed());
    }

    thread::sleep(time::Duration::from_secs(6));
    for _ in 0..=10 {
        thread::sleep(time::Duration::from_millis(10));
        println!("{}", limiter.allowed());
    }

    Ok(())
}

trait RateLimiter {
    fn new(window: time::Duration, limit: usize) -> Self;
    fn allowed(&mut self) -> bool;
}

struct FixedWindow {
    window_start: Instant,
    hits: usize,
    window: time::Duration,
    limit: usize,
}

impl RateLimiter for FixedWindow {
    fn new(window: time::Duration, limit: usize) -> Self {
        FixedWindow {
            window_start: Instant::now(),
            hits: 0,
            window,
            limit,
        }
    }

    fn allowed(&mut self) -> bool {
        let now = Instant::now();

        if now.duration_since(self.window_start) > self.window {
            self.window_start = now;
            self.hits = 0;
        };

        if self.hits >= self.limit {
            return false;
        };

        self.hits = self.hits + 1;
        true
    }
}

struct MovingWindow {
    prev_start: Instant,
    prev_count: usize,
    this_start: Instant,
    this_count: usize,
    window: time::Duration,
    limit: usize,
}

impl RateLimiter for MovingWindow {
    fn new(window: time::Duration, limit: usize) -> Self {
        let now = Instant::now();

        MovingWindow {
            prev_start: now,
            prev_count: 0,
            this_start: now,
            this_count: 0,
            window,
            limit,
        }
    }

    fn allowed(&mut self) -> bool {
        let now = Instant::now();

        // Cycle the current window values into the previous window repeatedly until we "catch up"
        // to the present time. In cases where more than two windows duration have passed since the
        // start of this window period this will cycle through twice and essentially reset the
        // counter.
        while now.duration_since(self.this_start) > self.window {
            self.prev_start = self.this_start;
            self.prev_count = self.this_count;
            self.this_start = self.prev_start + self.window;
            self.this_count = 0;
        }

        let this_period = now.duration_since(self.this_start);
        let last_period = self.window - this_period;

        let hits_from_last_period =
            (self.prev_count * last_period.as_micros() as usize) / self.window.as_micros() as usize;

        if self.this_count + hits_from_last_period >= self.limit {
            return false;
        }

        self.this_count = self.this_count + 1;

        true
    }
}

struct TokenBucket {
    tokens: usize,
    last_hit: Instant,
    window: time::Duration,
    limit: usize,
}

impl TokenBucket {
    fn new_tokens(&self, now: Instant) -> usize {
        // Calculate the number of new tokens that should be accumlated based on the provided time.
        // This is the time elapsed since the last token calculation times the rate of token
        // accumulation.
        let elapsed = now.duration_since(self.last_hit);

        // Rate as tokens per microsecond, which will probably be a very small number.
        let rate = self.limit as f64 / self.window.as_micros() as f64;

        // Microseconds elapsed * tokens per microsecond yields tokens to output.
        (elapsed.as_micros() as f64 * rate) as usize
    }
}

impl RateLimiter for TokenBucket {
    fn new(window: time::Duration, limit: usize) -> Self {
        TokenBucket {
            tokens: 0,
            last_hit: Instant::now(),
            window,
            limit,
        }
    }

    fn allowed(&mut self) -> bool {
        let now = Instant::now();

        // Accumulate tokens at the rate of limit / window (tokens per time)
        let new_tokens = self.new_tokens(now);

        // Only adjust the last hit time if at least one token was accumulated.
        if new_tokens > 0 {
            // Limit tokens to self.limit
            self.tokens = std::cmp::min(self.tokens + new_tokens as usize, self.limit);
            self.last_hit = now; // Based on accumulation of tokens
        }

        if self.tokens == 0 {
            return false;
        }

        self.tokens = self.tokens - 1;

        true
    }
}
