// https://dev.to/hackmamba/how-to-build-a-one-time-passwordotp-verification-api-with-rust-and-twilio-22il

use std::collections::HashMap;

pub mod models;
use reqwest::{Client, header};

use crate::models::{OTPResponse, OTPVerifyResponse};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct TwilioOptions {
    pub account_sid: String,
    pub auth_token: String,
    pub service_id: String,
}

#[derive(Debug, Clone)]
pub struct TwilioService {
    options: TwilioOptions,
}

impl TwilioService {
    pub fn new(options: TwilioOptions) -> Self {
        Self { options }
    }

    pub async fn send_otp(
        self: &TwilioService,
        recipient: &str,
    ) -> Result<OTPResponse, &'static str> {
        let account_sid = self.options.account_sid.clone();
        let auth_token = self.options.auth_token.clone();
        let service_id = self.options.service_id.clone();

        let url = format!(
            "https://verify.twilio.com/v2/Services/{serv_id}/Verifications",
            serv_id = service_id
        );

        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            "application/x-www-form-urlencoded"
                .parse()
                .expect("Header value should parse correctly"),
        );

        // Determine channel based on recipient format (email vs phone)
        let channel = if recipient.contains('@') {
            "email"
        } else {
            "sms"
        };

        let mut form_body: HashMap<&str, String> = HashMap::new();
        form_body.insert("To", recipient.to_string());
        form_body.insert("Channel", channel.to_string());

        let client = Client::new();
        let res = client
            .post(url)
            .basic_auth(account_sid, Some(auth_token))
            .headers(headers)
            .form(&form_body)
            .send()
            .await;

        match res {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    // Log the error response from Twilio
                    let error_body = response.text().await.unwrap_or_default();
                    eprintln!("Twilio error ({}): {}", status, error_body);
                    return Err("Twilio returned an error");
                }

                let result = response.json::<OTPResponse>().await;
                match result {
                    Ok(data) => Ok(data),
                    Err(e) => {
                        eprintln!("Failed to parse Twilio response: {}", e);
                        Err("Error parsing OTP response")
                    }
                }
            }
            Err(e) => {
                eprintln!("Request to Twilio failed: {}", e);
                Err("Error sending OTP")
            }
        }
    }

    pub async fn verify_otp(&self, recipient: &str, code: &str) -> Result<(), &'static str> {
        let account_sid = self.options.account_sid.clone();
        let auth_token = self.options.auth_token.clone();
        let service_id = self.options.service_id.clone();

        let url = format!(
            "https://verify.twilio.com/v2/Services/{serv_id}/VerificationCheck",
            serv_id = service_id,
        );

        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            "application/x-www-form-urlencoded"
                .parse()
                .expect("Header value should parse correctly"),
        );

        let mut form_body: HashMap<&str, &str> = HashMap::new();
        form_body.insert("To", recipient);
        form_body.insert("Code", code);

        let client = Client::new();
        let res = client
            .post(url)
            .basic_auth(account_sid, Some(auth_token))
            .headers(headers)
            .form(&form_body)
            .send()
            .await;

        match res {
            Ok(response) => {
                let data = response.json::<OTPVerifyResponse>().await;
                match data {
                    Ok(result) => {
                        if result.status == "approved" {
                            Ok(())
                        } else {
                            Err("Error verifying OTP")
                        }
                    }
                    Err(_) => Err("Error verifying OTP"),
                }
            }
            Err(_) => Err("Error verifying OTP"),
        }
    }

    pub async fn fetch_ice_servers(&self) -> Result<Value, &'static str> {
        let account_sid = self.options.account_sid.clone();
        let auth_token = self.options.auth_token.clone();

        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Tokens.json",
            account_sid
        );

        let client = Client::new();
        let response = client
            .post(url)
            .basic_auth(account_sid, Some(auth_token))
            .form(&HashMap::<&str, &str>::new())
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Err("Twilio returned an error when fetching ICE servers");
                }

                resp.json::<Value>()
                    .await
                    .map_err(|_| "Failed to parse Twilio ICE server response")
            }
            Err(_) => Err("Error fetching ICE servers from Twilio"),
        }
    }
}
