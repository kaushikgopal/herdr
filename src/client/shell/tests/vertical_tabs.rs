use super::*;

fn strip_state() -> ClientShellState {
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.vertical_tabs = true;
    ClientShellState::new(config)
}

fn strip_agent(pane_id: &str, tab_id: &str, status: AgentStatus) -> ClientShellAgent {
    ClientShellAgent {
        pane_id: pane_id.into(),
        workspace_id: "ws_1".into(),
        tab_id: tab_id.into(),
        name: None,
        display_agent: Some("pi".into()),
        agent: Some("pi".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: status,
        state_change_seq: 1,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: false,
    }
}

fn second_tab() -> ClientShellTab {
    ClientShellTab {
        tab_id: "tab_2".into(),
        workspace_id: "ws_1".into(),
        number: 2,
        label: "second".into(),
        custom_label: true,
        zoomed: false,
        focused: false,
        agent_status: AgentStatus::Idle,
    }
}

fn strip_text(state: &mut ClientShellState, cols: u16, rows: u16) -> String {
    state
        .compose(cols, rows)
        .expect("compose strip")
        .cells
        .chunks(cols as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn strip_renders_title_agent_badge_and_context_line() {
    let mut snapshot = snapshot();
    snapshot
        .agents
        .push(strip_agent("pane_1", "tab_1", AgentStatus::Working));
    let mut state = strip_state();
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state.compose(80, 24).expect("compose strip");

    assert_eq!(state.hits.tabs.len(), 1);
    let (rect, tab_id) = &state.hits.tabs[0];
    assert_eq!(tab_id, "tab_1");
    assert_eq!(rect.height, 2, "clickable region: title + context");

    let text = strip_text(&mut state, 80, 24);
    assert!(text.contains('●'), "agent status icon missing: {text}");
    assert!(text.contains(" pi "), "agent name missing: {text}");
    assert!(text.contains(" • "), "context line missing: {text}");
    assert!(!text.contains(" spaces"), "spaces header leaked: {text}");
}

#[test]
fn strip_agent_name_hides_when_the_strip_is_narrow() {
    let mut snapshot = snapshot();
    snapshot
        .agents
        .push(strip_agent("pane_1", "tab_1", AgentStatus::Working));
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.vertical_tabs = true;
    config.sidebar_width = 8;
    config.sidebar_min_width = 5;
    config.sidebar_max_width = 8;
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state.compose(80, 24).expect("compose narrow strip");

    let text = strip_text(&mut state, 80, 24);
    assert!(text.contains('●'), "status icon should survive: {text}");
    assert!(!text.contains("pi "), "agent name should yield: {text}");
}

#[test]
fn compact_strip_drops_the_context_line() {
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.vertical_tabs = true;
    config.vertical_tabs_compact = true;
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(80, 24).expect("compose compact strip");

    let (rect, _) = &state.hits.tabs[0];
    assert_eq!(rect.height, 1, "clickable region: title only");
    let text = strip_text(&mut state, 80, 24);
    assert!(!text.contains(" • "), "context line should be gone: {text}");
}

#[test]
fn extra_panes_render_below_the_tab_line_inside_the_group() {
    let mut snapshot = snapshot();
    let mut second_pane = snapshot.panes[0].clone();
    second_pane.pane_id = "pane_2".into();
    second_pane.cwd = Some("/repo/feature".into());
    second_pane.foreground_cwd = Some("/repo/feature".into());
    second_pane.focused = false;
    snapshot.panes.push(second_pane);
    let mut state = strip_state();
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state.compose(80, 24).expect("compose multi-pane strip");

    let (rect, _) = &state.hits.tabs[0];
    assert_eq!(
        rect.height, 2,
        "clickable region stays title + context even with panes"
    );
    let text = strip_text(&mut state, 80, 24);
    assert!(text.contains("feature"), "pane cwd title missing: {text}");

    let pane_hit = state
        .hits
        .agents
        .iter()
        .find(|(rect, pane_id)| pane_id == "pane_2" && rect.height == 1);
    assert!(pane_hit.is_some(), "pane line hit rect missing");

    let (pane_rect, _) = pane_hit.expect("pane hit");
    let down = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: pane_rect.x + 1,
        row: pane_rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let [ClientShellAction::Endpoint { request, .. }] = &down.actions[..] else {
        panic!("pane click should use endpoint API");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::PaneFocus(params) if params.pane_id == "pane_2"
    ));
}

#[test]
fn clicking_a_strip_entry_focuses_the_tab() {
    let mut snapshot = snapshot();
    snapshot.tabs.push(second_tab());
    let mut state = strip_state();
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state.compose(80, 24).expect("compose strip");

    let rect = state.hits.tabs[1].0;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 1,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: rect.x + 1,
            row: rect.y,
            modifiers: KeyModifiers::empty(),
        })]);
    let [ClientShellAction::Endpoint { request, .. }] = &release.actions[..] else {
        panic!("strip click should use endpoint API");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::TabFocus(params) if params.tab_id == "tab_2"
    ));
}

