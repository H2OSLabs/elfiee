//! Structured output parser for LLM responses
//!
//! Parses XML-formatted command proposals from LLM output.
//!
//! ## Expected Format
//!
//! ```xml
//! <thinking>
//! Analysis of the user request...
//! </thinking>
//!
//! <command>
//!   <cap_id>markdown.write</cap_id>
//!   <block_id>block-xxx</block_id>
//!   <payload>{"content": "# Hello World"}</payload>
//!   <description>Create markdown content</description>
//! </command>
//! ```

use super::error::LlmError;
use crate::extensions::agent::ProposedCommand;
use regex::Regex;

/// Parse structured output from LLM response.
///
/// Extracts `<command>` blocks and converts them to `ProposedCommand` structs.
/// Ignores `<thinking>` and other non-command tags.
///
/// # Arguments
/// * `response` - Raw LLM response text
///
/// # Returns
/// Vec of parsed commands or parse error
pub fn parse_structured_output(response: &str) -> Result<Vec<ProposedCommand>, LlmError> {
    let command_regex =
        Regex::new(r"<command>([\s\S]*?)</command>").map_err(|e| LlmError::ParseError(e.to_string()))?;

    let cap_id_regex =
        Regex::new(r"<cap_id>(.*?)</cap_id>").map_err(|e| LlmError::ParseError(e.to_string()))?;
    let block_id_regex =
        Regex::new(r"<block_id>(.*?)</block_id>").map_err(|e| LlmError::ParseError(e.to_string()))?;
    let payload_regex =
        Regex::new(r"<payload>([\s\S]*?)</payload>").map_err(|e| LlmError::ParseError(e.to_string()))?;
    let desc_regex =
        Regex::new(r"<description>(.*?)</description>").map_err(|e| LlmError::ParseError(e.to_string()))?;

    let mut commands = Vec::new();

    for cap in command_regex.captures_iter(response) {
        let command_block = &cap[1];

        // Extract cap_id (required)
        let cap_id = cap_id_regex
            .captures(command_block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .ok_or_else(|| LlmError::ParseError("Missing cap_id in command block".to_string()))?;

        // Extract block_id (required)
        let block_id = block_id_regex
            .captures(command_block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .ok_or_else(|| LlmError::ParseError("Missing block_id in command block".to_string()))?;

        // Extract payload (optional, defaults to {})
        let payload_str = payload_regex
            .captures(command_block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim())
            .unwrap_or("{}");

        // Parse payload as JSON
        let payload: serde_json::Value = serde_json::from_str(payload_str)
            .map_err(|e| LlmError::ParseError(format!("Invalid payload JSON: {}", e)))?;

        // Extract description (optional)
        let description = desc_regex
            .captures(command_block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string());

        commands.push(ProposedCommand {
            cap_id,
            block_id,
            payload,
            description,
        });
    }

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_command() {
        let response = r##"
<thinking>I need to write markdown content</thinking>

<command>
  <cap_id>markdown.write</cap_id>
  <block_id>block-123</block_id>
  <payload>{"content": "# Hello"}</payload>
  <description>Write heading</description>
</command>
"##;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].cap_id, "markdown.write");
        assert_eq!(commands[0].block_id, "block-123");
        assert_eq!(commands[0].payload["content"], "# Hello");
        assert_eq!(commands[0].description, Some("Write heading".to_string()));
    }

    #[test]
    fn test_parse_multiple_commands() {
        let response = r#"
<command>
  <cap_id>markdown.write</cap_id>
  <block_id>block-1</block_id>
  <payload>{"content": "First"}</payload>
</command>

<command>
  <cap_id>code.write</cap_id>
  <block_id>block-2</block_id>
  <payload>{"content": "fn main() {}", "language": "rust"}</payload>
  <description>Add Rust code</description>
</command>
"#;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].cap_id, "markdown.write");
        assert_eq!(commands[1].cap_id, "code.write");
        assert_eq!(commands[1].payload["language"], "rust");
    }

    #[test]
    fn test_parse_command_without_description() {
        let response = r#"
<command>
  <cap_id>core.link</cap_id>
  <block_id>block-abc</block_id>
  <payload>{"target_id": "block-xyz", "relation": "reference"}</payload>
</command>
"#;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].cap_id, "core.link");
        assert!(commands[0].description.is_none());
    }

    #[test]
    fn test_parse_command_with_empty_payload() {
        let response = r#"
<command>
  <cap_id>markdown.read</cap_id>
  <block_id>block-123</block_id>
  <payload>{}</payload>
</command>
"#;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(commands[0].payload.is_object());
    }

    #[test]
    fn test_parse_command_missing_payload_defaults_to_empty() {
        let response = r#"
<command>
  <cap_id>markdown.read</cap_id>
  <block_id>block-123</block_id>
</command>
"#;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(commands[0].payload.is_object());
    }

    #[test]
    fn test_parse_no_commands() {
        let response = r#"
<thinking>
The user just wants to chat, no commands needed.
</thinking>

I can help you with that! What would you like to discuss?
"#;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn test_parse_missing_cap_id_fails() {
        let response = r#"
<command>
  <block_id>block-123</block_id>
  <payload>{}</payload>
</command>
"#;
        let result = parse_structured_output(response);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing cap_id"));
    }

    #[test]
    fn test_parse_missing_block_id_fails() {
        let response = r#"
<command>
  <cap_id>markdown.write</cap_id>
  <payload>{}</payload>
</command>
"#;
        let result = parse_structured_output(response);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing block_id"));
    }

    #[test]
    fn test_parse_invalid_json_payload_fails() {
        let response = r#"
<command>
  <cap_id>markdown.write</cap_id>
  <block_id>block-123</block_id>
  <payload>{invalid json}</payload>
</command>
"#;
        let result = parse_structured_output(response);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid payload JSON"));
    }

    #[test]
    fn test_parse_multiline_payload() {
        let response = r#"
<command>
  <cap_id>code.write</cap_id>
  <block_id>block-123</block_id>
  <payload>{
    "content": "fn main() {\n    println!(\"Hello\");\n}",
    "language": "rust"
  }</payload>
</command>
"#;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(commands[0].payload["content"]
            .as_str()
            .unwrap()
            .contains("println"));
    }

    #[test]
    fn test_parse_with_surrounding_text() {
        let response = r##"
I'll help you create the document.

<thinking>User wants a markdown file</thinking>

<command>
  <cap_id>markdown.write</cap_id>
  <block_id>block-abc</block_id>
  <payload>{"content": "# Title"}</payload>
  <description>Create title</description>
</command>

Let me know if you need anything else!
"##;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].cap_id, "markdown.write");
    }
}
