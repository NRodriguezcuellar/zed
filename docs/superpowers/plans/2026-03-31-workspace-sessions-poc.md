# Workspace Sessions POC Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a narrow POC for project-scoped workspace sessions that preserve mixed work context and let users switch between several active sessions quickly.

**Architecture:** Build a lightweight session layer above existing Zed workspace infrastructure. Reuse `MultiWorkspace`, `Workspace`, the current sidebar, existing file panes, existing terminal state, and existing agent state where possible; avoid trying to make agents, terminals, and files share one new universal tab abstraction in the first slice.

**Tech Stack:** Rust, GPUI, existing `workspace`, `sidebar`, `agent_ui`, and `terminal_view` crates, workspace persistence tests.

---

## File map

- Create: `crates/workspace/src/workspace_session.rs` — workspace-session model and minimal serializable surface snapshot types.
- Modify: `crates/workspace/src/workspace.rs` — crate root for `workspace`; expose the new module and add small helpers for capturing and restoring the active pane state needed by the session layer.
- Modify: `crates/workspace/src/multi_workspace.rs` — own the active session list, switching logic, and restore hooks.
- Modify: `crates/workspace/src/persistence/model.rs` — add persisted session metadata for the POC.
- Modify: `crates/workspace/src/persistence.rs` — save and restore session metadata.
- Create: `crates/sidebar/src/session_switcher.rs` — sidebar UI for listing and activating sessions.
- Modify: `crates/sidebar/src/sidebar.rs` — render the session switcher and route selection events into `MultiWorkspace`.
- Modify: `crates/sidebar/src/sidebar_tests.rs` — add switching and restoration tests.
- Modify: `crates/zed/src/zed.rs` — wire the new session switcher into normal startup/sidebar registration.

## Notes before implementation

- The current codebase already separates `MultiWorkspace` (outer sidebar container) from `Workspace` (center panes + docks). Keep the session layer at the `MultiWorkspace` boundary.
- Do not move `AgentPanel` into the `Item` abstraction in this plan.
- Treat session restoration as best-effort for the POC: file tabs, active workspace, active terminal surface if already serializable, and agent thread identity if a stable reference already exists.
- If agent restoration cannot be done cleanly in the first slice, record that explicitly in the UI and preserve the rest of the session behavior.

### Task 1: Add a minimal workspace-session model

**Files:**
- Create: `crates/workspace/src/workspace_session.rs`
- Modify: `crates/workspace/src/workspace.rs`
- Test: `crates/workspace/src/workspace_session.rs`

- [ ] **Step 1: Add the new module export**

```rust
// crates/workspace/src/workspace.rs
pub mod workspace_session;
```

- [ ] **Step 2: Add the session data types**

```rust
// crates/workspace/src/workspace_session.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::WorkspaceId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSessionId(pub String);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    pub fn new(session_id: WorkspaceSessionId, title: String, root_paths: Vec<PathBuf>, workspace_id: WorkspaceId) -> Self {
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
}
```

- [ ] **Step 3: Keep the first slice intentionally narrow**

```rust
// crates/workspace/src/workspace_session.rs
impl WorkspaceSessionState {
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
```

- [ ] **Step 4: Add a model-level test at the bottom of `workspace_session.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

#[test]
fn workspace_session_state_starts_without_surface_restore_targets() {
    let state = WorkspaceSessionState::new(
        WorkspaceSessionId("session-a".into()),
        "project-a".into(),
        vec![std::path::PathBuf::from("/tmp/project-a")],
        WorkspaceId::from_proto(1),
    );

    assert_eq!(state.active_item_id, None);
    assert_eq!(state.active_terminal_item_id, None);
    assert_eq!(state.active_agent_thread_id, None);
}
}
```

- [ ] **Step 5: Run the narrow test target**

Run: `cargo test -p workspace workspace_session_state_starts_without_surface_restore_targets`

Expected: PASS

### Task 2: Teach `MultiWorkspace` to own and switch sessions

**Files:**
- Modify: `crates/workspace/src/multi_workspace.rs`
- Modify: `crates/workspace/src/workspace.rs`
- Modify: `crates/workspace/src/workspace_session.rs`
- Test: `crates/sidebar/src/sidebar_tests.rs`

- [ ] **Step 1: Add session fields to `MultiWorkspace`**

```rust
// crates/workspace/src/multi_workspace.rs
use crate::workspace_session::{WorkspaceSessionId, WorkspaceSessionState};

pub struct MultiWorkspace {
    // existing fields...
    sessions: Vec<WorkspaceSessionState>,
    active_session_id: Option<WorkspaceSessionId>,
}
```

