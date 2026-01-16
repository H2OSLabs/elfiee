//! LLM Error types for Agent extension

use std::fmt;

/// Errors that can occur during LLM operations
#[derive(Debug)]
pub enum LlmError {
    /// Network error during API call
    Network(String),
    /// Rate limit exceeded (HTTP 429)
    RateLimit,
    /// Invalid or unexpected response from API
    InvalidResponse(String),
    /// Unauthorized - invalid API key (HTTP 401)
    Unauthorized,
    /// Parsing error when processing LLM output
    ParseError(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::Network(msg) => write!(f, "Network error: {}", msg),
            LlmError::RateLimit => write!(f, "Rate limit exceeded"),
            LlmError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            LlmError::Unauthorized => write!(f, "Unauthorized: Invalid API key"),
            LlmError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for LlmError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            LlmError::Network("timeout".to_string()).to_string(),
            "Network error: timeout"
        );
        assert_eq!(LlmError::RateLimit.to_string(), "Rate limit exceeded");
        assert_eq!(
            LlmError::Unauthorized.to_string(),
            "Unauthorized: Invalid API key"
        );
        assert_eq!(
            LlmError::InvalidResponse("bad json".to_string()).to_string(),
            "Invalid response: bad json"
        );
        assert_eq!(
            LlmError::ParseError("missing cap_id".to_string()).to_string(),
            "Parse error: missing cap_id"
        );
    }
}
