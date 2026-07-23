use crate::github::input::{
    encoded_path, owner, page, page_size, path, positive_number, quoted_search_literal, repository,
    result_offset, search_term,
};

#[test]
fn validates_owner_repository_path_and_numeric_boundaries() {
    assert!(owner("octo-cat").is_ok());
    assert!(owner("-octo").is_err());
    assert!(owner(&"a".repeat(101)).is_err());
    assert!(repository("repo.name_v1").is_ok());
    assert!(repository("..").is_err());
    assert!(path("src/lib.rs").is_ok());
    assert!(path("src//lib.rs").is_err());
    assert!(path("../secret").is_err());
    assert_eq!(
        positive_number(2_147_483_647).expect("maximum should pass"),
        2_147_483_647
    );
    assert!(positive_number(0).is_err());
    assert!(page(Some(-1)).is_err());
    assert_eq!(page_size(None, 10, 20).expect("default should pass"), 10);
    assert!(page_size(Some(21), 10, 20).is_err());
}

#[test]
fn escapes_search_literals_and_encodes_path_segments() {
    assert_eq!(
        quoted_search_literal(r#"repo:foo \ "bar" *.rs"#),
        r#""repo:foo \\ \"bar\" *.rs""#
    );
    assert_eq!(encoded_path("folder/a b.rs"), "folder/a%20b%2Ers");
    assert_eq!(search_term("  rust  ").expect("query should pass"), "rust");
    assert!(search_term("\n").is_err());
    assert_eq!(result_offset(50, 20).expect("offset should fit"), 980);
}
