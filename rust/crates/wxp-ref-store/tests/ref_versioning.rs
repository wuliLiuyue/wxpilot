use wxp_ref_store::RefStore;

#[test]
fn replacing_snapshot_invalidates_previous_refs() {
    let mut store = RefStore::default();
    store.replace_snapshot("pages/index/index", 1, vec![(1, "page > input")]);
    store.replace_snapshot("pages/index/index", 2, vec![(1, "page > button")]);

    assert!(store.resolve(1, 1).is_none());
    assert_eq!(store.resolve(1, 2).as_deref(), Some("page > button"));
}
