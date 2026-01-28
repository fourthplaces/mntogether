# twilio-rs

A Rust client library for Twilio's Verify API and ICE server provisioning.

## Features

- OTP (One-Time Password) verification via SMS or email
- ICE server provisioning for WebRTC
- Simple async API with reqwest

## Installation

```toml
[dependencies]
twilio = { path = "../twilio-rs" }
```

## Usage

### Configuration

```rust
use twilio::{TwilioService, TwilioOptions};

let service = TwilioService::new(TwilioOptions {
    account_sid: "your_account_sid".to_string(),
    auth_token: "your_auth_token".to_string(),
    service_id: "your_verify_service_id".to_string(),
});
```

### Send OTP

```rust
// Send OTP via SMS
let response = service.send_otp("+1234567890").await?;

// Send OTP via email
let response = service.send_otp("user@example.com").await?;
```

The channel (SMS or email) is automatically determined based on the recipient format.

### Verify OTP

```rust
let result = service.verify_otp("+1234567890", "123456").await;
match result {
    Ok(()) => println!("Verification successful"),
    Err(e) => println!("Verification failed: {}", e),
}
```

### Fetch ICE Servers

For WebRTC TURN/STUN server credentials:

```rust
let ice_servers = service.fetch_ice_servers().await?;
// Returns JSON with ICE server configuration
```

## Environment Variables

| Variable             | Description                   |
| -------------------- | ----------------------------- |
| `TWILIO_ACCOUNT_SID` | Your Twilio Account SID       |
| `TWILIO_AUTH_TOKEN`  | Your Twilio Auth Token        |
| `TWILIO_SERVICE_ID`  | Your Twilio Verify Service ID |

## API Reference

### TwilioService

| Method                        | Description                |
| ----------------------------- | -------------------------- |
| `send_otp(recipient)`         | Send OTP to phone or email |
| `verify_otp(recipient, code)` | Verify OTP code            |
| `fetch_ice_servers()`         | Get TURN/STUN credentials  |

## Dependencies

- `reqwest` - HTTP client
- `serde` / `serde_json` - JSON serialization

## License

MIT
