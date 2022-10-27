# postcode-nl

Async client for the free Netherlands postcode API at <https://postcode.tech>.

## Example
```rust
// Initialize a client
let client: PostcodeClient = PostcodeClient::new("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx");

// Find the address matching on a postcode and house number
let (address, limits): (Address, ApiLimits) = client.get_address("1012RJ", 147).await?;

// Find the address and additional location information such as municipality, province and coordinates
let (address, limits): (ExtendedAddress, ApiLimits) = client.get_extended_address("1012RJ", 147).await?;
```

## Usage Limits
As of the latest release of this crate, API usage is limited to 10,000 requests per day as well as a 600 request rate limit over an unspecified time window. The library validates inputs in order to avoid making requests with invalid inputs, which count towards the usage limits. [`ApiLimits`], included with the address response, reports the current API limits.

## Disclaimer
I am not affiliated with the API provider and as such cannot make guarantees to the correctness of the results or the availability of the underlying service. Refer to <https://postcode.tech> for the service terms and conditions.

License: MIT
