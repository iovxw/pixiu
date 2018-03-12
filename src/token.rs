use std::time::{Duration, Instant};

use rand::{self, Rng};

pub struct UnverifiedTokenCache {
    timeout: Duration,
    cache: Vec<UnverifiedToken>,
}

impl UnverifiedTokenCache {
    pub fn new(timeout: Duration) -> UnverifiedTokenCache {
        UnverifiedTokenCache {
            timeout: timeout,
            cache: Vec::new(),
        }
    }

    pub fn generate(&mut self, user: u64, username: String) -> u64 {
        let token = UnverifiedToken::new(user, username);
        let r = token.value;
        self.cache.push(token);
        self.clean_up();
        r
    }

    pub fn verify(&mut self, user: u64, token: u64) -> Option<String> {
        self.clean_up();
        if let Ok(i) = self.cache
            .binary_search_by_key(&(user, token), |v| (v.user, v.value))
        {
            Some(self.cache.remove(i).username)
        } else {
            None
        }
    }

    fn clean_up(&mut self) {
        if !self.cache.is_empty() {
            let now = Instant::now();
            for i in (0..self.cache.len()).rev() {
                if (self.cache[i].created + self.timeout) < now {
                    self.cache.remove(i);
                } else {
                    break;
                }
            }
        }
    }
}

struct UnverifiedToken {
    created: Instant,
    user: u64,
    username: String,
    value: u64,
}

impl UnverifiedToken {
    fn new(user: u64, username: String) -> UnverifiedToken {
        UnverifiedToken {
            created: Instant::now(),
            user: user,
            username: username,
            value: rand::thread_rng().gen(),
        }
    }
}

impl ::std::ops::Deref for UnverifiedToken {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[test]
fn unverified_token_cache() {
    use super::*;

    let mut cache = UnverifiedTokenCache::new(Duration::from_secs(10));
    let token = cache.generate(1000,"username".into());
    assert_eq!(cache.verify(1000, token), Some("username".into()));
    assert!(cache.cache.is_empty());
}

#[test]
fn unverified_token_cache_timeout() {
    use super::*;

    let mut cache = UnverifiedTokenCache::new(Duration::from_millis(1));
    let token = cache.generate(1000,"username".into());
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(cache.verify(1000, token), None);
    assert!(cache.cache.is_empty());
}
