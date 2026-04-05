//! Structured error types for IPC communication
//!
//! This module provides rich, structured error types that can be serialized
//! across the Tauri IPC boundary with user-friendly messages and recovery actions.

use serde::{Deserialize, Serialize};

/// Error codes for categorizing different types of failures
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// Permission denied (not running as admin/root)
    PermissionDenied,
    /// Network interface not found
    InterfaceNotFound,
    /// MAC address validation failed
    InvalidMacAddress,
    /// IP address validation failed
    InvalidIpAddress,
    /// Raw socket creation failed
    RawSocketError,
    /// ARP operation failed
    ArpError,
    /// Poisoning operation failed
    PoisoningError,
    /// Scan operation failed
    ScanError,
    /// Platform not supported
    PlatformNotSupported,
    /// Internal server error
    InternalError,
    /// IO error
    IoError,
}

/// Structured API error with user-friendly information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Machine-readable error code
    pub code: ErrorCode,
    /// User-friendly error message
    pub user_message: String,
    /// Whether the operation can be retried
    pub retryable: bool,
    /// Suggested action for the user
    pub suggested_action: Option<String>,
    /// Technical details (for debugging)
    pub technical_details: Option<String>,
}

impl ApiError {
    /// Create a new API error
    pub fn new(code: ErrorCode, user_message: impl Into<String>) -> Self {
        Self {
            code,
            user_message: user_message.into(),
            retryable: false,
            suggested_action: None,
            technical_details: None,
        }
    }

    /// Mark the error as retryable
    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }

    /// Add a suggested action
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.suggested_action = Some(action.into());
        self
    }

    /// Add technical details
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.technical_details = Some(details.into());
        self
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", format_code(&self.code), self.user_message)
    }
}

impl std::error::Error for ApiError {}

fn format_code(code: &ErrorCode) -> &'static str {
    match code {
        ErrorCode::PermissionDenied => "PERMISSION_DENIED",
        ErrorCode::InterfaceNotFound => "INTERFACE_NOT_FOUND",
        ErrorCode::InvalidMacAddress => "INVALID_MAC_ADDRESS",
        ErrorCode::InvalidIpAddress => "INVALID_IP_ADDRESS",
        ErrorCode::RawSocketError => "RAW_SOCKET_ERROR",
        ErrorCode::ArpError => "ARP_ERROR",
        ErrorCode::PoisoningError => "POISONING_ERROR",
        ErrorCode::ScanError => "SCAN_ERROR",
        ErrorCode::PlatformNotSupported => "PLATFORM_NOT_SUPPORTED",
        ErrorCode::InternalError => "INTERNAL_ERROR",
        ErrorCode::IoError => "IO_ERROR",
    }
}

impl From<crate::network::types::NetworkError> for ApiError {
    fn from(err: crate::network::types::NetworkError) -> Self {
        use crate::network::types::NetworkError;

        match err {
            NetworkError::PermissionDenied(msg) => ApiError::new(
                ErrorCode::PermissionDenied,
                "Administrator privileges required",
            )
            .with_action("Run the application with administrator/root privileges")
            .with_details(msg),

            NetworkError::InterfaceNotFound(msg) => ApiError::new(
                ErrorCode::InterfaceNotFound,
                format!("Network interface not found: {}", msg),
            )
            .with_action("Check that the interface exists and is connected")
            .retryable(),

            NetworkError::MacAddressError(msg) => {
                ApiError::new(ErrorCode::InvalidMacAddress, "MAC address error").with_details(msg)
            }

            NetworkError::MacSetError(msg) => {
                ApiError::new(ErrorCode::InvalidMacAddress, "Failed to set MAC address")
                    .with_details(msg)
            }

            NetworkError::ArpScanError(msg) => {
                ApiError::new(ErrorCode::ScanError, "ARP scan failed")
                    .with_details(msg)
                    .retryable()
            }

            NetworkError::PingScanError(msg) => {
                ApiError::new(ErrorCode::ScanError, "Ping scan failed")
                    .with_details(msg)
                    .retryable()
            }

            NetworkError::PoisoningError(msg) => ApiError::new(
                ErrorCode::PoisoningError,
                "Network control operation failed",
            )
            .with_details(msg)
            .retryable(),

            NetworkError::RawSocketError(msg) => {
                ApiError::new(ErrorCode::RawSocketError, "Raw socket creation failed")
                    .with_action(
                        "Run with administrator/root privileges or check firewall settings",
                    )
                    .with_details(msg)
            }

            NetworkError::PacketSendError(msg) => {
                ApiError::new(ErrorCode::ArpError, "Failed to send network packet")
                    .with_details(msg)
                    .retryable()
            }

            NetworkError::InvalidMacAddress(msg) => ApiError::new(
                ErrorCode::InvalidMacAddress,
                format!("Invalid MAC address format: {}", msg),
            )
            .with_action("Use format XX:XX:XX:XX:XX:XX with valid hexadecimal characters"),

            NetworkError::MacValidationError(mac, reason) => {
                let reason_str = match reason {
                    crate::network::types::MacValidationError::BroadcastAddress => "broadcast",
                    crate::network::types::MacValidationError::MulticastAddress => "multicast",
                    crate::network::types::MacValidationError::AllZeros => "all-zeros",
                };
                ApiError::new(
                    ErrorCode::InvalidMacAddress,
                    format!("Invalid MAC address: {}", mac),
                )
                .with_action(format!(
                    "MAC address is a {} address and cannot be used",
                    reason_str
                ))
                .with_details(format!("{} is a {} address", mac, reason_str))
            }

            NetworkError::InvalidIpAddress(msg) => ApiError::new(
                ErrorCode::InvalidIpAddress,
                format!("Invalid IP address: {}", msg),
            )
            .with_action("Enter a valid IPv4 address"),

            NetworkError::PlatformNotSupported(msg) => ApiError::new(
                ErrorCode::PlatformNotSupported,
                "Operation not supported on this platform",
            )
            .with_details(msg),

            NetworkError::IoError(e) => ApiError::new(ErrorCode::IoError, "I/O error occurred")
                .with_details(e.to_string())
                .retryable(),

            NetworkError::ForwardingError(msg) => {
                ApiError::new(ErrorCode::InternalError, "Packet forwarding error").with_details(msg)
            }

            NetworkError::ConnectionTrackError(msg) => {
                ApiError::new(ErrorCode::InternalError, "Connection tracking error")
                    .with_details(msg)
            }

            NetworkError::IpForwardingDisabled => ApiError::new(
                ErrorCode::InternalError,
                "IP forwarding is disabled on this system",
            )
            .with_action("Enable IP forwarding or run with appropriate permissions"),

            NetworkError::BandwidthError(msg) => {
                ApiError::new(ErrorCode::InternalError, "Bandwidth control error").with_details(msg)
            }
        }
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = std::result::Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_creation() {
        let err = ApiError::new(ErrorCode::PermissionDenied, "Test error");
        assert_eq!(err.code, ErrorCode::PermissionDenied);
        assert_eq!(err.user_message, "Test error");
        assert!(!err.retryable);
    }

    #[test]
    fn test_api_error_with_action() {
        let err =
            ApiError::new(ErrorCode::PermissionDenied, "Test error").with_action("Run as admin");
        assert_eq!(err.suggested_action, Some("Run as admin".to_string()));
    }

    #[test]
    fn test_api_error_retryable() {
        let err = ApiError::new(ErrorCode::ScanError, "Test error").retryable();
        assert!(err.retryable);
    }
}
