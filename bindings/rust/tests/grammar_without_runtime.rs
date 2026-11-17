#![cfg(not(feature = "reuse-allocator"))]

// This integration executable deliberately never links the Rust Tree-sitter
// runtime. Loading the grammar must keep working with default features.
#[test]
fn grammar_loads_without_rust_runtime() {
    let language = std::hint::black_box(brokk_tree_sitter_cpp::LANGUAGE).into_raw();
    // The grammar crate supplies this generated C language entry point.
    let grammar = unsafe { language() };
    assert!(!grammar.is_null());
}
