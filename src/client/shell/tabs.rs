use super::*;

const TAB_SCROLL_BUTTON_WIDTH: u16 = 3;
const MIN_TAB_STRIP_WIDTH: u16 =
    MIN_TAB_WIDTH + NEW_TAB_WIDTH + TAB_SCROLL_BUTTON_WIDTH.saturating_mul(2);

pub(crate) fn render_tab_bar(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    tab_scroll: &mut usize,
    reveal_focused_tab: &mut bool,
    tab_drag_insert_index: Option<usize>,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    buffer.set_style(area, Style::default().bg(palette.panel_bg));
    let tabs = snapshot
        .tabs
        .iter()
        .filter(|tab| Some(tab.workspace_id.as_str()) == snapshot.focused_workspace_id.as_deref())
        .collect::<Vec<_>>();
    let desired_widths = tabs
        .iter()
        .map(|tab| {
            let label = tab_label(tab);
            display_width(&label).saturating_add(4).max(MIN_TAB_WIDTH)
        })
        .collect::<Vec<_>>();
    let content = tab_bar_content_area(snapshot, area);
    let mouse_chrome = config.mouse_capture;
    let new_tab_width = if mouse_chrome { NEW_TAB_WIDTH } else { 0 };
    let desired_total = desired_widths
        .iter()
        .copied()
        .fold(0_u16, u16::saturating_add)
        .saturating_add(tabs.len().saturating_sub(1).min(u16::MAX as usize) as u16)
        .saturating_add(new_tab_width);
    let overflow =
        desired_total > content.width && (!mouse_chrome || content.width >= MIN_TAB_STRIP_WIDTH);
    let available = if overflow && mouse_chrome {
        content
            .width
            .saturating_sub(NEW_TAB_WIDTH)
            .saturating_sub(TAB_SCROLL_BUTTON_WIDTH.saturating_mul(2))
    } else {
        content.width.saturating_sub(new_tab_width)
    };
    let max_scroll = max_tab_scroll(&desired_widths, available);
    if !overflow {
        *tab_scroll = 0;
    } else if *reveal_focused_tab {
        if let Some(focused) = tabs.iter().position(|tab| tab.focused) {
            *tab_scroll = centered_tab_scroll(focused, &desired_widths, available).min(max_scroll);
        }
    } else {
        *tab_scroll = (*tab_scroll).min(max_scroll);
    }
    *reveal_focused_tab = false;

    let mut x = content.x;
    let tab_right = if overflow && mouse_chrome {
        hits.tab_scroll_left = Rect::new(
            content.x,
            content.y,
            TAB_SCROLL_BUTTON_WIDTH.min(content.width),
            1,
        );
        put_text(
            buffer,
            hits.tab_scroll_left.x,
            content.y,
            hits.tab_scroll_left.width,
            " < ",
            Style::default()
                .fg(if *tab_scroll > 0 {
                    palette.overlay1
                } else {
                    palette.overlay0
                })
                .bg(palette.surface0),
        );
        x = hits.tab_scroll_left.right();
        content
            .right()
            .saturating_sub(NEW_TAB_WIDTH + TAB_SCROLL_BUTTON_WIDTH)
    } else {
        content.right().saturating_sub(new_tab_width)
    };

    let mut first_visible = None;
    let mut last_visible = None;
    for (index, tab) in tabs.iter().enumerate().skip(*tab_scroll) {
        let name = tab_label(tab);
        let desired = desired_widths[index];
        let remaining = tab_right.saturating_sub(x);
        let width = desired.min(remaining);
        if width == 0 {
            break;
        }
        let rect = Rect::new(x, area.y, width, 1);
        let style = if tab.focused {
            let base = Style::default()
                .fg(panel_contrast_fg(palette))
                .bg(palette.accent);
            if tab.custom_label {
                base.add_modifier(Modifier::BOLD)
            } else {
                base
            }
        } else if tab.custom_label {
            Style::default().fg(palette.overlay1).bg(palette.surface0)
        } else {
            Style::default()
                .fg(palette.overlay0)
                .bg(palette.surface0)
                .add_modifier(Modifier::DIM)
        };
        let padding = width.saturating_sub(display_width(&name));
        let left = padding / 2;
        let text = format!(
            "{empty:left$}{name}{empty:right_padding$}",
            empty = "",
            left = left as usize,
            right_padding = padding.saturating_sub(left) as usize,
        );
        put_text(buffer, rect.x, rect.y, rect.width, &text, style);
        hits.tabs.push((rect, tab.tab_id.clone()));
        first_visible.get_or_insert(index);
        last_visible = Some(index);
        x = x.saturating_add(width + 1);
        if width < desired {
            break;
        }
    }

    if overflow && mouse_chrome {
        hits.tab_scroll_right = Rect::new(tab_right, area.y, TAB_SCROLL_BUTTON_WIDTH, 1);
        let can_scroll_right = last_visible.is_some_and(|index| index + 1 < tabs.len());
        put_text(
            buffer,
            hits.tab_scroll_right.x,
            area.y,
            hits.tab_scroll_right.width,
            " > ",
            Style::default()
                .fg(if can_scroll_right {
                    palette.overlay1
                } else {
                    palette.overlay0
                })
                .bg(palette.surface0),
        );
        hits.new_tab = Rect::new(
            hits.tab_scroll_right.right(),
            area.y,
            content
                .right()
                .saturating_sub(hits.tab_scroll_right.right())
                .min(NEW_TAB_WIDTH),
            1,
        );
    } else if mouse_chrome {
        hits.new_tab = Rect::new(
            x.min(content.right()),
            area.y,
            content.right().saturating_sub(x).min(NEW_TAB_WIDTH),
            1,
        );
    }
    if mouse_chrome {
        put_text(
            buffer,
            hits.new_tab.x,
            area.y,
            hits.new_tab.width,
            " + ",
            Style::default().fg(palette.overlay1).bg(palette.panel_bg),
        );
    }

    if first_visible.is_some_and(|index| index > 0) {
        let ellipsis_x = if hits.tab_scroll_left.width > 0 {
            hits.tab_scroll_left.right()
        } else {
            content.x
        };
        put_text(
            buffer,
            ellipsis_x,
            area.y,
            u16::from(ellipsis_x < content.right()),
            "…",
            Style::default().fg(palette.overlay0),
        );
    }
    if last_visible.is_some_and(|index| index + 1 < tabs.len()) {
        let ellipsis_x = if hits.tab_scroll_right.width > 0 {
            hits.tab_scroll_right.x.saturating_sub(1)
        } else {
            content.right().saturating_sub(1)
        };
        put_text(
            buffer,
            ellipsis_x,
            area.y,
            u16::from(ellipsis_x >= content.x && ellipsis_x < content.right()),
            "…",
            Style::default().fg(palette.overlay0),
        );
    }

    if let Some(insert_index) = tab_drag_insert_index {
        if let Some(indicator_x) = tab_drop_indicator_x(hits, &tabs, insert_index) {
            put_text(
                buffer,
                indicator_x.min(content.right().saturating_sub(1)),
                area.y,
                1,
                "│",
                Style::default().fg(palette.accent),
            );
        }
    }
    render_tab_bar_status(buffer, area, snapshot, palette);
}

