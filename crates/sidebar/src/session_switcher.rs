use gpui::{App, Context, Entity, Render, SharedString, Subscription, WeakEntity};
use ui::{ButtonLike, ButtonSize, ButtonStyle, Color, Icon, TintColor, prelude::*};
use workspace::{MultiWorkspace, workspace_session::WorkspaceSessionId};

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionSwitcherEntry {
    session_id: WorkspaceSessionId,
    title: SharedString,
    is_active: bool,
}

pub(crate) struct SessionSwitcher {
    multi_workspace: WeakEntity<MultiWorkspace>,
    _subscriptions: Vec<Subscription>,
}

impl SessionSwitcher {
    pub fn new(multi_workspace: Entity<MultiWorkspace>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&multi_workspace, |_this, _multi_workspace, cx| {
            cx.notify();
        });

        Self {
            multi_workspace: multi_workspace.downgrade(),
            _subscriptions: vec![subscription],
        }
    }

    fn entries(&self, cx: &App) -> Vec<SessionSwitcherEntry> {
        let Some(multi_workspace) = self.multi_workspace.upgrade() else {
            return Vec::new();
        };

        multi_workspace.read_with(cx, |multi_workspace, _cx| {
            let active_session_id = multi_workspace.active_session_id();

            multi_workspace
                .sessions()
                .iter()
                .map(|session| SessionSwitcherEntry {
                    session_id: session.session_id.clone(),
                    title: SharedString::from(session.title.clone()),
                    is_active: active_session_id == Some(&session.session_id),
                })
                .collect()
        })
    }

    pub(super) fn activate_session(
        &mut self,
        session_id: WorkspaceSessionId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(multi_workspace) = self.multi_workspace.upgrade() else {
            return;
        };

        multi_workspace.update(cx, |multi_workspace, cx| {
            multi_workspace.activate_session(&session_id, window, cx);
        });
    }

    #[cfg(test)]
    pub(super) fn entries_for_tests(&self, cx: &App) -> Vec<(String, bool)> {
        self.entries(cx)
            .into_iter()
            .map(|entry| (entry.title.to_string(), entry.is_active))
            .collect()
    }
}

impl Render for SessionSwitcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = self.entries(cx);

        if entries.is_empty() {
            return div().id("sidebar-session-switcher").into_any_element();
        }

        v_flex()
            .id("sidebar-session-switcher")
            .px_1p5()
            .py_1()
            .gap_0p5()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .children(entries.into_iter().map(|entry| {
                let session_id = entry.session_id.clone();

                ButtonLike::new(SharedString::from(format!(
                    "workspace-session-{}",
                    entry.session_id.0
                )))
                .full_width()
                .size(ButtonSize::Compact)
                .style(ButtonStyle::Subtle)
                .when(entry.is_active, |this| {
                    this.toggle_state(true)
                        .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                })
                .child(
                    h_flex()
                        .w_full()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .child(Label::new(entry.title.clone())),
                        )
                        .when(entry.is_active, |this| {
                            this.child(Icon::new(IconName::Check).color(Color::Accent))
                        }),
                )
                .on_click(cx.listener(move |this: &mut Self, _, window, cx| {
                    this.activate_session(session_id.clone(), window, cx);
                }))
                .into_any_element()
            }))
            .into_any_element()
    }
}
