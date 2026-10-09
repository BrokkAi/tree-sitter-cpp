from unittest import TestCase

import tree_sitter, tree_sitter_cpp


class TestLanguage(TestCase):
    def test_can_load_grammar(self):
        language = tree_sitter.Language(tree_sitter_cpp.language())
        parser = tree_sitter.Parser(language)
        tree = parser.parse(b"int answer() { return 42; }")
        self.assertFalse(tree.root_node.has_error)
