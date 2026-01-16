//! Context collector for Agent extension
//!
//! Collects context from related blocks for LLM prompts.

use crate::models::Block;
use std::collections::{HashMap, HashSet};

/// Context item representing a block's content for LLM
#[derive(Debug, Clone)]
pub struct ContextItem {
    /// Block ID
    pub block_id: String,
    /// Block type (e.g., "markdown", "code")
    pub block_type: String,
    /// Block name
    pub name: String,
    /// Extracted content
    pub content: String,
    /// Depth from the starting block (0 = starting block)
    pub depth: u32,
}

/// Collect context from a block and its related blocks.
///
/// Traverses `Block.children` to find related blocks via "reference" and "implement" relations.
///
/// # Arguments
/// * `block` - Starting block
/// * `blocks` - All blocks in the file (for lookup)
/// * `max_depth` - Maximum traversal depth
///
/// # Returns
/// Vec of context items, ordered by depth (shallower first)
pub fn collect_context(
    block: &Block,
    blocks: &HashMap<String, Block>,
    max_depth: u32,
) -> Vec<ContextItem> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut context_items: Vec<ContextItem> = Vec::new();

    collect_recursive(
        block,
        blocks,
        0,
        max_depth,
        &mut visited,
        &mut context_items,
    );

    context_items
}

fn collect_recursive(
    block: &Block,
    blocks: &HashMap<String, Block>,
    current_depth: u32,
    max_depth: u32,
    visited: &mut HashSet<String>,
    context_items: &mut Vec<ContextItem>,
) {
    if current_depth > max_depth || visited.contains(&block.block_id) {
        return;
    }

    visited.insert(block.block_id.clone());

    // Extract content based on block type
    let content = extract_content(block);

    context_items.push(ContextItem {
        block_id: block.block_id.clone(),
        block_type: block.block_type.clone(),
        name: block.name.clone(),
        content,
        depth: current_depth,
    });

    // Traverse children with specific relation types
    for (relation, child_ids) in &block.children {
        // Only follow "reference" and "implement" relations
        if relation == "reference" || relation == "implement" || relation == "child" {
            for child_id in child_ids {
                if let Some(child_block) = blocks.get(child_id) {
                    collect_recursive(
                        child_block,
                        blocks,
                        current_depth + 1,
                        max_depth,
                        visited,
                        context_items,
                    );
                }
            }
        }
    }
}

