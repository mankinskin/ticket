use super::*;

fn t(id: &str, title: &str, state: &str) -> TicketSummary {
    TicketSummary::new(
        id.into(),
        "tracker-improvement".into(),
        title.into(),
        state.into(),
    )
}

fn e(from: &str, to: &str) -> EdgeRecord {
    EdgeRecord::new(from.into(), to.into(), "depends_on".into())
}

#[test]
fn version_is_non_empty() {
    assert!(!core_version().is_empty());
}

// Host-kind gates
#[test]
fn host_kind_gates() {
    assert!(supports_server_control(HostKind::DesktopNode));
    assert!(supports_server_control(HostKind::RemoteWorkspace));
    assert!(!supports_server_control(HostKind::BrowserWeb));
    assert!(!supports_server_control(HostKind::Virtual));
    assert!(supports_browser_bridge(HostKind::DesktopNode));
    assert!(!supports_browser_bridge(HostKind::RemoteWorkspace));
    assert!(supports_file_browsing(HostKind::DesktopNode));
    assert!(supports_file_browsing(HostKind::RemoteWorkspace));
    assert!(!supports_file_browsing(HostKind::BrowserWeb));
    assert!(!supports_file_browsing(HostKind::Virtual));
}

// Filtering
#[test]
fn filter_by_state() {
    let ticket = t("a", "Add feature", "planned");
    assert!(ticket_matches(&ticket, "planned", ""));
    assert!(!ticket_matches(&ticket, "done", ""));
    assert!(ticket_matches(&ticket, "", ""));
}
#[test]
fn filter_by_query() {
    let ticket = t("abc123", "Add feature", "planned");
    assert!(ticket_matches(&ticket, "", "feature"));
    assert!(ticket_matches(&ticket, "", "FEATURE"));
    assert!(!ticket_matches(&ticket, "", "missing"));
    assert!(ticket_matches(&ticket, "", "abc"));
}
#[test]
fn filter_combined() {
    let ticket = t("a", "Add feature", "planned");
    assert!(ticket_matches(&ticket, "planned", "feature"));
    assert!(!ticket_matches(&ticket, "done", "feature"));
}

// Dependency maps
#[test]
fn dependency_maps_basic() {
    let tickets = vec![t("a", "Parent", "planned"), t("b", "Child", "planned")];
    let edges = vec![e("a", "b")];
    let maps = DependencyMaps::build(&tickets, &edges);
    assert_eq!(maps.deps_of["a"], vec!["b"]);
    assert_eq!(maps.parent_of["b"], vec!["a"]);
    assert!(!maps.deps_of.contains_key("b"));
    assert!(!maps.parent_of.contains_key("a"));
}
#[test]
fn dependency_maps_skips_unknown() {
    let tickets = vec![t("a", "A", "planned")];
    let edges = vec![e("a", "unknown")];
    let maps = DependencyMaps::build(&tickets, &edges);
    assert!(!maps.deps_of.contains_key("a"));
}
#[test]
fn dependency_maps_skips_non_depends_on() {
    let tickets = vec![t("a", "A", "planned"), t("b", "B", "planned")];
    let edges = vec![EdgeRecord::new("a".into(), "b".into(), "linked".into())];
    let maps = DependencyMaps::build(&tickets, &edges);
    assert!(!maps.deps_of.contains_key("a"));
}

// State grouping
#[test]
fn state_groups_roots() {
    let tickets = vec![
        t("a", "Parent", "planned"),
        t("b", "Child", "planned"),
        t("c", "Done", "done"),
    ];
    let edges = vec![e("a", "b")];
    let groups = build_state_groups(&tickets, &edges, &[], "", "");
    let done = groups.iter().find(|g| g.state == "done").unwrap();
    let ready = groups.iter().find(|g| g.state == "planned").unwrap();
    assert_eq!(done.total, 1);
    assert_eq!(ready.total, 2);
    assert_eq!(ready.root_ids, vec!["a"]);
}
#[test]
fn state_groups_schema_order() {
    let tickets = vec![t("a", "A", "done"), t("b", "B", "planned")];
    let order: Vec<String> = vec!["planned".into(), "done".into()];
    let groups = build_state_groups(&tickets, &[], &order, "", "");
    assert_eq!(groups[0].state, "planned");
    assert_eq!(groups[1].state, "done");
}
#[test]
fn state_groups_state_filter() {
    let tickets = vec![t("a", "A", "planned"), t("b", "B", "done")];
    let groups = build_state_groups(&tickets, &[], &[], "planned", "");
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].state, "planned");
}
#[test]
fn state_groups_query_filter() {
    let tickets = vec![
        t("a", "Alpha feature", "planned"),
        t("b", "Beta thing", "planned"),
    ];
    let groups = build_state_groups(&tickets, &[], &[], "", "alpha");
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].root_ids, vec!["a"]);
}

// URL / intent
#[test]
fn url_format() {
    assert_eq!(
        ticket_viewer_url("http://localhost:3002", "default", "abc123"),
        "http://localhost:3002/?workspace=default&ticket=abc123"
    );
}
#[test]
fn display_label_with_title() {
    assert_eq!(ticket_display_label("abc123", "My ticket"), "My ticket");
}
#[test]
fn display_label_no_title() {
    assert_eq!(ticket_display_label("abcdef1234", ""), "(abcdef12)");
}
#[test]
fn display_label_short_id() {
    assert_eq!(ticket_display_label("ab", ""), "(ab)");
}