pub(crate) fn tab_bar_status_width(snapshot: &ClientShellSnapshot) -> u16 {
    let content = snapshot.tab_bar_right.iter().fold(0u16, |width, segment| {
        width.saturating_add(display_width(&segment.text))
    });
    let separators = snapshot.tab_bar_right.len().saturating_sub(1);
    content.saturating_add(
        display_width(&snapshot.tab_bar_right_separator)
            .saturating_mul(separators.min(u16::MAX as usize) as u16),
    )
}

fn tab_bar_status_area(snapshot: &ClientShellSnapshot, area: Rect) -> Option<Rect> {
    let width = tab_bar_status_width(snapshot);
    if width == 0 {
        return None;
    }
    let reserved = width.saturating_add(1);
    (area.width.saturating_sub(reserved) >= MIN_TAB_STRIP_WIDTH)
        .then(|| Rect::new(area.right().saturating_sub(width), area.y, width, 1))
}

fn tab_bar_content_area(snapshot: &ClientShellSnapshot, area: Rect) -> Rect {
    let reserved = tab_bar_status_area(snapshot, area)
        .map(|status| status.width.saturating_add(1))
        .unwrap_or(0);
    Rect {
        width: area.width.saturating_sub(reserved),
        ..area
    }
}

fn render_tab_bar_status(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    palette: &Palette,
) {
    let Some(status) = tab_bar_status_area(snapshot, area) else {
        return;
    };
    let separator_width = display_width(&snapshot.tab_bar_right_separator);
    let mut x = status.x;
    for (index, segment) in snapshot.tab_bar_right.iter().enumerate() {
        if index > 0 && separator_width > 0 {
            put_text(
                buffer,
                x,
                area.y,
                separator_width,
                &snapshot.tab_bar_right_separator,
                Style::default().fg(palette.overlay0).bg(palette.panel_bg),
            );
            x = x.saturating_add(separator_width);
        }
        let width = display_width(&segment.text);
        let style = if segment.accent {
            Style::default()
                .fg(panel_contrast_fg(palette))
                .bg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.overlay1).bg(palette.panel_bg)
        };
        put_text(buffer, x, area.y, width, &segment.text, style);
        x = x.saturating_add(width);
    }
}