- [ ] **Step 2: Add a small capture API on `Workspace`**

```rust
// crates/workspace/src/workspace.rs
impl Workspace {
    pub fn active_item_id_for_session(&self, cx: &App) -> Option<u64> {
        self.active_item(cx)
            .map(|item| item.item_id().as_u64())
    }
}
```

- [ ] **Step 3: Add session registration and capture logic**

```rust
// crates/workspace/src/multi_workspace.rs
impl MultiWorkspace {
    fn ensure_session_for_workspace(&mut self, workspace: &Entity<Workspace>, cx: &App) {
        let workspace_ref = workspace.read(cx);
        let workspace_id = workspace_ref.database_id();
        let root_paths = workspace_ref.root_paths(cx);

        if self.sessions.iter().any(|session| session.workspace_id == workspace_id) {
            return;
        }

        let title = root_paths
            .first()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Workspace".to_string());

        self.sessions.push(WorkspaceSessionState::new(
            WorkspaceSessionId(format!("workspace-{}", workspace_id.to_proto())),
            title,
            root_paths,
            workspace_id,
        ));
    }
}
```

- [ ] **Step 4: Add switching logic that captures before switching**

```rust
// crates/workspace/src/multi_workspace.rs
impl MultiWorkspace {
    pub fn activate_session(
        &mut self,
        session_id: &WorkspaceSessionId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.capture_active_session(cx);

        let Some(target_workspace_id) = self
            .sessions
            .iter()
            .find(|session| &session.session_id == session_id)
            .map(|session| session.workspace_id)
        else {
            return;
        };

        if let Some(workspace) = self
            .workspaces()
            .iter()
            .find(|workspace| workspace.read(cx).database_id() == target_workspace_id)
            .cloned()
        {
            self.activate_workspace(&workspace, window, cx);
            self.active_session_id = Some(session_id.clone());
            self.restore_active_session(window, cx);
        }
    }
}
```

- [ ] **Step 5: Add sidebar-facing accessors**

```rust
// crates/workspace/src/multi_workspace.rs
impl MultiWorkspace {
    pub fn sessions(&self) -> &[WorkspaceSessionState] {
        &self.sessions
    }

    pub fn active_session_id(&self) -> Option<&WorkspaceSessionId> {
        self.active_session_id.as_ref()
    }
}
```

- [ ] **Step 6: Add a switching test in the existing sidebar test file**

```rust
#[gpui::test]
async fn test_session_switcher_tracks_multiple_workspaces(cx: &mut TestAppContext) {
    let (multi_workspace, cx) = workspace::MultiWorkspace::test_new(project.clone(), cx);

    multi_workspace.update_in(cx, |mw, window, cx| {
        mw.create_test_workspace(window, cx).detach();
    });

    multi_workspace.read_with(cx, |mw, _| {
        assert!(mw.sessions().len() >= 2);
    });
}
```

- [ ] **Step 7: Run the focused sidebar test**

Run: `cargo test -p sidebar test_session_switcher_tracks_multiple_workspaces -- --nocapture`

Expected: PASS

### Task 3: Persist minimal session state

**Files:**
- Modify: `crates/workspace/src/persistence/model.rs`
- Modify: `crates/workspace/src/persistence.rs`
- Modify: `crates/workspace/src/multi_workspace.rs`
- Test: `crates/workspace/src/persistence.rs`

- [ ] **Step 1: Add a serializable session payload to the persistence model**

```rust
// crates/workspace/src/persistence/model.rs
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoredWorkspaceSession {
    pub session_id: String,
    pub title: String,
    pub root_paths: Vec<String>,
    pub workspace_id: i64,
    pub active_item_id: Option<u64>,
    pub active_terminal_item_id: Option<u64>,
    pub active_agent_thread_id: Option<String>,
}

pub struct MultiWorkspaceState {
    pub sidebar_open: bool,
    pub sidebar_state: Option<String>,
    pub active_workspace_id: Option<i64>,
    pub sessions: Vec<StoredWorkspaceSession>,
}
```

- [ ] **Step 2: Convert runtime session state into persisted state**

