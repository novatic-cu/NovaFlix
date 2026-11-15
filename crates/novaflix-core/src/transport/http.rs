use std::time::Duration;

use futures::StreamExt;
use reqwest::StatusCode;

use super::Download;
use crate::{
    catalog::{self, RawEntry},
    error::{Error, Result},
};

pub async fn fetch_text(http: &reqwest::Client, url: &str) -> Result<String> {
    let mut last_error = String::from("sin intentos");
    for attempt in 0..3 {
        match http.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                let bytes = response.bytes().await?;
                return Ok(catalog::decode_body(&bytes));
            }
            Ok(response) => last_error = format!("HTTP {}", response.status()),
            Err(err) => last_error = err.to_string(),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1))).await;
    }
    Err(Error::Net(last_error))
}

/// Lista un directorio remoto leyendo el índice de Apache.
pub async fn list(http: &reqwest::Client, base: &str, path: &str) -> Result<Vec<RawEntry>> {
    let mut target = path.to_string();
    if !target.ends_with('/') {
        target.push('/');
    }
    let html = fetch_text(http, &catalog::absolute_url_with(base, &target)).await?;
    Ok(parse(&html))
}

fn parse(html: &str) -> Vec<RawEntry> {
    catalog::parse_listing(html)
}

pub async fn open(http: &reqwest::Client, base: &str, path: &str, offset: u64) -> Result<Download> {
    let url = catalog::absolute_url_with(base, path);
    let mut request = http.get(&url).timeout(Duration::from_secs(60 * 60 * 24));
    if offset > 0 {
        request = request.header("Range", format!("bytes={offset}-"));
    }
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() && status != StatusCode::PARTIAL_CONTENT {
        return Err(Error::Net(format!("HTTP {status}")));
    }
    let resumed = status == StatusCode::PARTIAL_CONTENT;
    let start = if resumed { offset } else { 0 };
    let total = response.content_length().map(|length| length + start);
    let stream = response
        .bytes_stream()
        .map(|chunk| chunk.map_err(Error::from))
        .boxed();
    Ok(Download {
        total,
        resumed,
        stream,
    })
}