fn tab_drop_indicator_x(
    hits: &ShellHitMap,
    tabs: &[&ClientShellTab],
    insert_index: usize,
) -> Option<u16> {
    let visible = hits
        .tabs
        .iter()
        .filter_map(|(rect, tab_id)| {
            tabs.iter()
                .position(|tab| tab.tab_id == *tab_id)
                .map(|index| (index, *rect))
        })
        .collect::<Vec<_>>();
    let (first_index, first_rect) = *visible.first()?;
    let (last_index, last_rect) = *visible.last()?;
    if insert_index == 0 {
        return Some(if first_index == 0 {
            first_rect.x
        } else {
            hits.tab_scroll_left.right()
        });
    }
    if let Some((_, rect)) = visible.iter().find(|(index, _)| *index == insert_index) {
        return Some(rect.x.saturating_sub(1));
    }
    if insert_index >= tabs.len() {
        return Some(if last_index + 1 >= tabs.len() {
            last_rect.right()
        } else {
            hits.tab_scroll_right.x.saturating_sub(1)
        });
    }
    None
}

fn centered_tab_scroll(focused: usize, widths: &[u16], available: u16) -> usize {
    let mut best = focused;
    let mut best_distance = u16::MAX;
    for start in 0..=focused {
        let before = widths
            .iter()
            .copied()
            .enumerate()
            .skip(start)
            .take(focused.saturating_sub(start))
            .fold(0u16, |width, (_, tab)| width.saturating_add(tab + 1));
        if before >= available {
            continue;
        }
        let focused_width = widths[focused].min(available.saturating_sub(before));
        let center = before.saturating_mul(2).saturating_add(focused_width);
        let distance = center.abs_diff(available);
        if distance <= best_distance {
            best_distance = distance;
            best = start;
        }
    }
    best
}

fn max_tab_scroll(widths: &[u16], available: u16) -> usize {
    (0..widths.len())
        .find(|start| last_visible_tab(*start, widths, available) == widths.len().checked_sub(1))
        .unwrap_or(0)
}

fn last_visible_tab(start: usize, widths: &[u16], available: u16) -> Option<usize> {
    let mut remaining = available;
    let mut last = None;
    for (index, width) in widths.iter().copied().enumerate().skip(start) {
        if remaining == 0 {
            break;
        }
        last = Some(index);
        if width >= remaining {
            break;
        }
        remaining = remaining.saturating_sub(width.saturating_add(1));
    }
    last
}

fn tab_label(tab: &ClientShellTab) -> String {
    if tab.zoomed {
        format!("{} Z", tab.label)
    } else {
        tab.label.clone()
    }
}
const TAB_STRIP_CHROME_HEIGHT: u16 = 1;
struct TabStripEntry<'a> {
    tab: &'a ClientShellTab,
    workspace: &'a ClientShellWorkspace,
    panes: Vec<&'a crate::protocol::ClientShellPane>,
    agents_by_pane: &'a HashMap<&'a str, Vec<&'a crate::protocol::ClientShellAgent>>,
}

