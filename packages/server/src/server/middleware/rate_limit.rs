// Rate limiting middleware using tower-governor
//
// Configuration:
// - GraphQL: 100 requests per minute per IP (10/sec with burst of 20)
// - Prevents API abuse, DoS attacks, and resource exhaustion
// - Applies to OTP operations (sendVerificationCode, verifyCode) via GraphQL
//
// Applied in app.rs as a layer on the /graphql route
