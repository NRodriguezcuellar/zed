# Agent-first workspace sessions POC

## Summary

This POC explores whether Zed should optimize for multiple persistent project contexts rather than a single global ACP agent surface.

The idea is to introduce **workspace sessions**: folder-scoped working sets that preserve a mix of agent threads, terminals, files, and the active surface so users can jump between parallel efforts without reconstructing state.

The POC should validate workflow value first and avoid a deep architectural rewrite until the behavior proves useful in daily use.

## Problem

The current model makes it easy to have an agent available, but it does not yet prove the workflow needed by users who work across several active contexts at once. In practice, productivity may depend less on having one prominent agent surface and more on being able to keep several folder-based working sets alive and switch between them quickly.

The risk is spending time on a large layout or abstraction rewrite before confirming the core product hypothesis.

## Product hypothesis

Users working with ACP agents are more productive when the editor supports **multiple persistent project contexts**, each with its own open agents, terminals, files, and active tab state.

If switching between these contexts is fast and stateful, users will stay in Zed more often instead of bouncing between separate editor, terminal, and agent workflows.

## Goals

- Prove that one agent tab is not enough for real multitasking workflows.
- Prove that users benefit from several persistent, project-scoped working contexts.
- Make switching between contexts feel instant and stateful.
- Preserve enough file, terminal, and agent state to support real usage over multiple days.

## Non-goals

- Fully unify agents, terminals, and files under one new internal abstraction.
- Replace Zed's current workspace, pane, or dock architecture.
- Perfect the final UI or naming.
- Solve every persistence edge case before the workflow is validated.

## Proposed design

### Core concept: workspace session

A **workspace session** is a project- or folder-scoped container for active work.

Each session owns:

- a project root or folder association,
- a set of open files,
- a set of terminal sessions,
- a set of ACP agent threads or agent surfaces,
- the currently active surface and recent navigation state.

The user experience should be that each session is a distinct mental context that can be suspended and resumed.

### UI shape

- A **sidebar session switcher** lists active workspace sessions.
- The **main area** shows the selected session's current tab set and active surface.
- Agents are first-class surfaces within a session, but the main feature is the session itself, not a single privileged agent view.

This keeps the POC focused on concurrency and context restoration rather than on making ACP the only dominant surface.

### Implementation direction

Build the POC as a pragmatic layer on top of existing Zed workspace infrastructure.

Use the current layout and state systems where possible:

- reuse existing workspace and pane behavior for files,
- reuse existing terminal state and surfaces,
- reuse existing agent/thread state where feasible,
- add session ownership and switching behavior above those pieces.

For the POC, it is acceptable if agents, terminals, and files are not all backed by the same internal type, as long as they present as one coherent per-session experience.

## Why this direction

This is the least wasteful path because it tests the product hypothesis without first paying the cost of re-architecting `AgentPanel` and related UI into a perfect universal tab model.

The codebase already has separate abstractions for dock panels, pane items, sidebar containers, and session-like workspace state. The POC should use those seams to learn quickly instead of flattening them prematurely.

## Alternatives considered

### 1. Single center-first agent mode

Make ACP the dominant center surface and treat code and terminals as secondary.

Why not first: this tests prominence of one agent, not whether users need multiple parallel contexts.

### 2. Full peer-tab architecture immediately

Make agents, terminals, and files all first-class peers in one unified pane/tab model.

Why not first: this is the cleanest long-term architecture, but it is also the most expensive and highest-risk path for an initial POC.

## Success criteria

The POC succeeds if a user can keep roughly 3-5 active project contexts alive and switch between them with minimal setup cost while preserving enough state to continue work immediately.

Concretely, a successful POC should make it easy to return to a session and find:

- the expected active file or files,
- the expected terminal session state,
- the expected agent thread or agent context,
- the expected selected tab or surface.

The POC fails if switching sessions still feels like reopening tools manually, or if one important surface type repeatedly falls out of the model.

## Technical constraints and risks

- Live agent UX is currently panel-oriented, while files are pane items and terminals can appear through more than one path.
- Persistence is split across multiple crates and subsystems.
- A deep internal unification effort could consume the POC without answering the product question.
- Focus, restoration, and switching semantics may be more important than layout polish.

These risks are acceptable only if the first implementation stays narrow and centered on session state ownership.

## Suggested POC slices

1. Introduce a lightweight session model bound to a folder/root.
2. Add a sidebar switcher for active sessions.
3. Persist and restore a minimal set of surfaces per session.
4. Validate real usage with several concurrent sessions.
5. Only then decide whether agents need to become true peer tabs internally.

## Open questions for implementation planning

- What is the smallest viable persisted state for a session?
- Should sessions map directly onto existing workspace concepts or sit above them?
- Which existing agent surface is best suited for session restoration in a POC?
- How much terminal fidelity is required for the workflow to feel real?
- Should session creation be explicit or emerge from opening folders/projects?
