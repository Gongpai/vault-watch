use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table,
    },
};

use crate::app::{AppState, FocusedPanel};
use crate::storage::{Confidence, Materialization, StorageEdgeKind, StorageKind, StorageNode};
use crate::widgets::scroll;

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

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(6)])
        .split(inner);
    let table_area = sections[0];
    let detail_area = sections[1];

    let visible = table_area.height.saturating_sub(1) as usize;
    let max_scroll = scroll::max_offset(inventory.nodes.len(), visible);
    state.topology_scroll = state.topology_scroll.min(max_scroll);
    state.topology_selected = state
        .topology_selected
        .min(inventory.nodes.len().saturating_sub(1));

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
        .enumerate()
        .map(|(relative_index, node)| {
            let relations = relation_summary(state, node);
            let values = topology_values(node, relations);
            let row = Row::new(values.into_iter().map(Cell::from));
            if state.topology_scroll + relative_index == state.topology_selected {
                row.style(Style::default().bg(Color::DarkGray))
            } else {
                row
            }
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
    frame.render_widget(table, table_area);

    render_node_details(frame, detail_area, state);

    if inventory.nodes.len() > visible {
        let mut scrollbar =
            ScrollbarState::new(scroll::position_count(inventory.nodes.len(), visible))
                .position(state.topology_scroll);
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

fn render_node_details(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Selected Node · privacy-safe details ")
        .borders(Borders::TOP);
    let Some(node) = state.storage_inventory.nodes.get(state.topology_selected) else {
        frame.render_widget(Paragraph::new("No node selected").block(block), area);
        return;
    };
    let (availability, source, scope) = health_observation(state, node);
    let relations = relation_summary(state, node);
    let lines = vec![
        Line::from(vec![
            Span::styled("Node: ", Style::default().fg(Color::DarkGray)),
            Span::styled(node.name.clone(), Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "  Layer: {}  Protocol: {}  Media: {}",
                materialization_label(node.materialization),
                kind_label(node.kind),
                removable_label(node.removable)
            )),
        ]),
        Line::from(format!(
            "Health availability: {}  Source: {source}  Scope: {scope}",
            availability.label()
        )),
        Line::from(format!(
            "Topology confidence: {}  Generation: {}",
            confidence_label(node),
            generation_label(node)
        )),
        Line::from(format!(
            "Relations: {relations}  Identity values: REDACTED by UI policy"
        )),
    ];
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn health_observation(
    state: &AppState,
    node: &StorageNode,
) -> (crate::app::MetricAvailability, &'static str, &'static str) {
    use crate::app::{MetricAvailability, RaidAvailability};

    if state.storage_inventory.partial {
        return (
            MetricAvailability::Stale,
            "sysfs-topology",
            "last-known-node",
        );
    }

    match node.kind {
        StorageKind::MdRaid => match state.raid_availability {
            RaidAvailability::Complete => (MetricAvailability::Available, "md-sysfs", "array"),
            RaidAvailability::Partial | RaidAvailability::Unavailable => (
                MetricAvailability::TemporarilyUnavailable,
                "md-sysfs",
                "array",
            ),
        },
        _ if node.materialization == Materialization::BlockDevice => {
            match state.disks.iter().find(|disk| disk.device == node.name) {
                Some(disk) => (disk.health_availability, "legacy-smart", "whole-device"),
                None => (
                    MetricAvailability::Unsupported,
                    "topology-only",
                    "whole-device",
                ),
            }
        }
        _ if node.materialization == Materialization::Stacked => {
            (MetricAvailability::Hidden, "topology-only", "stacked")
        }
        _ => (
            MetricAvailability::Unsupported,
            "topology-only",
            materialization_label(node.materialization),
        ),
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
    use crate::security::SecurityPosture;
    use crate::storage::model::{Generation, IdentityClaim, IdentityScope, IdentitySource};
    use crate::storage::{StorageInventory, StorageNode};

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

    #[test]
    fn node_details_keep_array_and_partition_availability_scoped() {
        let partition = StorageNode {
            id: "block:test0p1".into(),
            name: "test0p1".into(),
            kind: StorageKind::Nvme,
            materialization: Materialization::Partition,
            removable: None,
            identities: Vec::new(),
            generation: Generation::default(),
        };
        let md = StorageNode {
            id: "block:array-test".into(),
            name: "array-test".into(),
            kind: StorageKind::MdRaid,
            materialization: Materialization::Stacked,
            removable: Some(false),
            identities: Vec::new(),
            generation: Generation::default(),
        };
        let inventory = StorageInventory {
            nodes: vec![partition.clone(), md.clone()],
            edges: Vec::new(),
            partial: false,
        };
        let mut state = AppState::new(Vec::new(), inventory, SecurityPosture::new(false));

        assert_eq!(
            health_observation(&state, &partition),
            (
                crate::app::MetricAvailability::Unsupported,
                "topology-only",
                "partition"
            )
        );
        state.raid_availability = crate::app::RaidAvailability::Partial;
        assert_eq!(
            health_observation(&state, &md),
            (
                crate::app::MetricAvailability::TemporarilyUnavailable,
                "md-sysfs",
                "array"
            )
        );

        state.storage_inventory.partial = true;
        assert_eq!(
            health_observation(&state, &md),
            (
                crate::app::MetricAvailability::Stale,
                "sysfs-topology",
                "last-known-node"
            )
        );
    }
}
