//! This crate provides C++ language support for the [tree-sitter][] parsing library.
//!
//! Typically, you will use the [LANGUAGE][] constant to add this language to a
//! tree-sitter [Parser][], and then use the parser to parse some code:
//!
//! ```
//! use tree_sitter::Parser;
//!
//! let code = r#"
//! int double(int x) {
//!     return x * 2;
//! }
//! "#;
//! let mut parser = tree_sitter::Parser::new();
//! let language = brokk_tree_sitter_cpp::LANGUAGE;
//! parser
//!     .set_language(&language.into())
//!     .expect("Error loading C++ parser");
//! let tree = parser.parse(code, None).unwrap();
//! assert!(!tree.root_node().has_error());
//! ```
//!
//! [Parser]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter_language::LanguageFn;

extern "C" {
    fn brokk_tree_sitter_cpp() -> *const ();
}

/// The tree-sitter [`LanguageFn`][LanguageFn] for this grammar.
///
/// [LanguageFn]: https://docs.rs/tree-sitter-language/*/tree_sitter_language/struct.LanguageFn.html
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(brokk_tree_sitter_cpp) };

/// The content of the [`node-types.json`][] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers#static-node-types
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

/// The syntax highlighting query for this language.
pub const HIGHLIGHT_QUERY: &str = include_str!("../../queries/highlights.scm");

/// The symbol tagging query for this language.
pub const TAGS_QUERY: &str = include_str!("../../queries/tags.scm");

#[cfg(test)]
mod tests {
    macro_rules! runtime_regressions {
        ($test:ident, $runtime:ident) => {
            #[test]
            fn $test() {
                let mut parser = $runtime::Parser::new();
                parser.set_language(&super::LANGUAGE.into()).unwrap();
                for directive in ["ifdef", "ifndef"] {
                    let source = format!(
                        "extern char buffer[\n#{directive} FEATURE\n16 +\n#endif\n1];\nvoid target(void);\n"
                    );
                    let tree = parser.parse(&source, None).unwrap();
                    let root = tree.root_node();
                    assert!(!root.has_error(), "{}", root.to_sexp());
                    assert_eq!(root.named_child_count(), 2);
                    let array = root.named_child(0).unwrap().child_by_field_name("declarator").unwrap();
                    let size = array.child_by_field_name("size").unwrap();
                    assert_eq!(size.kind(), "preproc_array_size");
                    assert_eq!(
                        size.utf8_text(source.as_bytes()).unwrap(),
                        format!("#{directive} FEATURE\n16 +\n#endif\n1")
                    );
                    let target = root.named_child(1).unwrap();
                    assert_eq!(target.kind(), "declaration");
                    assert_eq!(
                        target.child_by_field_name("declarator").unwrap()
                            .child_by_field_name("declarator").unwrap()
                            .utf8_text(source.as_bytes()).unwrap(),
                        "target"
                    );
                }
                let source = "namespace views { struct Widget { Widget() noexcept; }; }\nviews::Widget::Widget() noexcept = default;\n";
                let tree = parser.parse(source, None).unwrap();
                let root = tree.root_node();
                assert!(!root.has_error(), "{}", root.to_sexp());
                let constructor = root.named_child(1).unwrap();
                assert_eq!(constructor.kind(), "function_definition");
                assert_eq!(
                    constructor.child_by_field_name("declarator").unwrap()
                        .child_by_field_name("declarator").unwrap().kind(),
                    "qualified_identifier"
                );
                for source in [
                    "namespace demo { struct Widget { Widget(); }; Widget::Widget() = default; }",
                    "namespace outer { namespace demo { struct Widget { Widget() noexcept; }; Widget::Widget() noexcept = default; } }",
                ] {
                    let tree = parser.parse(source, None).unwrap();
                    let root = tree.root_node();
                    assert!(!root.has_error(), "{}", root.to_sexp());
                    let mut pending = vec![root];
                    let mut constructors = 0;
                    while let Some(node) = pending.pop() {
                        if node.kind() == "function_definition"
                            && node.child_by_field_name("declarator").and_then(|declarator| declarator.child_by_field_name("declarator")).is_some_and(|name| name.kind() == "qualified_identifier")
                        {
                            constructors += 1;
                        }
                        pending.extend(node.named_children(&mut node.walk()));
                    }
                    assert_eq!(constructors, 1, "{}", root.to_sexp());
                }
                for source in ["ordinary() = default;", "namespace demo { ordinary() = default; }"] {
                    let tree = parser.parse(source, None).unwrap();
                    let root = tree.root_node();
                    let mut pending = vec![root];
                    let mut expressions = 0;
                    while let Some(node) = pending.pop() {
                        assert_ne!(node.kind(), "function_definition", "{}", root.to_sexp());
                        expressions += usize::from(node.kind() == "expression_statement");
                        pending.extend(node.named_children(&mut node.walk()));
                    }
                    assert_eq!(expressions, 1);
                }
                let tree = parser.parse("ordinary() = default;", None).unwrap();
                assert_eq!(tree.root_node().named_child(0).unwrap().kind(), "expression_statement");
                for source in [
                    "#include \"C:\\include\\helper.h\"\n",
                    "#ifdef __cplusplus\nextern \"C\" {\n#endif\nvoid f(void);\n#ifdef __cplusplus\n}\n#endif\nint after(void);\n",
                ] {
                    let tree = parser.parse(source, None).unwrap();
                    assert!(!tree.root_node().has_error(), "{}", tree.root_node().to_sexp());
                }
            }
        };
    }

