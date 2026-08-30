use mdbook_structured_core::{Limits, SourceLocation, StructuredPath};

#[test]
fn limits_require_positive_values_and_keep_exact_ceilings() {
    assert!(Limits::try_new(0, 10, 10).is_err());
    let limits = Limits::try_new(1_048_576, 10_000, 64).unwrap();
    assert_eq!(limits.max_input_bytes().get(), 1_048_576);
    assert_eq!(limits.max_nodes().get(), 10_000);
    assert_eq!(limits.max_depth().get(), 64);
}

#[test]
fn locations_are_one_based_and_paths_are_typed() {
    let location = SourceLocation::try_new(3, 5).unwrap();
    assert_eq!((location.line().get(), location.column().get()), (3, 5));
    let path = StructuredPath::root()
        .with_key("services")
        .with_index(2)
        .with_key("port");
    assert_eq!(path.segments().len(), 3);
}