impl TabStripEntry<'_> {
    fn line_count(&self, compact: bool) -> u16 {
        let pane_lines = self
            .panes
            .len()
            .saturating_sub(1)
            .min(u16::MAX as usize - 3) as u16;
        1 + u16::from(!compact) + pane_lines
    }

    /// The pane whose info populates the tab line: the one hosting the
    /// highest-priority agent, else the focused pane, else the first pane.
    fn primary_pane(&self) -> Option<&crate::protocol::ClientShellPane> {
        // rev(): max_by_key prefers the last of equal keys, but tab-title
        // primacy follows pane order — first pane wins ties.
        self.panes.iter().copied().rev().max_by_key(|pane| {
            self.pane_agent(pane)
                .map(|agent| (status_priority(agent.agent_status), agent.state_change_seq))
                .unwrap_or((0, 0))
        })
    }

    fn pane_agent(
        &self,
        pane: &crate::protocol::ClientShellPane,
    ) -> Option<&crate::protocol::ClientShellAgent> {
        self.agents_by_pane
            .get(pane.pane_id.as_str())?
            .iter()
            .max_by_key(|agent| (status_priority(agent.agent_status), agent.state_change_seq))
            .copied()
    }
}

struct TabStripGroup<'a> {
    workspace: &'a ClientShellWorkspace,
    tabs: Vec<TabStripEntry<'a>>,
}

impl TabStripGroup<'_> {
    /// One rounded box wraps the whole group: content rows + two borders.
    fn height(&self, compact: bool) -> u16 {
        self.tabs
            .iter()
            .map(|tab| tab.line_count(compact))
            .sum::<u16>()
            .saturating_add(2)
    }
}

pub(crate) fn render_tab_strip(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    state: &mut ShellRenderState<'_>,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    // The group boxes delineate the strip themselves; no sidebar separator —
    // past the last box it read as a stray border line.
    buffer.set_style(area, Style::default().bg(palette.sidebar_bg));
    if area.width == 0 || area.height == 0 {
        return;
    }

    let content = if config.mouse_capture {
        Rect::new(
            area.x,
            area.y,
            area.width,
            area.height.saturating_sub(TAB_STRIP_CHROME_HEIGHT),
        )
    } else {
        area
    };

    let focused_tabs = snapshot
        .tabs
        .iter()
        .filter(|tab| Some(tab.workspace_id.as_str()) == snapshot.focused_workspace_id.as_deref())
        .collect::<Vec<_>>();
    let mut panes_by_tab: HashMap<&str, Vec<&crate::protocol::ClientShellPane>> = HashMap::new();
    for pane in &snapshot.panes {
        panes_by_tab
            .entry(pane.tab_id.as_str())
            .or_default()
            .push(pane);
    }
    let agents_by_pane: HashMap<&str, Vec<&crate::protocol::ClientShellAgent>> = snapshot
        .agents
        .iter()
        .fold(HashMap::new(), |mut map, agent| {
            map.entry(agent.pane_id.as_str()).or_default().push(agent);
            map
        });

    // One box per workspace: its tabs render inside the group's border.
    let groups: Vec<TabStripGroup<'_>> = snapshot
        .workspaces
        .iter()
        .map(|workspace| TabStripGroup {
            workspace,
            tabs: snapshot
                .tabs
                .iter()
                .filter(|tab| tab.workspace_id == workspace.workspace_id)
                .map(|tab| TabStripEntry {
                    tab,
                    workspace,
                    panes: panes_by_tab.remove(tab.tab_id.as_str()).unwrap_or_default(),
                    agents_by_pane: &agents_by_pane,
                })
                .collect(),
        })
        .collect();
    let row_heights = groups
        .iter()
        .map(|group| group.height(config.vertical_tabs_compact))
        .collect::<Vec<_>>();

    let max_scroll = max_tab_strip_scroll(&row_heights, content.height);
    if !overflow_tab_strip(&row_heights, content.height) {
        *state.tab_scroll = 0;
    } else if *state.reveal_focused_tab {
        if let Some(focused) = groups
            .iter()
            .position(|group| group.tabs.iter().any(|entry| entry.tab.focused))
        {
            *state.tab_scroll =
                strip_scroll_revealing(*state.tab_scroll, focused, &row_heights, content.height)
                    .min(max_scroll);
        }
    } else {
        *state.tab_scroll = (*state.tab_scroll).min(max_scroll);
    }
    *state.reveal_focused_tab = false;

    let mut y = content.y;
    let mut last_visible = None;
    for (index, group) in groups.iter().enumerate().skip(*state.tab_scroll) {
        let row_height = row_heights[index];
        let remaining = content.bottom().saturating_sub(y);
        let height = row_height.min(remaining);
        if height == 0 {
            break;
        }
        render_tab_strip_group(
            buffer,
            Rect::new(content.x, y, content.width, height),
            group,
            config,
            hits,
        );
        last_visible = Some(index);
        y = y.saturating_add(height);
        if height < row_height {
            break;
        }
    }

    if config.mouse_capture {
        render_tab_strip_chrome(
            buffer,
            area,
            overflow_tab_strip(&row_heights, content.height),
            last_visible.is_some_and(|index| index + 1 < groups.len()),
            palette,
            hits,
        );
    }

    if let Some(insert_index) = state.tab_drag_insert_index {
        if let Some(indicator_y) = tab_drop_indicator_y(hits, &focused_tabs, insert_index) {
            let indicator_y = indicator_y.min(content.bottom().saturating_sub(1));
            put_text(
                buffer,
                content.x,
                indicator_y,
                content.width,
                &"─".repeat(content.width as usize),
                Style::default().fg(palette.accent),
            );
        }
    }
}

