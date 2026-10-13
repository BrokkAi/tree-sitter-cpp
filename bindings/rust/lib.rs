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

    fn find_declaration_containing<'tree>(
        node: tree_sitter::Node<'tree>,
        source: &str,
        text: &str,
    ) -> Option<tree_sitter::Node<'tree>> {
        let mut pending = vec![node];
        while let Some(current) = pending.pop() {
            if matches!(current.kind(), "declaration" | "function_definition")
                && current
                    .utf8_text(source.as_bytes())
                    .is_ok_and(|node_text| node_text.contains(text))
            {
                return Some(current);
            }
            let mut cursor = current.walk();
            pending.extend(current.named_children(&mut cursor));
        }
        None
    }

    fn find_preprocessor_node<'tree>(
        node: tree_sitter::Node<'tree>,
        source: &str,
        kind: &str,
        first_line_prefix: &str,
    ) -> Option<tree_sitter::Node<'tree>> {
        let mut pending = vec![node];
        while let Some(current) = pending.pop() {
            if current.kind() == kind
                && current
                    .utf8_text(source.as_bytes())
                    .is_ok_and(|text| text.starts_with(first_line_prefix))
            {
                return Some(current);
            }
            let mut cursor = current.walk();
            pending.extend(current.named_children(&mut cursor));
        }
        None
    }

    fn enclosing_preprocessor_directives(node: tree_sitter::Node<'_>, source: &str) -> Vec<String> {
        let mut directives = Vec::new();
        let mut ancestor = node.parent();
        while let Some(current) = ancestor {
            if matches!(current.kind(), "preproc_if" | "preproc_ifdef") {
                let first_line = current
                    .utf8_text(source.as_bytes())
                    .unwrap()
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim();
                directives.push(first_line.to_owned());
            }
            ancestor = current.parent();
        }
        directives
    }

    fn assert_guard_ancestry(
        node: tree_sitter::Node<'_>,
        source: &str,
        required: &[&str],
        forbidden: &[&str],
    ) -> Vec<String> {
        let directives = enclosing_preprocessor_directives(node, source);
        for guard in required {
            assert!(
                directives.iter().any(|directive| directive.contains(guard)),
                "expected guard {guard} in {directives:?}"
            );
        }
        for guard in forbidden {
            assert!(
                directives
                    .iter()
                    .all(|directive| !directive.contains(guard)),
                "did not expect guard {guard} in {directives:?}"
            );
        }
        directives
    }

    fn has_ancestor_kind(node: tree_sitter::Node<'_>, kind: &str) -> bool {
        let mut ancestor = node.parent();
        while let Some(current) = ancestor {
            if current.kind() == kind {
                return true;
            }
            ancestor = current.parent();
        }
        false
    }

    fn add_declaration_before_final_endif(source: &str) -> String {
        let header_endif = "#endif\n";
        let body = source
            .strip_suffix(header_endif)
            .expect("UV fixture should end with its header guard");
        format!("{body}int uv_after(void);\n{header_endif}")
    }

    fn assert_uv_header_linkage_scopes(source: &str, cpp_guard: &str) {
        let tree = parse_cpp(source);
        let root = tree.root_node();
        let c_declaration =
            find_declaration_containing(root, source, "int uv_tty_set_mode(void*, mode_t);")
                .expect("C API declaration should be retained");
        assert_guard_ancestry(c_declaration, source, &["UV_H"], &[cpp_guard]);

        let cpp_definition = find_declaration_containing(
            root,
            source,
            "inline int uv_tty_set_mode(void* handle, int mode)",
        )
        .expect("C++ overload should be retained");
        assert_guard_ancestry(cpp_definition, source, &["UV_H", cpp_guard], &[]);

        let following = find_declaration_containing(root, source, "int uv_after(void);")
            .expect("declaration following the C++ conditional should be retained");
        assert_guard_ancestry(following, source, &["UV_H"], &[cpp_guard]);

        let header_guard = find_preprocessor_node(root, source, "preproc_ifdef", "#ifndef UV_H")
            .expect("UV_H header guard should remain a conditional node");
        assert!(header_guard
            .utf8_text(source.as_bytes())
            .unwrap()
            .trim_end()
            .ends_with("#endif"));
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
    fn test_uv_conditional_cpp_overload_stays_inside_its_guard() {
        const UV_HEADER: &str = r#"
#ifndef UV_H
#define UV_H
#ifdef __cplusplus
extern "C" {
#endif
typedef enum { MODE_NORMAL } mode_t;
int uv_tty_set_mode(void*, mode_t);
#ifdef __cplusplus
}
extern "C++" {
inline int uv_tty_set_mode(void* handle, int mode) {
  return uv_tty_set_mode(handle, static_cast<mode_t>(mode));
}
}
#endif
#endif
"#;

        let ifdef = add_declaration_before_final_endif(UV_HEADER);
        assert_uv_header_linkage_scopes(&ifdef, "__cplusplus");

        let if_defined = ifdef.replace("#ifdef __cplusplus", "#if defined (__cplusplus)");
        assert_uv_header_linkage_scopes(&if_defined, "__cplusplus");

        let arbitrary_guard = ifdef.replace("__cplusplus", "UV_CPP_ABI");
        assert_uv_header_linkage_scopes(&arbitrary_guard, "UV_CPP_ABI");

        let nested_guard = ifdef
            .replacen(
                "#ifdef __cplusplus\n}\nextern \"C++\" {",
                "#ifdef __cplusplus\n}\n#if defined(UV_CPP_ABI)\nextern \"C++\" {",
                1,
            )
            .replacen(
                "}\n#endif\nint uv_after(void);",
                "}\n#endif\n#endif\nint uv_after(void);",
                1,
            );
        assert_ne!(
            nested_guard, ifdef,
            "nested guard fixture should be changed"
        );
        assert_uv_header_linkage_scopes(&nested_guard, "__cplusplus");
        let nested_tree = parse_cpp(&nested_guard);
        let cpp_definition = find_declaration_containing(
            nested_tree.root_node(),
            &nested_guard,
            "inline int uv_tty_set_mode(void* handle, int mode)",
        )
        .unwrap();
        assert!(
            enclosing_preprocessor_directives(cpp_definition, &nested_guard)
                .iter()
                .any(|directive| directive.contains("UV_CPP_ABI")),
            "nested ABI guard should remain an ancestor of the C++ overload: {}",
            nested_tree.root_node().to_sexp()
        );
        let following = find_declaration_containing(
            nested_tree.root_node(),
            &nested_guard,
            "int uv_after(void);",
        )
        .unwrap();
        assert_guard_ancestry(
            following,
            &nested_guard,
            &["UV_H"],
            &["__cplusplus", "UV_CPP_ABI"],
        );

        for (branch_directive, expected_kind) in [
            ("#else", "preproc_else"),
            ("#elif defined(UV_CPP_ABI)", "preproc_elif"),
        ] {
            let branch_source = ifdef.replace(
                "}\n#endif\nint uv_after(void);",
                &format!(
                    "}}\n{branch_directive}\nint uv_alternate(void);\n#endif\nint uv_after(void);"
                ),
            );
            assert_uv_header_linkage_scopes(&branch_source, "__cplusplus");
            let branch_tree = parse_cpp(&branch_source);
            let alternate = find_declaration_containing(
                branch_tree.root_node(),
                &branch_source,
                "int uv_alternate(void);",
            )
            .expect("alternate declaration should be retained");
            assert!(
                has_ancestor_kind(alternate, expected_kind),
                "alternate declaration should remain inside {expected_kind}: {}",
                branch_tree.root_node().to_sexp()
            );
        }
    }

    #[test]
    fn test_conditional_linkage_empty_true_branch_preserves_else() {
        let source = "#ifdef __cplusplus\nextern \"C\" {\n#endif\nvoid c_api(void);\n#ifdef __cplusplus\n}\n#else\nint alternate(void);\n#endif\nint after(void);\n";
        let tree = parse_cpp(source);
        let root = tree.root_node();
        let c_api = find_declaration_containing(root, source, "void c_api(void);").unwrap();
        assert_guard_ancestry(c_api, source, &[], &["__cplusplus"]);

        let alternate = find_declaration_containing(root, source, "int alternate(void);").unwrap();
        assert!(has_ancestor_kind(alternate, "preproc_else"));

        let after = find_declaration_containing(root, source, "int after(void);").unwrap();
        assert_guard_ancestry(after, source, &[], &["__cplusplus"]);
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