```rust
// crates/workspace/src/multi_workspace.rs
impl MultiWorkspace {
    pub fn serialize_sessions(&self) -> Vec<StoredWorkspaceSession> {
        self.sessions
            .iter()
            .map(|session| StoredWorkspaceSession {
                session_id: session.session_id.0.clone(),
                title: session.title.clone(),
                root_paths: session
                    .root_paths
                    .iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect(),
                workspace_id: session.workspace_id.to_proto(),
                active_item_id: session.active_item_id,
                active_terminal_item_id: session.active_terminal_item_id,
                active_agent_thread_id: session.active_agent_thread_id.clone(),
            })
            .collect()
    }
}
```

- [ ] **Step 3: Restore sessions during multi-workspace restoration**

```rust
// crates/workspace/src/multi_workspace.rs
impl MultiWorkspace {
    pub fn restore_sessions(&mut self, sessions: Vec<StoredWorkspaceSession>) {
        self.sessions = sessions
            .into_iter()
            .map(|session| WorkspaceSessionState {
                session_id: WorkspaceSessionId(session.session_id),
                title: session.title,
                root_paths: session.root_paths.into_iter().map(Into::into).collect(),
                workspace_id: WorkspaceId::from_proto(session.workspace_id),
                active_item_id: session.active_item_id,
                active_terminal_item_id: session.active_terminal_item_id,
                active_agent_thread_id: session.active_agent_thread_id,
            })
            .collect();
    }
}
```

- [ ] **Step 4: Extend a persistence round-trip test**

```rust
#[test]
fn multi_workspace_state_round_trips_sessions() {
    let state = MultiWorkspaceState {
        sidebar_open: true,
        sidebar_state: Some("sidebar".into()),
        active_workspace_id: Some(7),
        sessions: vec![StoredWorkspaceSession {
            session_id: "workspace-7".into(),
            title: "zed".into(),
            root_paths: vec!["/tmp/zed".into()],
            workspace_id: 7,
            active_item_id: Some(11),
            active_terminal_item_id: Some(13),
            active_agent_thread_id: Some("thread-1".into()),
        }],
    };

    let json = serde_json::to_string(&state).unwrap();
    let restored: MultiWorkspaceState = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.sessions.len(), 1);
    assert_eq!(restored.sessions[0].title, "zed");
}
```

- [ ] **Step 5: Run the persistence test target**

Run: `cargo test -p workspace multi_workspace_state_round_trips_sessions`

Expected: PASS

### Task 4: Add the sidebar session switcher UI

**Files:**
- Create: `crates/sidebar/src/session_switcher.rs`
- Modify: `crates/sidebar/src/sidebar.rs`
- Modify: `crates/sidebar/src/sidebar_tests.rs`
- Test: `crates/sidebar/src/sidebar_tests.rs`

- [ ] **Step 1: Add the session switcher view**

```rust
// crates/sidebar/src/session_switcher.rs
use gpui::*;
use workspace::{MultiWorkspace, WorkspaceSessionId};

pub struct SessionSwitcher {
    multi_workspace: WeakEntity<MultiWorkspace>,
}

impl SessionSwitcher {
    pub fn new(multi_workspace: WeakEntity<MultiWorkspace>) -> Self {
        Self { multi_workspace }
    }
}
```

- [ ] **Step 2: Render session rows from `MultiWorkspace` state**

```rust
impl Render for SessionSwitcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(multi_workspace) = self.multi_workspace.upgrade() else {
            return div();
        };

        multi_workspace.read_with(cx, |mw, _| {
            v_flex()
                .gap_1()
                .children(mw.sessions().iter().map(|session| {
                    let selected = mw.active_session_id() == Some(&session.session_id);
                    button_like(session.title.clone())
                        .selected(selected)
                        .on_click(cx.listener({
                            let session_id = session.session_id.clone();
                            move |_, _, window, cx| {
                                multi_workspace.update(cx, |mw, cx| {
                                    mw.activate_session(&session_id, window, cx);
                                });
                            }
                        }))
                }))
        })
    }
}
```

- [ ] **Step 3: Mount the switcher inside the existing sidebar**

```rust
// crates/sidebar/src/sidebar.rs
use crate::session_switcher::SessionSwitcher;

pub struct Sidebar {
    // existing fields...
    session_switcher: Entity<SessionSwitcher>,
}
```

- [ ] **Step 4: Prefer session switching near the top of the sidebar**

```rust
// crates/sidebar/src/sidebar.rs render path
v_flex()
    .child(self.render_header(window, cx))
    .child(self.session_switcher.clone())
    .child(self.render_existing_content(window, cx))
```

- [ ] **Step 5: Add a behavioral sidebar test**

