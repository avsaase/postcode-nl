//! Async client for the free Netherlands postcode API at <https://postcode.tech>.
//!
//! # Example
//! ```rust,no_run
//! # use std::error::Error;
//! # use postcode_nl::*;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn Error>> {
//! // Initialize a client
//! let client: PostcodeClient = PostcodeClient::new("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx");
//!
//! // Find the address matching on a postcode and house number
//! let (address, limits): (Address, ApiLimits) = client.get_address("1012RJ", 147).await?;
//!
//! // Find the address and additional location information such as municipality, province and coordinates
//! let (address, limits): (ExtendedAddress, ApiLimits) = client.get_extended_address("1012RJ", 147).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Usage Limits
//! As of the latest release of this crate, API usage is limited to 10,000 requests per day as well as a 600 request rate limit over an unspecified time window. The library validates inputs in order to avoid making requests with invalid inputs, which count towards the usage limits. [`ApiLimits`], included with the address response, reports the current API limits.
//!
//! # Disclaimer
//! I am not affiliated with the API provider and as such cannot make guarantees to the correctness of the results or the availability of the underlying service. Refer to <https://postcode.tech> for the service terms and conditions.

use regex::Regex;
use reqwest::Client;
use rest::{call_api, Geo, PostcodeApiFullResponse, PostcodeApiSimpleResponse};
use thiserror::Error;

mod rest;

/// The client that calls the API.
pub struct PostcodeClient {
    api_token: String,
    client: Client,
}

/// Simple address response.
#[derive(Debug, Clone)]
pub struct Address {
    pub street: String,
    pub house_number: u32,
    pub postcode: String,
    pub city: String,
}

/// Extended address response.
#[derive(Debug, Clone)]
pub struct ExtendedAddress {
    pub street: String,
    pub house_number: u32,
    pub postcode: String,
    pub city: String,
    pub municipality: String,
    pub province: String,
    pub coordinates: Coordinates,
}

/// Coordinates of the address
#[derive(Debug, Clone)]
pub struct Coordinates {
    pub lat: f32,
    pub lon: f32,
}

/// Usage limits of the API, returned with every request
#[derive(Debug, Clone)]
pub struct ApiLimits {
    pub ratelimit_limit: u32,
    pub ratelimit_remaining: u32,
    pub api_limit: u32,
    pub api_remaining: u32,
    pub api_reset: String,
}

impl PostcodeClient {
    /// Initialize a new client with an API token.
    /// ```rust,no_run
    /// # use std::error::Error;
    /// # use postcode_nl::*;
    /// # fn main()  {
    /// let client: PostcodeClient = PostcodeClient::new("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx");
    /// # }
    /// ```
    pub fn new(api_token: &str) -> Self {
        let client = Client::new();

        Self {
            api_token: api_token.to_string(),
            client,
        }
    }

    /// Find the address matching the given postcode and house number. Postcodes are formatted 1234AB or 1234 AB (with a single space). House numbers must be integers and not include postfix characters.
    /// ```rust,no_run
    /// # use std::error::Error;
    /// # use postcode_nl::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn Error>> {
    /// # let client: PostcodeClient = PostcodeClient::new("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx");
    /// let (address, limits): (Address, ApiLimits) = client.get_address("1012RJ", 147).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_address(&self, postcode: &str, house_number: u32) -> Result<(Address, ApiLimits), PostcodeError> {
        let postcode = Self::validate_postcode_input(postcode)?;

        let response = call_api(&self.client, &self.api_token, postcode, house_number, false).await?;

        let limits = response.headers().try_into()?;

        let result = response
            .json::<PostcodeApiSimpleResponse>()
            .await
            .map_err(|e| PostcodeError::InvalidApiResponse(format! {"Failed to deserialize API response, {e}"}))?
            .into_internal(postcode, house_number);

        Ok((result, limits))
    }

    /// Find the address, municipality, province and coordinates matching the given postcode and house number. Postcodes are formatted 1234AB or 1234 AB (with a single space). House numbers must be integers and not include postfix characters.
    /// ```rust,no_run
    /// # use std::error::Error;
    /// # use postcode_nl::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn Error>> {
    /// # let client: PostcodeClient = PostcodeClient::new("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx");
    /// let (address, limits): (ExtendedAddress, ApiLimits) = client.get_extended_address("1012RJ", 147).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_extended_address(
        &self,
        postcode: &str,
        house_number: u32,
    ) -> Result<(ExtendedAddress, ApiLimits), PostcodeError> {
        let postcode = Self::validate_postcode_input(postcode)?;

        let response = call_api(&self.client, &self.api_token, postcode, house_number, true).await?;

        let limits = response.headers().try_into()?;

        let result = response
            .json::<PostcodeApiFullResponse>()
            .await
            .map_err(|e| PostcodeError::InvalidApiResponse(format! {"Failed to deserialize API response, {e}"}))?
            .into();

        Ok((result, limits))
    }

    fn validate_postcode_input(postcode: &str) -> Result<&str, PostcodeError> {
        let postcode_pattern = Regex::new(r"^\d{4} {0,1}[a-zA-Z]{2}$").unwrap();
        if postcode_pattern.is_match(postcode) {
            Ok(postcode)
        } else {
            Err(PostcodeError::InvalidInput(format!(
                "Postcodes should be formatted as `1234AB` or `1234 AB`, input: {postcode}"
            )))
        }
    }
}

/// Possible errors when fetching an address.
#[derive(Debug, Error)]
pub enum PostcodeError {
    /// The supplied postcode does not have the correct format: 1234AB or 1234 AB (with one space).
    #[error("Invalid input")]
    InvalidInput(String),
    /// The API did not respond to the request.
    #[error("Did not get response from API")]
    NoApiResponse(String),
    /// The API response body could not be parsed.
    #[error("Failed to parse API response")]
    InvalidApiResponse(String),
    /// The API responded that the inputs are incorrect. This should not happen and instead [`PostcodeError::InvalidInput`] should be returned.
    #[error("API returned input is invalid")]
    InvalidData(String),
    /// No address could be found for the given postcode and house number combination.
    #[error("Postcode not found")]
    NotFound(String),
    /// The API returned an unexpected error code.
    #[error("API returned an error")]
    OtherApiError(String),
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
