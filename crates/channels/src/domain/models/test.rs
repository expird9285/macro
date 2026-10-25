use super::ReferencedShareItemType;

#[test]
fn from_raw_rejects_unknown_types() {
    assert_eq!(ReferencedShareItemType::from_raw("unknown"), None);
    assert_eq!(ReferencedShareItemType::from_raw(""), None);
}
