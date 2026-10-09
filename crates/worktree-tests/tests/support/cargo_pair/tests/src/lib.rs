#[test]
fn consumes_real_producer() {
    assert_eq!(bounded_cache_producer::answer(), 42);
}
