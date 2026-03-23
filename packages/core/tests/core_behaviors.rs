use submora_core::{
    is_strong_password, is_valid_password_length, is_valid_source_url, is_valid_username,
    normalize_links_preserve_order,
};

#[test]
fn username_validation_accepts_expected_values() {
    assert!(is_valid_username("alpha-feed"));
    assert!(is_valid_username("alpha_feed_01"));
    assert!(!is_valid_username(""));
    assert!(!is_valid_username("contains space"));
    assert!(!is_valid_username("slash/name"));
}

#[test]
fn password_rules_keep_length_and_strength_separate() {
    assert!(is_valid_password_length("admin"));
    assert!(is_strong_password("Admin123!"));
    assert!(!is_strong_password("admin1234"));
    assert!(!is_strong_password("AdminOnly!"));
    assert!(!is_strong_password("12345678!"));
}

#[test]
fn source_url_validation_requires_http_or_https_with_host() {
    assert!(is_valid_source_url("https://example.com/feed"));
    assert!(is_valid_source_url("http://example.com/feed"));
    assert!(!is_valid_source_url("ftp://example.com/feed"));
    assert!(!is_valid_source_url("https://"));
    assert!(!is_valid_source_url("not-a-url"));
}

#[test]
fn normalize_links_trims_dedupes_and_preserves_order() {
    let links = vec![
        " https://example.com/a ".to_string(),
        String::new(),
        "https://example.com/b".to_string(),
        "https://example.com/a".to_string(),
    ];

    let normalized = normalize_links_preserve_order(&links, 10).unwrap();

    assert_eq!(
        normalized,
        vec![
            "https://example.com/a".to_string(),
            "https://example.com/b".to_string()
        ]
    );
}
