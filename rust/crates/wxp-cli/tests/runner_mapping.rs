use serde_json::json;
use wxp_cli::command::CliCommand;
use wxp_cli::runner::build_rpc_call;

#[test]
fn maps_goto_to_route_param() {
    let call = build_rpc_call(&CliCommand::Goto {
        route: "/pages/order/index".into(),
        timeout_ms: 10_000,
    })
    .expect("call");

    assert_eq!(call.method, "goto");
    assert_eq!(call.params, json!({ "route": "/pages/order/index" }));
}

#[test]
fn maps_type_ref_string_to_numeric_ref() {
    let call = build_rpc_call(&CliCommand::Type {
        reference: "%2".into(),
        text: "13800138000".into(),
    })
    .expect("call");

    assert_eq!(call.method, "type");
    assert_eq!(call.params, json!({ "ref": 2, "text": "13800138000" }));
}

#[test]
fn maps_net_log_with_expected_defaults() {
    let call = build_rpc_call(&CliCommand::NetLog {
        filter: None,
        limit: 20,
        with_body: false,
        json: false,
    })
    .expect("call");

    assert_eq!(call.method, "net.log");
    assert_eq!(
        call.params,
        json!({ "filter": null, "limit": 20, "withBody": false })
    );
}

#[test]
fn rejects_invalid_ref_format() {
    let error = build_rpc_call(&CliCommand::Tap {
        reference: "2".into(),
    })
    .expect_err("invalid ref should fail");

    assert_eq!(error.to_string(), "ref must be in %N format");
}
