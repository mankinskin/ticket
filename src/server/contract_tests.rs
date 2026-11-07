use super::*;

#[test]
fn advertised_ticket_contracts_distinguish_workspace_defaults_and_query_filters() {
    let tools = TicketServer::tool_router().list_all();
    assert_eq!(tools.len(), 42);
    for tool in &tools {
        let value = serde_json::to_value(tool).unwrap();
        if value["inputSchema"]["properties"]["workspace"]["type"] == "string" {
            assert!(
                value["inputSchema"]["properties"]["workspace"]["description"]
                    .as_str()
                    .is_some(),
                "{} lacks workspace field contract",
                tool.name
            );
        }
    }
    let list = tools
        .iter()
        .find(|tool| tool.name == "list_tickets")
        .unwrap();
    assert!(
        list.description
            .as_deref()
            .unwrap()
            .contains("query ignores state/type")
    );
    let get = tools.iter().find(|tool| tool.name == "get_ticket").unwrap();
    assert!(
        get.description
            .as_deref()
            .unwrap()
            .contains("Omitted/blank workspace")
    );
    let close = tools
        .iter()
        .find(|tool| tool.name == "close_ticket")
        .unwrap();
    assert!(
        close
            .description
            .as_deref()
            .unwrap()
            .contains("explicit workspace")
    );
}
