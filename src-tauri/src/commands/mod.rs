pub mod agent;
pub mod block;
pub mod checkout;
pub mod editor;
pub mod event;
pub mod file;
pub mod task;

// Re-export all commands for easy registration
pub use agent::{agent_create, agent_disable, agent_enable};
pub use block::{check_permission, execute_command, get_all_blocks, get_block, rename_block};
pub use checkout::checkout_workspace;
pub use event::get_state_at_event;
pub use file::{
    close_file, create_file, get_all_events, get_file_info, list_open_files, open_file,
    rename_file, save_file, FileMetadata,
};
pub use task::{commit_task, inject_hooks_for_repo, is_hooks_active, remove_hooks_for_repo};
