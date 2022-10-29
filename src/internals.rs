use reqwest::{header::HeaderMap, Client, Response, StatusCode, Url};
use serde::Deserialize;

use crate::{Address, ApiLimits, Coordinates, ExtendedAddress, PostcodeError};

pub const API_URL_SIMPLE: &str = "https://postcode.tech/api/v1/postcode";
pub const API_URL_FULL: &str = "https://postcode.tech/api/v1/postcode/full";

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct PostcodeApiSimpleResponse {
    pub street: String,
    pub city: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PostcodeApiFullResponse {
    pub postcode: String,
    pub number: u32,
    pub street: String,
    pub city: String,
    pub municipality: String,
    pub province: String,
    pub geo: Geo,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Geo {
    pub lat: f32,
    pub lon: f32,
}

pub(crate) async fn call_api(
    client: &Client,
    token: &str,
    postcode: &str,
    house_number: u32,
    full: bool,
) -> Result<Response, PostcodeError> {
    let url = if full { API_URL_FULL } else { API_URL_SIMPLE };
    let url = Url::parse_with_params(url, &[("postcode", postcode), ("number", &house_number.to_string())]).unwrap();

    let response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| PostcodeError::NoApiResponse(format!("Error contacting API, {e}")))?;

    match response.status() {
        StatusCode::OK => (),
        StatusCode::NOT_FOUND => (), // This is not an error, it just means the address was not found
        StatusCode::TOO_MANY_REQUESTS => return Err(PostcodeError::TooManyRequests("API limits exceeded".to_string())),
        _ => {
            return Err(PostcodeError::OtherApiError(format!(
                "Received error from API, code: {}, {}",
                response.status(),
                response.text().await.unwrap()
            )))
        }
    }

    Ok(response)
}

impl TryFrom<&HeaderMap> for ApiLimits {
    type Error = PostcodeError;

    fn try_from(headers: &HeaderMap) -> Result<Self, PostcodeError> {
        let ratelimit_limit = extract_header_u32(headers, "x-ratelimit-limit")?;
        let ratelimit_remaining = extract_header_u32(headers, "x-ratelimit-remaining")?;
        let api_limit = extract_header_u32(headers, "x-api-limit")?;
        let api_remaining = extract_header_u32(headers, "x-api-remaining")?;
        let api_reset = extract_header_string(headers, "x-api-reset")?;

        Ok(Self {
            ratelimit_limit,
            ratelimit_remaining,
            api_limit,
            api_remaining,
            api_reset,
        })
    }
}

fn extract_header_u32(headers: &HeaderMap, header_key: &str) -> Result<u32, PostcodeError> {
    let value = headers
        .get(header_key)
        .ok_or_else(|| PostcodeError::InvalidApiResponse("API did not return API limits".to_string()))?
        .to_str()
        .map_err(|_e| PostcodeError::InvalidApiResponse("Failed to parse API rate limit from header".to_string()))?
        .parse::<u32>()
        .map_err(|_e| PostcodeError::InvalidApiResponse("Failed to parse API rate limit from header".to_string()))?;

    Ok(value)
}

fn extract_header_string(headers: &HeaderMap, header_key: &str) -> Result<String, PostcodeError> {
    let value = headers
        .get(header_key)
        .ok_or_else(|| PostcodeError::InvalidApiResponse("API did not return API limits".to_string()))?
        .to_str()
        .map_err(|_e| PostcodeError::InvalidApiResponse("Failed to parse API reset frequency from header".to_string()))?
        .to_string();

    Ok(value)
}

pub(crate) trait IntoInternal<T> {
    fn into_internal(self, postcode: &str, house_number: u32) -> T;
}

impl IntoInternal<Address> for PostcodeApiSimpleResponse {
    fn into_internal(self, postcode: &str, house_number: u32) -> Address {
        Address {
            street: self.street,
            house_number,
            postcode: postcode.to_string(),
            city: self.city,
        }
    }
}

impl From<PostcodeApiFullResponse> for ExtendedAddress {
    fn from(p: PostcodeApiFullResponse) -> Self {
        Self {
            street: p.street,
            house_number: p.number,
            postcode: p.postcode,
            city: p.city,
            municipality: p.municipality,
            province: p.province,
            coordinates: p.geo.into(),
        }
    }
}

impl From<Geo> for Coordinates {
    fn from(g: Geo) -> Self {
        Self { lat: g.lat, lon: g.lon }
    }
}
