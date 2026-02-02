pub mod agent;
pub mod code;
pub mod directory;
/// Extensions for Elfiee capability system.
///
/// This module contains extension capabilities that build on top of the core system.
/// Extensions provide domain-specific functionality for different block types.
///
/// ## Available Extensions
///
/// - `agent`: AI assistant integration with LLM APIs
/// - `markdown`: Read and write markdown content to markdown blocks
/// - `terminal`: Execute terminal commands and record them in terminal blocks
pub mod markdown;
pub mod task;
pub mod terminal;
