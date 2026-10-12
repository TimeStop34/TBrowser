use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Response {
    pub status_line: String,
    pub status_code: u16,
    pub status_name: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub text: String,
}

impl Response {
    pub fn ok(&self) -> bool {
        (20..30).contains(&self.status_code)
    }
}

pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

pub struct FetchOptions {
    pub follow_redirects: bool,
    pub max_redirects: u32,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            follow_redirects: true,
            max_redirects: 10,
        }
    }
}

#[async_trait]
pub trait TextProtocol: Send + Sync {
    async fn fetch(
        &self,
        url: &str,
        progress: Option<&ProgressCallback>,
        options: &FetchOptions,
    ) -> Result<Response>;
}

#[async_trait]
pub trait PostCompatibleProtocol: TextProtocol {
    #[allow(unused)]
    async fn post(
        &self,
        url: &str,
        data: &[(&str, &str)],
        progress: Option<&ProgressCallback>,
    ) -> Result<Response>;
}