/// Text padding inside strip rows: text keeps a column of breathing room
/// from the box borders while the highlight runs flush beneath them.
const STRIP_TEXT_INSET: u16 = 1;

/// Inset a strip row's text area by [`STRIP_TEXT_INSET`] on each side.
fn strip_text_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x.saturating_add(STRIP_TEXT_INSET),
        rect.y,
        rect.width.saturating_sub(2 * STRIP_TEXT_INSET),
        rect.height,
    )
}

/// One workspace group: a single rounded box around all its tabs. Each tab
/// renders a title row (workspace label right-aligned), a details row
/// (cwd leaf • branch + agent badge), then one line per extra pane in the
/// same details format.
fn render_tab_strip_group(
    buffer: &mut Buffer,
    rect: Rect,
    group: &TabStripGroup<'_>,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    let border_style = Style::default().fg(if group.workspace.focused {
        palette.overlay0
    } else {
        palette.surface_dim
    });
    let inner = draw_strip_box(buffer, rect, border_style);
    if inner.is_empty() {
        return;
    }

    let compact = config.vertical_tabs_compact;
    let mut line_y = inner.y;
    for entry in &group.tabs {
        let active = entry.tab.focused;
        let primary = entry.primary_pane();

        // Title line. The tab hit region stops before the pane lines so
        // pane rows can route clicks to their own pane.
        if line_y >= inner.bottom() {
            break;
        }
        let tab_hit_height = (1 + u16::from(!compact)).min(inner.bottom().saturating_sub(line_y));
        let title_rect = Rect::new(inner.x, line_y, inner.width, 1);
        if active {
            // Muted highlight: the same surface the inactive tabs used to
            // get. It bleeds under the adjacent border cells so the block
            // reads flush with the box; glyphs stay drawn on top.
            let highlight = Style::default().bg(palette.surface0);
            let block_height = entry
                .line_count(compact)
                .min(inner.bottom().saturating_sub(line_y));
            buffer.set_style(
                Rect::new(inner.x, line_y, inner.width, tab_hit_height),
                highlight,
            );
            buffer.set_style(Rect::new(rect.x, line_y, 1, block_height), highlight);
            buffer.set_style(
                Rect::new(rect.right().saturating_sub(1), line_y, 1, block_height),
                highlight,
            );
            if line_y == inner.y {
                buffer.set_style(Rect::new(rect.x, rect.y, rect.width, 1), highlight);
            }
            if line_y + entry.line_count(compact) >= inner.bottom() {
                buffer.set_style(
                    Rect::new(rect.x, rect.bottom().saturating_sub(1), rect.width, 1),
                    highlight,
                );
            }
        }
        hits.tabs.push((
            Rect::new(inner.x, line_y, inner.width, tab_hit_height),
            entry.tab.tab_id.clone(),
        ));
        let title_style = if active {
            Style::default()
                .fg(palette.text)
                .add_modifier(Modifier::BOLD)
        } else if entry.tab.custom_label {
            Style::default().fg(palette.overlay1)
        } else {
            Style::default()
                .fg(palette.overlay0)
                .add_modifier(Modifier::DIM)
        };
        if compact {
            // No details row: the agent badge stays on the title line.
            let badge = primary
                .and_then(|pane| entry.pane_agent(pane))
                .map(|agent| (strip_agent_name(agent), agent.agent_status));
            render_strip_line_with_badge(
                buffer,
                strip_text_rect(title_rect),
                &[tab_label(entry.tab)],
                title_style,
                badge,
                config,
            );
        } else {
            // Per-pane facts (cwd, agent) live on the details row, so the
            // title row stays meaningful with any number of panes: the tab
            // title plus the workspace it belongs to.
            render_strip_title_row(
                buffer,
                strip_text_rect(title_rect),
                &tab_label(entry.tab),
                title_style,
                entry.workspace.label.as_str(),
                config,
            );
        }
        line_y = line_y.saturating_add(1);

        // Details row: cwd leaf • branch, then the primary pane's agent
        // badge. The leaf comes first (the identity); the branch drops
        // first when the strip is narrow.
        if !compact {
            if line_y >= inner.bottom() {
                break;
            }
            let context_rect = Rect::new(inner.x, line_y, inner.width, 1);
            if active {
                buffer.set_style(context_rect, Style::default().bg(palette.surface0));
            }
            let badge = primary
                .and_then(|pane| entry.pane_agent(pane))
                .map(|agent| (strip_agent_name(agent), agent.agent_status));
            let leaf = primary.and_then(strip_cwd_leaf);
            let segments = detail_segments(leaf.as_deref(), entry.workspace.branch.as_deref());
            render_strip_line_with_badge(
                buffer,
                strip_text_rect(context_rect),
                &segments,
                Style::default()
                    .fg(palette.overlay0)
                    .add_modifier(Modifier::DIM),
                badge,
                config,
            );
            line_y = line_y.saturating_add(1);
        }

        // Secondary panes, one line each in the same details format with
        // their own agent badge. Clicks focus that pane (the hit map
        // routes agents to PaneFocus).
        for pane in &entry.panes {
            if primary.is_some_and(|primary| primary.pane_id == pane.pane_id) {
                continue;
            }
            if line_y >= inner.bottom() {
                break;
            }
            let pane_rect = Rect::new(inner.x, line_y, inner.width, 1);
            if active {
                buffer.set_style(pane_rect, Style::default().bg(palette.surface0));
            }
            let pane_badge = entry
                .pane_agent(pane)
                .map(|agent| (strip_agent_name(agent), agent.agent_status));
            let pane_style = if pane.focused {
                Style::default().fg(palette.overlay1)
            } else {
                Style::default()
                    .fg(palette.overlay0)
                    .add_modifier(Modifier::DIM)
            };
            let mut segments = match pane.label.as_deref().filter(|label| !label.is_empty()) {
                Some(label) => vec![label.to_owned()],
                None => vec![strip_cwd_leaf(pane).unwrap_or_else(|| "pane".to_owned())],
            };
            if let Some(branch) = &entry.workspace.branch {
                segments.push(branch.clone());
            }
            render_strip_line_with_badge(
                buffer,
                strip_text_rect(pane_rect),
                &segments,
                pane_style,
                pane_badge,
                config,
            );
            hits.agents.push((pane_rect, pane.pane_id.clone()));
            line_y = line_y.saturating_add(1);
        }
    }
}

