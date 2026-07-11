use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};

use crate::app::{AppState, FocusedPanel};
use crate::storage::{Confidence, Materialization, StorageEdgeKind, StorageKind, StorageNode};

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState) {
    state.panel_rects.insert(FocusedPanel::Topology, area);
    let inventory = &state.storage_inventory;
    let availability = if inventory.partial {
        "PARTIAL — last-known graph may be incomplete"
    } else if inventory.nodes.is_empty() {
        "EMPTY — no block nodes discovered"
    } else {
        "AVAILABLE"
    };
    let title = format!(
        " Topology Overview · source=sysfs · {availability} · stacked counters are NOT additive "
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = inner.height.saturating_sub(1) as usize;
    let max_scroll = inventory.nodes.len().saturating_sub(visible);
    state.topology_scroll = state.topology_scroll.min(max_scroll);

    let header = Row::new([
        "Node",
        "Layer",
        "Protocol",
        "Media",
        "Confidence",
        "Generation",
        "Relations",
    ])
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let rows = inventory
        .nodes
        .iter()
        .skip(state.topology_scroll)
        .take(visible)
        .map(|node| {
            let relations = relation_summary(state, node);
            let values = topology_values(node, relations);
            Row::new(values.into_iter().map(Cell::from))
        });
    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(11),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(11),
            Constraint::Length(12),
            Constraint::Min(12),
        ],
    )
    .header(header)
    .column_spacing(1);
    frame.render_widget(table, inner);

    if inventory.nodes.len() > visible {
        let mut scrollbar =
            ScrollbarState::new(inventory.nodes.len()).position(state.topology_scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar,
        );
    }
}

fn topology_values(node: &StorageNode, relations: String) -> [String; 7] {
    [
        node.name.clone(),
        materialization_label(node.materialization).into(),
        kind_label(node.kind).into(),
        removable_label(node.removable).into(),
        confidence_label(node).into(),
        generation_label(node),
        relations,
    ]
}

fn relation_summary(state: &AppState, node: &StorageNode) -> String {
    let mut contains = 0;
    let mut backed_by = 0;
    let mut member_of = 0;
    for edge in &state.storage_inventory.edges {
        if edge.from == node.id || edge.to == node.id {
            match edge.kind {
                StorageEdgeKind::ContainsPartition => contains += 1,
                StorageEdgeKind::BackedBy => backed_by += 1,
                StorageEdgeKind::MemberOf => member_of += 1,
            }
        }
    }
    format!("part:{contains} back:{backed_by} md:{member_of}")
}

fn kind_label(kind: StorageKind) -> &'static str {
    match kind {
        StorageKind::ScsiLike => "scsi-like",
        StorageKind::Nvme => "nvme",
        StorageKind::Mmc => "mmc",
        StorageKind::MdRaid => "md",
        StorageKind::DeviceMapper => "dm",
        StorageKind::Virtual => "virtual",
        StorageKind::Other => "unknown",
    }
}

fn materialization_label(value: Materialization) -> &'static str {
    match value {
        Materialization::BlockDevice => "whole",
        Materialization::Partition => "partition",
        Materialization::Stacked => "stacked",
        Materialization::Virtual => "virtual",
        Materialization::Unknown => "unknown",
    }
}

fn removable_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "removable",
        Some(false) => "fixed",
        None => "unknown",
    }
}

fn confidence_label(node: &StorageNode) -> &'static str {
    if node
        .identities
        .iter()
        .any(|claim| claim.confidence == Confidence::High)
    {
        "high"
    } else if node
        .identities
        .iter()
        .any(|claim| claim.confidence == Confidence::Medium)
    {
        "medium"
    } else {
        "low"
    }
}

fn generation_label(node: &StorageNode) -> String {
    format!(
        "seq:{} dev:{}",
        if node.generation.diskseq.is_some() {
            "yes"
        } else {
            "no"
        },
        if node.generation.dev_t.is_some() {
            "yes"
        } else {
            "no"
        }
    )
}

#[cfg(test)]
mod tests {
    use crate::storage::{Generation, IdentityClaim, IdentityScope, IdentitySource, StorageNode};

    use super::*;

    #[test]
    fn topology_row_does_not_render_identity_claim_values() {
        let node = StorageNode {
            id: "block:test0".into(),
            name: "test0".into(),
            kind: StorageKind::Nvme,
            materialization: Materialization::BlockDevice,
            removable: Some(false),
            identities: vec![IdentityClaim {
                value: "sensitive-identity".into(),
                scope: IdentityScope::BlockDevice,
                source: IdentitySource::DevNumber,
                confidence: Confidence::Medium,
            }],
            generation: Generation {
                diskseq: Some(42),
                dev_t: Some((259, 0)),
            },
        };

        let row = topology_values(&node, "part:0 back:0 md:0".into()).join(" ");
        assert!(!row.contains("sensitive-identity"));
        assert!(!row.contains("259:0"));
        assert!(!row.contains("42"));
        assert!(row.contains("seq:yes dev:yes"));
    }
}
