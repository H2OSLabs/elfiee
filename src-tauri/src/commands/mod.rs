pub mod block;
pub mod editor;
pub mod event;
pub mod file;

// Re-export all commands for easy registration
pub use block::{check_permission, execute_command, get_all_blocks, get_block};
pub use event::get_state_at_event;
pub use file::{
    close_file, create_file, get_all_events, get_file_info, list_open_files, open_file,
    rename_file, FileMetadata,
};
