use wxp_daemon::handlers::normalize_route_action;

#[test]
fn goto_back_reload_actions_map_to_automator_calls() {
    assert_eq!(normalize_route_action("goto"), "navigateTo");
    assert_eq!(normalize_route_action("back"), "navigateBack");
    assert_eq!(normalize_route_action("reload"), "reLaunch");
}