fn strip_agent_name(agent: &crate::protocol::ClientShellAgent) -> Option<&str> {
    agent
        .display_agent
        .as_deref()
        .or(agent.name.as_deref())
        .or(agent.agent.as_deref())
        .or(agent.title.as_deref())
        .filter(|name| !name.is_empty())
}

/// Widest cwd leaf the strip shows: 8 columns, tail ellipsized.
const MAX_CWD_LEAF_WIDTH: u16 = 8;

/// The pane's cwd leaf — the only part of the path a strip row has room
/// for — capped to `MAX_CWD_LEAF_WIDTH` columns by ellipsizing the tail.
fn strip_cwd_leaf(pane: &crate::protocol::ClientShellPane) -> Option<String> {
    let cwd = pane
        .foreground_cwd
        .as_deref()
        .or(pane.cwd.as_deref())?
        .trim_end_matches('/');
    let base = cwd.rsplit('/').next().filter(|base| !base.is_empty())?;
    Some(ellipsize_tail(base, MAX_CWD_LEAF_WIDTH))
}

/// Truncate to `max` display columns, replacing the tail with '…'.
fn ellipsize_tail(text: &str, max: u16) -> String {
    use unicode_width::UnicodeWidthChar as _;
    if display_width(text) <= max {
        return text.to_owned();
    }
    let budget = usize::from(max.saturating_sub(1));
    let mut kept = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let width = ch.width().unwrap_or(0);
        if used + width > budget {
            break;
        }
        used += width;
        kept.push(ch);
    }
    kept.push('…');
    kept
}