```rust
#[gpui::test]
async fn test_sidebar_renders_session_entries_for_open_workspaces(cx: &mut TestAppContext) {
    let (multi_workspace, cx) = workspace::MultiWorkspace::test_new(project.clone(), cx);
    let sidebar = setup_sidebar(&multi_workspace, cx);

    multi_workspace.read_with(cx, |mw, _| {
        assert!(!mw.sessions().is_empty());
    });

    sidebar.read_with(cx, |sidebar, _| {
        assert!(sidebar.has_session_switcher_for_tests());
    });
}
```

- [ ] **Step 6: Run the sidebar test group**

Run: `cargo test -p sidebar session -- --nocapture`

Expected: PASS

### Task 5: Restore visible context when switching

**Files:**
- Modify: `crates/workspace/src/multi_workspace.rs`
- Modify: `crates/workspace/src/workspace.rs`
- Modify: `crates/agent_ui/src/agent_panel.rs` (only if an existing stable thread-selection entry point already exists)
- Test: `crates/sidebar/src/sidebar_tests.rs`

- [ ] **Step 1: Restore the active pane item first**

```rust
// crates/workspace/src/multi_workspace.rs
fn restore_active_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(session_id) = self.active_session_id.clone() else {
        return;
    };
    let Some(session) = self.sessions.iter().find(|session| session.session_id == session_id) else {
        return;
    };

    let workspace = self.workspace().clone();
    workspace.update(cx, |workspace, cx| {
        if let Some(item_id) = session.active_item_id {
            workspace.activate_item_by_id(item_id, window, cx);
        }
    });
}
```

- [ ] **Step 2: Restore terminal focus only through an existing item path**

```rust
// crates/workspace/src/multi_workspace.rs
if let Some(terminal_item_id) = session.active_terminal_item_id {
    workspace.activate_item_by_id(terminal_item_id, window, cx);
}
```

- [ ] **Step 3: Restore agent thread only if `AgentPanel` already has a stable public selector**

```rust
// crates/agent_ui/src/agent_panel.rs
impl AgentPanel {
    pub fn reveal_thread_for_session_restore(
        &mut self,
        thread_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_saved_thread_by_id(thread_id, window, cx);
    }
}
```

- [ ] **Step 4: Add an end-to-end switching test**

```rust
#[gpui::test]
async fn test_switching_sessions_restores_the_last_active_context(cx: &mut TestAppContext) {
    let (multi_workspace, cx) = workspace::MultiWorkspace::test_new(project.clone(), cx);

    // Arrange two workspaces, activate different items, switch away and back.
    // Assert the original active item is restored after reactivation.

    multi_workspace.read_with(cx, |mw, _| {
        assert!(mw.active_session_id().is_some());
    });
}
```

- [ ] **Step 5: Run the focused restoration test**

Run: `cargo test -p sidebar test_switching_sessions_restores_the_last_active_context -- --nocapture`

Expected: PASS

### Task 6: Verify the POC with the smallest useful integration pass

**Files:**
- Modify: `crates/zed/src/zed.rs`
- Test: existing workspace/sidebar test targets

- [ ] **Step 1: Wire startup so the session switcher appears in normal app startup**

```rust
// crates/zed/src/zed.rs
// Keep existing sidebar registration, but ensure session state is initialized
// when multi-workspace and sidebar are created.
initialize_workspace_sessions(&multi_workspace, cx);
```

- [ ] **Step 2: Run the targeted crate tests**

Run: `cargo test -p workspace -p sidebar`

Expected: PASS

- [ ] **Step 3: Run the project lints for touched crates**

Run: `./script/clippy workspace sidebar agent_ui terminal_view`

Expected: PASS

- [ ] **Step 4: Manual QA in a dev build**

Run: `cargo run -p zed`

Expected workflow:

1. Open multiple folders or projects so multiple workspaces exist.
2. Confirm the sidebar shows one session per open workspace.
3. Open different files in different sessions.
4. If terminal and agent restoration landed, confirm those contexts come back after switching.
5. Confirm switching does not destroy the hidden session's visible state.

## Spec coverage check

- **Multiple persistent project contexts:** covered by Tasks 1-3.
- **Sidebar session switcher:** covered by Task 4.
- **Fast switching and restoration:** covered by Task 5.
- **Pragmatic overlay on existing systems:** enforced by file choices and the explicit non-goal of universal tab unification.

## First slice recommendation

If implementation time is tight, stop after:

1. Task 2 with file restoration only, and
2. Task 4 with the sidebar switcher.

That smaller slice is still enough to prove whether users benefit from persistent project-scoped session switching before investing in terminal or agent restoration.