    runtime_regressions!(regressions_on_selected_runtime, tree_sitter);

    fn parse_cpp(source: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading C++ parser");
        let tree = parser.parse(source, None).expect("C++ parser timed out");
        let root = tree.root_node();
        assert!(!root.has_error(), "{source}\n{}", root.to_sexp());
        assert_no_error_nodes(root, source);
        tree
    }

    fn assert_no_error_nodes(node: tree_sitter::Node<'_>, source: &str) {
        assert!(
            !node.is_error() && !node.is_missing(),
            "unexpected error node in {source}: {}",
            node.to_sexp()
        );
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            assert_no_error_nodes(child, source);
        }
    }

    fn find_named_node<'tree>(
        node: tree_sitter::Node<'tree>,
        kind: &str,
    ) -> Option<tree_sitter::Node<'tree>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .find_map(|child| find_named_node(child, kind));
        found
    }

    fn count_named_nodes(node: tree_sitter::Node<'_>, kind: &str) -> usize {
        let mut cursor = node.walk();
        usize::from(node.kind() == kind)
            + node
                .named_children(&mut cursor)
                .map(|child| count_named_nodes(child, kind))
                .sum::<usize>()
    }

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading C++ parser");
    }

    #[test]
    fn test_windows_quoted_include_path_with_backslashes() {
        let source = r#"#include "C:\Users\ADMINI~1\include\helper.h"
"#;
        let tree = parse_cpp(source);
        let include = find_named_node(tree.root_node(), "preproc_include")
            .expect("quoted include should be a preproc_include node");
        let path = include
            .child_by_field_name("path")
            .expect("include path should be a named field");
        assert_eq!(path.kind(), "string_literal");
        assert_eq!(
            path.utf8_text(source.as_bytes()).unwrap(),
            r#""C:\Users\ADMINI~1\include\helper.h""#
        );
    }

    #[test]
    fn test_split_conditional_extern_c_guards_preserve_declarations() {
        let source = r#"#ifdef __cplusplus
#include <cstddef>
#endif
#ifdef __cplusplus
extern "C" {
#endif
void f(void);
#ifdef __cplusplus
}
#endif
int after(void);
"#;
        let tree = parse_cpp(source);
        let root = tree.root_node();

        let ordinary_guard = find_named_node(root, "preproc_ifdef")
            .expect("ordinary include guard should remain a preproc_ifdef");
        assert_eq!(
            ordinary_guard
                .child_by_field_name("name")
                .expect("ordinary guard should have a name")
                .utf8_text(source.as_bytes())
                .unwrap(),
            "__cplusplus"
        );
        let include = find_named_node(ordinary_guard, "preproc_include")
            .expect("ordinary include should remain inside its guard");
        assert_eq!(
            include
                .child_by_field_name("path")
                .expect("ordinary include should have a path")
                .kind(),
            "system_lib_string"
        );

        let open = find_named_node(root, "preproc_linkage_open")
            .expect("split extern C opener should be recognized");
        assert_eq!(
            open.child_by_field_name("value")
                .expect("linkage opener should have a value")
                .utf8_text(source.as_bytes())
                .unwrap(),
            "\"C\""
        );
        find_named_node(root, "preproc_linkage_close")
            .expect("split extern C closer should be recognized");

        let declaration = find_named_node(root, "declaration")
            .expect("declaration inside the linkage guard should be preserved");
        assert_eq!(
            declaration.utf8_text(source.as_bytes()).unwrap(),
            "void f(void);"
        );
        assert_eq!(count_named_nodes(root, "declaration"), 2);
        assert_eq!(count_named_nodes(root, "function_declarator"), 2);
    }

    #[test]
    fn test_modules_and_latest_cpp_syntax() {
        let source = r#"export module sample.core;
import std.core;

export struct Value {
    int number;
    int get(this Value &self) { return self.number; }
    auto operator<=>(const Value &) const = default;
};

auto increment = [](int input) { return input + 1; };
"#;
        let tree = parse_cpp(source);
        let root = tree.root_node();

        let module = find_named_node(root, "module_declaration")
            .expect("module declaration should be preserved");
        assert_eq!(
            module
                .child_by_field_name("name")
                .expect("module declaration should have a name")
                .utf8_text(source.as_bytes())
                .unwrap(),
            "sample.core"
        );
        find_named_node(root, "import_declaration").expect("module import should be preserved");
        find_named_node(root, "explicit_object_parameter_declaration")
            .expect("explicit object parameter should be recognized");
        let operator = find_named_node(root, "operator_name")
            .expect("spaceship operator should be recognized");
        assert_eq!(
            operator.utf8_text(source.as_bytes()).unwrap(),
            "operator<=>"
        );
        find_named_node(root, "lambda_expression").expect("lambda expression should be recognized");
    }
}