/// Details-row segments: cwd leaf first (the identity), branch second —
/// rightmost drops first when the strip is narrow.
fn detail_segments(leaf: Option<&str>, branch: Option<&str>) -> Vec<String> {
    let mut segments = Vec::new();
    if let Some(leaf) = leaf {
        segments.push(leaf.to_owned());
    }
    if let Some(branch) = branch {
        segments.push(branch.to_owned());
    }
    segments
}

/// Left text built from `segments` joined by " • " (rightmost segment
/// dropped first when narrow); badge right-aligned, the agent name
/// yielding before its status icon.
fn render_strip_line_with_badge(
    buffer: &mut Buffer,
    rect: Rect,
    segments: &[String],
    title_style: Style,
    badge: Option<(Option<&str>, crate::api::schema::AgentStatus)>,
    config: &ClientShellConfig,
) {
    let palette = &config.palette;
    const MIN_TITLE_WIDTH: u16 = 4;
    let icon = badge
        .map(|(_, status)| status_icon(status, config.status_indicators))
        .unwrap_or_default();
    let icon_width = u16::from(!icon.is_empty());
    let name = badge.and_then(|(name, _)| name).unwrap_or_default();
    let name_width = display_width(name);
    let show_name = icon_width > 0
        && name_width > 0
        && rect.width >= icon_width + name_width + 1 + MIN_TITLE_WIDTH;
    let right_width = if show_name {
        name_width + 1 + icon_width
    } else {
        icon_width
    };
    let title_width = rect
        .width
        .saturating_sub(right_width)
        .min(rect.width.saturating_sub(1));
    let mut count = segments.len();
    let text = loop {
        let text = segments[..count].join(" • ");
        if display_width(&text) <= title_width || count <= 1 {
            break text;
        }
        count -= 1;
    };
    put_text(buffer, rect.x, rect.y, title_width, &text, title_style);

    let badge_status = badge.map(|(_, status)| status);
    if right_width == 0 {
        return;
    }
    let mut x = rect.right().saturating_sub(right_width);
    if show_name {
        x = put_segment(
            buffer,
            x,
            rect.y,
            rect.right(),
            name,
            Style::default().fg(palette.overlay0),
        );
        x = put_segment(buffer, x, rect.y, rect.right(), " ", title_style);
    }
    put_segment(
        buffer,
        x,
        rect.y,
        rect.right(),
        icon,
        Style::default().fg(status_color(
            badge_status.unwrap_or(crate::api::schema::AgentStatus::Unknown),
            palette,
        )),
    );
}

/// Title left-aligned; workspace label right-aligned and dim, hidden
/// before the title drops below its minimum width. The label sits where
/// the agent badge used to: per-pane facts moved to the details row.
fn render_strip_title_row(
    buffer: &mut Buffer,
    rect: Rect,
    title: &str,
    title_style: Style,
    workspace_label: &str,
    config: &ClientShellConfig,
) {
    const MIN_TITLE_WIDTH: u16 = 4;
    let label_width = display_width(workspace_label);
    let show_label = label_width > 0 && rect.width >= label_width + 1 + MIN_TITLE_WIDTH;
    let title_width = if show_label {
        rect.width.saturating_sub(label_width + 1)
    } else {
        rect.width
    };
    put_text(buffer, rect.x, rect.y, title_width, title, title_style);
    if show_label {
        let style = Style::default()
            .fg(config.palette.overlay0)
            .add_modifier(Modifier::DIM);
        put_right_text(buffer, rect, rect.y, workspace_label, style);
    }
}

