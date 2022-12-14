[![crates.io](https://img.shields.io/crates/d/postcode-nl)](https://crates.io/crates/postcode-nl)
[![crates.io](https://img.shields.io/crates/v/postcode-nl)](https://crates.io/crates/postcode-nl)
[![API](https://docs.rs/postcode/badge.svg)](https://docs.rs/postcode-nl)

# postcode-nl

Async client for the free Netherlands postcode API at <https://postcode.tech>.

There are two methods, one to find the street and city matching the supplied postcode and house number, and one that also includes the municipality, province and coordinates. If no address can be found for the postcode and house number combination, `None` is returned.

## Example
```rust
// Initialize a client
let client = PostcodeClient::new("YOUR_API_TOKEN");

// Find the address matching on a postcode and house number
let (address, limits) = client.get_address("1012RJ", 147).await?;

// Find the address and additional location information such as municipality, province and coordinates
let (address, limits) = client.get_extended_address("1012RJ", 147).await?;
```

## Usage Limits
As of the latest release of this crate, API usage is limited to 10,000 requests per day as well as 600 requests per 30 seconds. Please do not abuse this free service and ruin it for everyone else. [`ApiLimits`], included with the address response as shown above, reports the API limits (extracted from the response headers). The library validates the inputs in order to avoid making requests with invalid inputs, which would count towards the usage limits.

## Disclaimer
I am not affiliated with the API provider and as such cannot make guarantees to the correctness of the results or the availability of the underlying service. Refer to <https://postcode.tech> for the service terms and conditions.