#[test]
fn strip_overflow_scroll_buttons_scroll_groups() {
    // The strip scrolls by workspace group, so overflow needs several.
    let mut snapshot = snapshot();
    let ws_template = snapshot.workspaces[0].clone();
    let tab_template = snapshot.tabs[0].clone();
    for number in 2..=10 {
        snapshot.workspaces.push(ClientShellWorkspace {
            workspace_id: format!("ws_{number}"),
            active_tab_id: format!("tab_{number}"),
            number,
            label: format!("space-{number}"),
            focused: false,
            ..ws_template.clone()
        });
        snapshot.tabs.push(ClientShellTab {
            tab_id: format!("tab_{number}"),
            workspace_id: format!("ws_{number}"),
            number: 1,
            label: number.to_string(),
            ..tab_template.clone()
        });
    }
    let mut state = strip_state();
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state.compose(80, 6).expect("compose overflow strip");

    assert!(state.hits.tab_scroll_right.width > 0);
    let scroll_down = state.hits.tab_scroll_right;
    let outcome =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: scroll_down.x,
            row: scroll_down.y,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(outcome.repaint);
    assert_eq!(state.tab_scroll, 1);
}

#[test]
fn strip_groups_every_workspaces_tabs_under_one_box() {
    let mut snapshot = snapshot();
    let mut second = second_tab();
    second.workspace_id = "ws_2".into();
    snapshot.tabs.push(second);
    let template = snapshot.workspaces[0].clone();
    snapshot.workspaces.push(ClientShellWorkspace {
        workspace_id: "ws_2".into(),
        active_tab_id: "tab_2".into(),
        number: 2,
        label: "space-two".into(),
        focused: false,
        ..template.clone()
    });
    let mut state = strip_state();
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state.compose(80, 24).expect("compose grouped strip");

    assert_eq!(state.hits.tabs.len(), 2, "tabs from both workspaces");
    let (_, second_id) = &state.hits.tabs[1];
    assert_eq!(second_id, "tab_2");

    let text = strip_text(&mut state, 80, 24);
    assert!(
        text.contains("space-two"),
        "workspace header missing: {text}"
    );
}

#[test]
fn strip_wheel_cycles_tabs_across_workspaces() {
    let mut snapshot = snapshot();
    let mut second = second_tab();
    second.workspace_id = "ws_2".into();
    snapshot.tabs.push(second);
    let template = snapshot.workspaces[0].clone();
    snapshot.workspaces.push(ClientShellWorkspace {
        workspace_id: "ws_2".into(),
        active_tab_id: "tab_2".into(),
        number: 2,
        label: "space-two".into(),
        focused: false,
        ..template.clone()
    });
    let mut state = strip_state();
    state.set_snapshot(Box::new(snapshot.clone()));
    state.set_pane_surface(surface());
    state.compose(80, 24).expect("compose strip");

    // Wheel down over the first tab: cycles into ws_2's tab.
    let rect = state.hits.tabs[0].0;
    let down = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: rect.x + 1,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let [ClientShellAction::Endpoint { request, .. }] = &down.actions[..] else {
        panic!("strip wheel should use endpoint API");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::TabFocus(params) if params.tab_id == "tab_2"
    ));

    // Simulate the server focusing tab_2 (the last tab), then wheel down
    // again: clamped, no request.
    snapshot.focused_tab_id = Some("tab_2".into());
    snapshot.focused_workspace_id = Some("ws_2".into());
    for tab in &mut snapshot.tabs {
        tab.focused = tab.tab_id == "tab_2";
    }
    for workspace in &mut snapshot.workspaces {
        workspace.focused = workspace.workspace_id == "ws_2";
    }
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state.compose(80, 24).expect("recompose strip");

    let second_rect = state.hits.tabs[1].0;
    let clamped =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: second_rect.x + 1,
            row: second_rect.y,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(clamped.actions.is_empty(), "clamped at the last tab");
}