/// Rounded, muted box; returns the inner content rect. Falls back to the
/// rect itself when there is no room for a box.
fn draw_strip_box(buffer: &mut Buffer, rect: Rect, style: Style) -> Rect {
    if rect.width < 5 || rect.height < 3 {
        return rect;
    }
    let inner = Rect::new(rect.x + 1, rect.y + 1, rect.width - 2, rect.height - 2);
    for x in inner.x..inner.right() {
        put_text(buffer, x, rect.y, 1, "─", style);
        put_text(buffer, x, rect.bottom().saturating_sub(1), 1, "─", style);
    }
    for y in inner.y..inner.bottom() {
        put_text(buffer, rect.x, y, 1, "│", style);
        put_text(buffer, rect.right().saturating_sub(1), y, 1, "│", style);
    }
    put_text(buffer, rect.x, rect.y, 1, "╭", style);
    put_text(
        buffer,
        rect.right().saturating_sub(1),
        rect.y,
        1,
        "╮",
        style,
    );
    put_text(
        buffer,
        rect.x,
        rect.bottom().saturating_sub(1),
        1,
        "╰",
        style,
    );
    put_text(
        buffer,
        rect.right().saturating_sub(1),
        rect.bottom().saturating_sub(1),
        1,
        "╯",
        style,
    );
    inner
}

fn render_tab_strip_chrome(
    buffer: &mut Buffer,
    area: Rect,
    overflow: bool,
    can_scroll_down: bool,
    palette: &Palette,
    hits: &mut ShellHitMap,
) {
    let chrome_y = area.bottom().saturating_sub(1);
    hits.new_tab = Rect::new(area.x, chrome_y, NEW_TAB_WIDTH.min(area.width), 1);
    put_text(
        buffer,
        hits.new_tab.x,
        chrome_y,
        hits.new_tab.width,
        " + ",
        Style::default().fg(palette.overlay1).bg(palette.panel_bg),
    );
    if !overflow {
        return;
    }
    hits.tab_scroll_left = Rect::new(area.right().saturating_sub(2), chrome_y, 1, 1);
    hits.tab_scroll_right = Rect::new(area.right().saturating_sub(1), chrome_y, 1, 1);
    put_text(
        buffer,
        hits.tab_scroll_left.x,
        chrome_y,
        1,
        "▲",
        Style::default().fg(palette.overlay1).bg(palette.surface0),
    );
    put_text(
        buffer,
        hits.tab_scroll_right.x,
        chrome_y,
        1,
        "▼",
        Style::default()
            .fg(if can_scroll_down {
                palette.overlay1
            } else {
                palette.overlay0
            })
            .bg(palette.surface0),
    );
}

fn overflow_tab_strip(heights: &[u16], available: u16) -> bool {
    max_tab_strip_scroll(heights, available) > 0
}

fn max_tab_strip_scroll(heights: &[u16], available: u16) -> usize {
    (0..heights.len())
        .find(|start| {
            last_visible_entry(*start, heights, available) == heights.len().checked_sub(1)
        })
        .unwrap_or(0)
}

fn last_visible_entry(start: usize, heights: &[u16], available: u16) -> Option<usize> {
    let mut remaining = available;
    let mut last = None;
    for (index, height) in heights.iter().copied().enumerate().skip(start) {
        if remaining == 0 {
            break;
        }
        last = Some(index);
        if height >= remaining {
            break;
        }
        remaining = remaining.saturating_sub(height);
    }
    last
}

fn strip_scroll_revealing(
    current: usize,
    focused: usize,
    heights: &[u16],
    available: u16,
) -> usize {
    let mut start = current.min(focused);
    while start < focused
        && last_visible_entry(start, heights, available).is_none_or(|last| last < focused)
    {
        start += 1;
    }
    start
}

fn tab_drop_indicator_y(
    hits: &ShellHitMap,
    tabs: &[&ClientShellTab],
    insert_index: usize,
) -> Option<u16> {
    let visible = hits
        .tabs
        .iter()
        .filter_map(|(rect, tab_id)| {
            tabs.iter()
                .position(|tab| tab.tab_id == *tab_id)
                .map(|index| (index, *rect))
        })
        .collect::<Vec<_>>();
    let (first_index, first_rect) = *visible.first()?;
    let (last_index, last_rect) = *visible.last()?;
    if insert_index == 0 {
        return Some(if first_index == 0 {
            first_rect.y
        } else {
            hits.tab_scroll_left.y
        });
    }
    if let Some((_, rect)) = visible.iter().find(|(index, _)| *index == insert_index) {
        return Some(rect.y.saturating_sub(1));
    }
    if insert_index >= tabs.len() {
        return Some(if last_index + 1 >= tabs.len() {
            last_rect.bottom()
        } else {
            hits.tab_scroll_right.y.saturating_sub(1)
        });
    }
    None
}
