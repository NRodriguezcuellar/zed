use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::WorkspaceId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSessionId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSessionState {
    pub session_id: WorkspaceSessionId,
    pub title: String,
    pub root_paths: Vec<PathBuf>,
    pub workspace_id: WorkspaceId,
    pub active_item_id: Option<u64>,
    pub active_terminal_item_id: Option<u64>,
    pub active_agent_thread_id: Option<String>,
}

impl WorkspaceSessionState {
    pub fn new(
        session_id: WorkspaceSessionId,
        title: String,
        root_paths: Vec<PathBuf>,
        workspace_id: WorkspaceId,
    ) -> Self {
        Self {
            session_id,
            title,
            root_paths,
            workspace_id,
            active_item_id: None,
            active_terminal_item_id: None,
            active_agent_thread_id: None,
        }
    }

    pub fn update_active_item(&mut self, item_id: Option<u64>) {
        self.active_item_id = item_id;
    }

    pub fn update_active_terminal(&mut self, terminal_item_id: Option<u64>) {
        self.active_terminal_item_id = terminal_item_id;
    }

    pub fn update_active_agent_thread(&mut self, thread_id: Option<String>) {
        self.active_agent_thread_id = thread_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_session_state_starts_without_surface_restore_targets() {
        let state = WorkspaceSessionState::new(
            WorkspaceSessionId("session-a".into()),
            "project-a".into(),
            vec![PathBuf::from("/tmp/project-a")],
            WorkspaceId::from_i64(1),
        );

        assert_eq!(state.active_item_id, None);
        assert_eq!(state.active_terminal_item_id, None);
        assert_eq!(state.active_agent_thread_id, None);
    }
}
