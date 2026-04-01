use wxp_ref_store::RefStore;

#[test]
fn old_ref_expires_after_new_snapshot_version() {
    let mut store = RefStore::default();
    store.replace_snapshot("page/home", 1, vec![(1, ".btn")]);
    store.replace_snapshot("page/home", 2, vec![(1, ".next")]);
    assert!(store.resolve(1, 1).is_none());
    assert_eq!(store.resolve(1, 2).as_deref(), Some(".next"));
}
