//! Async client for the free Netherlands postcode API at <https://postcode.tech>.
//!
//! There are two methods, one to find the street and city matching the supplied postcode and house number, and one that also includes the municipality, province and coordinates. If no address can be found for the postcode and house number combination, `None` is returned.
//!
//! # Example
//! ```rust,no_run
//! # use std::error::Error;
//! # use postcode_nl::*;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn Error>> {
//! // Initialize a client
//! let client = PostcodeClient::new("YOUR_API_TOKEN");
//!
//! // Find the address matching on a postcode and house number
//! let (address, limits) = client.get_address("1012RJ", 147).await?;
//!
//! // Find the address and additional location information such as municipality, province and coordinates
//! let (address, limits) = client.get_extended_address("1012RJ", 147).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Usage Limits
//! As of the latest release of this crate, API usage is limited to 10,000 requests per day as well as 600 requests per 30 seconds. Please do not abuse this free service and ruin it for everyone else. [`ApiLimits`], included with the address response as shown above, reports the API limits (extracted from the response headers). The library validates the inputs in order to avoid making requests with invalid inputs, which would count towards the usage limits.
//!
//! # Disclaimer
//! I am not affiliated with the API provider and as such cannot make guarantees to the correctness of the results or the availability of the underlying service. Refer to <https://postcode.tech> for the service terms and conditions.

use internals::{call_api, IntoInternal, PostcodeApiFullResponse, PostcodeApiSimpleResponse};
use regex::Regex;
use reqwest::{Client, StatusCode};
use thiserror::Error;

mod internals;

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
    /// let client = PostcodeClient::new("YOUR_API_TOKEN");
    /// # }
    /// ```
    pub fn new(api_token: &str) -> Self {
        let client = Client::new();

        Self {
            api_token: api_token.to_string(),
            client,
        }
    }

    /// Find the address matching the given postcode and house number. Postcodes are formatted 1234AB or 1234 AB (with a single space). House numbers must be integers and not include postfix characters. Returns `None` when the address could not be found.
    /// ```rust,no_run
    /// # use std::error::Error;
    /// # use postcode_nl::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn Error>> {
    /// # let client: PostcodeClient = PostcodeClient::new("YOUR_API_TOKEN");
    /// let (address, limits) = client.get_address("1012RJ", 147).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_address(
        &self,
        postcode: &str,
        house_number: u32,
    ) -> Result<(Option<Address>, ApiLimits), PostcodeError> {
        let postcode = Self::validate_postcode_input(postcode)?;

        let response = call_api(&self.client, &self.api_token, postcode, house_number, false).await?;

        let limits = response.headers().try_into()?;
        let address = if response.status() == StatusCode::OK {
            Some(
                response
                    .json::<PostcodeApiSimpleResponse>()
                    .await
                    .map_err(|e| {
                        PostcodeError::InvalidApiResponse(format! {"Failed to deserialize API response, {e}"})
                    })?
                    .into_internal(postcode, house_number),
            )
        } else {
            None
        };

        Ok((address, limits))
    }

    /// Find the address, municipality, province and coordinates matching the given postcode and house number. Postcodes are formatted 1234AB or 1234 AB (with a single space). House numbers must be integers and not include postfix characters. Returns `None` when the address could not be found.
    /// ```rust,no_run
    /// # use std::error::Error;
    /// # use postcode_nl::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn Error>> {
    /// # let client: PostcodeClient = PostcodeClient::new("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx");
    /// let (address, limits) = client.get_extended_address("1012RJ", 147).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_extended_address(
        &self,
        postcode: &str,
        house_number: u32,
    ) -> Result<(Option<ExtendedAddress>, ApiLimits), PostcodeError> {
        let postcode = Self::validate_postcode_input(postcode)?;

        let response = call_api(&self.client, &self.api_token, postcode, house_number, true).await?;

        let limits = response.headers().try_into()?;
        let address = if response.status() == StatusCode::OK {
            Some(
                response
                    .json::<PostcodeApiFullResponse>()
                    .await
                    .map_err(|e| {
                        PostcodeError::InvalidApiResponse(format! {"Failed to deserialize API response, {e}"})
                    })?
                    .into(),
            )
        } else {
            None
        };

        Ok((address, limits))
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
    #[error("API returned that inputs are invalid")]
    InvalidData(String),
    /// The API responded with 429 TOO MANY REQUESTS. You've exceeded the API limits.
    #[error("API limits exceeded")]
    TooManyRequests(String),
    /// The API returned an unexpected error code.
    #[error("API returned an error")]
    OtherApiError(String),
}