/// Extract readable content from a block based on its type
fn extract_content(block: &Block) -> String {
    match block.block_type.as_str() {
        "markdown" => block
            .contents
            .get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "code" => {
            let content = block
                .contents
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let language = block
                .contents
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("text");
            format!("```{}\n{}\n```", language, content)
        }
        "agent" => {
            // For agent blocks, show the system prompt if available
            block
                .contents
                .get("system_prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("[Agent Configuration]")
                .to_string()
        }
        _ => {
            // For unknown types, pretty-print the contents JSON
            serde_json::to_string_pretty(&block.contents).unwrap_or_default()
        }
    }
}

/// Format context items as markdown for LLM consumption
///
/// # Arguments
/// * `items` - Context items to format
///
/// # Returns
/// Markdown-formatted string
pub fn format_context_as_markdown(items: &[ContextItem]) -> String {
    let mut output = String::new();

    for item in items {
        output.push_str(&format!(
            "## {} ({})\n**ID:** `{}`\n\n{}\n\n---\n\n",
            item.name, item.block_type, item.block_id, item.content
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_block(id: &str, name: &str, block_type: &str, contents: serde_json::Value) -> Block {
        Block {
            block_id: id.to_string(),
            name: name.to_string(),
            block_type: block_type.to_string(),
            owner: "test-owner".to_string(),
            contents,
            children: HashMap::new(),
            metadata: crate::models::BlockMetadata::new(),
        }
    }

    #[test]
    fn test_collect_single_block() {
        let block = create_test_block(
            "block-1",
            "Test Document",
            "markdown",
            serde_json::json!({"markdown": "# Hello"}),
        );

        let blocks: HashMap<String, Block> = HashMap::new();
        let items = collect_context(&block, &blocks, 2);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].block_id, "block-1");
        assert_eq!(items[0].name, "Test Document");
        assert_eq!(items[0].content, "# Hello");
        assert_eq!(items[0].depth, 0);
    }

    #[test]
    fn test_collect_with_references() {
        let mut parent = create_test_block(
            "block-parent",
            "Parent",
            "markdown",
            serde_json::json!({"markdown": "# Parent"}),
        );
        parent.children.insert(
            "reference".to_string(),
            vec!["block-child".to_string()],
        );

        let child = create_test_block(
            "block-child",
            "Child",
            "code",
            serde_json::json!({"content": "fn main() {}", "language": "rust"}),
        );

        let mut blocks: HashMap<String, Block> = HashMap::new();
        blocks.insert("block-child".to_string(), child);

        let items = collect_context(&parent, &blocks, 2);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].block_id, "block-parent");
        assert_eq!(items[0].depth, 0);
        assert_eq!(items[1].block_id, "block-child");
        assert_eq!(items[1].depth, 1);
        assert!(items[1].content.contains("```rust"));
    }

    #[test]
    fn test_collect_respects_max_depth() {
        let mut level0 = create_test_block(
            "block-0",
            "Level 0",
            "markdown",
            serde_json::json!({"markdown": "L0"}),
        );
        level0.children.insert(
            "reference".to_string(),
            vec!["block-1".to_string()],
        );

        let mut level1 = create_test_block(
            "block-1",
            "Level 1",
            "markdown",
            serde_json::json!({"markdown": "L1"}),
        );
        level1.children.insert(
            "reference".to_string(),
            vec!["block-2".to_string()],
        );

        let level2 = create_test_block(
            "block-2",
            "Level 2",
            "markdown",
            serde_json::json!({"markdown": "L2"}),
        );

        let mut blocks: HashMap<String, Block> = HashMap::new();
        blocks.insert("block-1".to_string(), level1);
        blocks.insert("block-2".to_string(), level2);

        // Max depth = 1, should only get level 0 and 1
        let items = collect_context(&level0, &blocks, 1);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].block_id, "block-0");
        assert_eq!(items[1].block_id, "block-1");
    }

    #[test]
    fn test_collect_avoids_cycles() {
        let mut block_a = create_test_block(
            "block-a",
            "Block A",
            "markdown",
            serde_json::json!({"markdown": "A"}),
        );
        block_a.children.insert(
            "reference".to_string(),
            vec!["block-b".to_string()],
        );

        let mut block_b = create_test_block(
            "block-b",
            "Block B",
            "markdown",
            serde_json::json!({"markdown": "B"}),
        );
        // Circular reference back to A
        block_b.children.insert(
            "reference".to_string(),
            vec!["block-a".to_string()],
        );

        let mut blocks: HashMap<String, Block> = HashMap::new();
        blocks.insert("block-a".to_string(), block_a.clone());
        blocks.insert("block-b".to_string(), block_b);

        // Should not infinite loop
        let items = collect_context(&block_a, &blocks, 10);

        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_format_context_as_markdown() {
        let items = vec![
            ContextItem {
                block_id: "block-1".to_string(),
                block_type: "markdown".to_string(),
                name: "Introduction".to_string(),
                content: "# Welcome".to_string(),
                depth: 0,
            },
            ContextItem {
                block_id: "block-2".to_string(),
                block_type: "code".to_string(),
                name: "Example".to_string(),
                content: "```rust\nfn main() {}\n```".to_string(),
                depth: 1,
            },
        ];

        let formatted = format_context_as_markdown(&items);

        assert!(formatted.contains("## Introduction (markdown)"));
        assert!(formatted.contains("**ID:** `block-1`"));
        assert!(formatted.contains("## Example (code)"));
        assert!(formatted.contains("fn main()"));
    }

    #[test]
    fn test_extract_code_content() {
        let block = create_test_block(
            "block-1",
            "Code",
            "code",
            serde_json::json!({"content": "print('hello')", "language": "python"}),
        );

        let blocks: HashMap<String, Block> = HashMap::new();
        let items = collect_context(&block, &blocks, 0);

        assert!(items[0].content.contains("```python"));
        assert!(items[0].content.contains("print('hello')"));
    }
}
