// ── Generated protobuf types ──────────────────────────────────────────────────
pub mod proto {
    tonic::include_proto!("users");
}

use proto::auth_service_client::AuthServiceClient;
use proto::{OtpRequest, VerifyRequest};

/// Address of the gRPC auth server.
/// Change this to match your deployment (e.g. "https://api.example.com:443").
pub const SERVER_ADDR: &str = "http://[::1]:50051";

// ── Public async helpers ──────────────────────────────────────────────────────

/// Send a one-time-password to `identifier` (email or phone).
///
/// Returns the opaque `request_id` that must be passed to `verify_otp`, or an
/// error string to display to the user.
pub async fn request_otp(identifier: String) -> Result<String, String> {
    let mut client = AuthServiceClient::connect(SERVER_ADDR)
        .await
        .map_err(|e| format!("Cannot reach server: {e}"))?;

    let resp = client
        .request_otp(OtpRequest {
            identifier: identifier.clone(),
        })
        .await
        .map_err(|s| format!("Server error: {}", s.message()))?
        .into_inner();

    if resp.success {
        Ok(resp.request_id)
    } else {
        Err(if resp.message.is_empty() {
            "Failed to send OTP. Please try again.".into()
        } else {
            resp.message
        })
    }
}

/// Verify the OTP the user entered.
///
/// Returns the session token on success, or an error string.
pub async fn verify_otp(
    request_id: String,
    identifier: String,
    otp: String,
) -> Result<String, String> {
    let mut client = AuthServiceClient::connect(SERVER_ADDR)
        .await
        .map_err(|e| format!("Cannot reach server: {e}"))?;

    let resp = client
        .verify_otp(VerifyRequest {
            request_id,
            identifier,
            otp,
        })
        .await
        .map_err(|s| format!("Server error: {}", s.message()))?
        .into_inner();

    if resp.success {
        Ok(resp.token)
    } else {
        Err(if resp.message.is_empty() {
            "Invalid OTP. Please check and try again.".into()
        } else {
            resp.message
        })
    }
}