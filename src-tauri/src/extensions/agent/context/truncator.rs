//! Token truncation for context management
//!
//! Truncates context to fit within LLM token limits.

use super::collector::ContextItem;

/// Estimate token count for a string.
///
/// Uses a simple heuristic: ~4 characters per token (average for English text).
/// This is a rough approximation - for more accurate counts, use tiktoken.
///
/// # Arguments
/// * `text` - Text to count tokens for
///
/// # Returns
/// Estimated token count
pub fn count_tokens(text: &str) -> usize {
    // Simple heuristic: average ~4 characters per token
    // This is a rough approximation for Claude models
    (text.len() + 3) / 4
}

/// Truncate context items to fit within a token limit.
///
/// Strategy: Prioritize shallower items (closer to the starting block).
/// Removes deeper items first when truncation is needed.
///
/// # Arguments
/// * `items` - Context items to potentially truncate
/// * `max_tokens` - Maximum total tokens allowed
///
/// # Returns
/// Truncated list of context items that fit within the limit
pub fn truncate_context(items: &[ContextItem], max_tokens: u32) -> Vec<ContextItem> {
    // Sort by depth (ascending - keep shallower items first)
    let mut sorted_items: Vec<_> = items.to_vec();
    sorted_items.sort_by_key(|item| item.depth);

    let mut result: Vec<ContextItem> = Vec::new();
    let mut total_tokens: u32 = 0;

    for item in sorted_items {
        // Calculate tokens for this item (including formatting overhead)
        let item_text = format!(
            "## {} ({})\n**ID:** `{}`\n\n{}\n\n---\n\n",
            item.name, item.block_type, item.block_id, item.content
        );
        let item_tokens = count_tokens(&item_text) as u32;

        if total_tokens + item_tokens <= max_tokens {
            total_tokens += item_tokens;
            result.push(item);
        } else {
            // Try to include a truncated version of this item if there's room
            let remaining_tokens = max_tokens.saturating_sub(total_tokens);
            if remaining_tokens > 50 {
                // Only truncate if we have at least 50 tokens of space
                let truncated = truncate_item(&item, remaining_tokens);
                if let Some(truncated_item) = truncated {
                    result.push(truncated_item);
                }
            }
            break;
        }
    }

    result
}

/// Truncate a single context item to fit within a token limit
fn truncate_item(item: &ContextItem, max_tokens: u32) -> Option<ContextItem> {
    // Calculate overhead for formatting
    let header = format!(
        "## {} ({})\n**ID:** `{}`\n\n",
        item.name, item.block_type, item.block_id
    );
    let footer = "\n\n[...truncated...]\n\n---\n\n";
    let overhead_tokens = count_tokens(&header) + count_tokens(footer);

    let content_max_tokens = max_tokens.saturating_sub(overhead_tokens as u32);
    if content_max_tokens < 20 {
        return None;
    }

    // Estimate max characters for content
    let max_chars = (content_max_tokens * 4) as usize;
    let truncated_content = if item.content.len() > max_chars {
        format!("{}...", &item.content[..max_chars.min(item.content.len())])
    } else {
        item.content.clone()
    };

    Some(ContextItem {
        block_id: item.block_id.clone(),
        block_type: item.block_type.clone(),
        name: item.name.clone(),
        content: truncated_content,
        depth: item.depth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_context_item(id: &str, content: &str, depth: u32) -> ContextItem {
        ContextItem {
            block_id: id.to_string(),
            block_type: "markdown".to_string(),
            name: format!("Block {}", id),
            content: content.to_string(),
            depth,
        }
    }

    #[test]
    fn test_count_tokens_basic() {
        // ~4 chars per token
        assert_eq!(count_tokens("hello"), 2); // 5 chars -> ~2 tokens
        assert_eq!(count_tokens("hello world"), 3); // 11 chars -> ~3 tokens
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn test_truncate_empty_input() {
        let items: Vec<ContextItem> = vec![];
        let result = truncate_context(&items, 1000);
        assert!(result.is_empty());
    }

    #[test]
    fn test_truncate_fits_all() {
        let items = vec![
            create_context_item("1", "Short content", 0),
            create_context_item("2", "More content", 1),
        ];

        let result = truncate_context(&items, 10000);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_truncate_prioritizes_shallow() {
        let items = vec![
            create_context_item("deep", "Deep content that is important", 2),
            create_context_item("shallow", "Shallow content", 0),
            create_context_item("middle", "Middle content", 1),
        ];

        // Very limited tokens - should keep shallow items
        let result = truncate_context(&items, 200);

        // First item should be the shallow one (depth 0)
        assert!(!result.is_empty());
        assert_eq!(result[0].block_id, "shallow");
    }

    #[test]
    fn test_truncate_respects_limit() {
        let long_content = "x".repeat(1000);
        let items = vec![
            create_context_item("1", &long_content, 0),
            create_context_item("2", &long_content, 1),
            create_context_item("3", &long_content, 2),
        ];

        // Only enough space for roughly one item
        let result = truncate_context(&items, 300);

        // Should have truncated to fit
        let total_content: String = result.iter().map(|i| i.content.clone()).collect();
        let total_tokens = count_tokens(&total_content);
        assert!(total_tokens <= 400); // Allow some overhead
    }

    #[test]
    fn test_truncate_maintains_order_by_depth() {
        let items = vec![
            create_context_item("3", "Third", 2),
            create_context_item("1", "First", 0),
            create_context_item("2", "Second", 1),
        ];

        let result = truncate_context(&items, 10000);

        // Should be sorted by depth
        assert_eq!(result[0].depth, 0);
        assert_eq!(result[1].depth, 1);
        assert_eq!(result[2].depth, 2);
    }
}
