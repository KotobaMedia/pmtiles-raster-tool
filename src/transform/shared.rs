use anyhow::Result;
use bytes::Bytes;

pub trait TransformProcess: Send + Sync + Clone {
    fn new() -> Self
    where
        Self: Sized;
    fn transform(&self, input: &[u8]) -> Result<Bytes>;
}
