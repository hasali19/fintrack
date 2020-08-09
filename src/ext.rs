use crate::Request;

pub trait RequestExt {
    fn url_for(&self, path: &str) -> anyhow::Result<String>;
}

impl RequestExt for Request {
    fn url_for(&self, path: &str) -> anyhow::Result<String> {
        let req_url = self.url();
        Ok(format!(
            "{}://{}/{}",
            req_url.scheme(),
            req_url
                .host_str()
                .ok_or_else(|| anyhow::anyhow!("failed to get request url"))?,
            path
        ))
    }
}
