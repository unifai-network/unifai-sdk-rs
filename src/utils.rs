use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};

pub fn build_api_client(api_key: &str) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    headers.insert("Authorization", HeaderValue::from_str(api_key).unwrap());

    Client::builder().default_headers(headers).build().unwrap()
}
