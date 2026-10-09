package tree_sitter_cpp_test

import (
	"testing"

	tree_sitter_cpp "github.com/BrokkAi/tree-sitter-cpp/bindings/go"
	tree_sitter "github.com/tree-sitter/go-tree-sitter"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_cpp.Language())
	parser := tree_sitter.NewParser()
	defer parser.Close()
	if err := parser.SetLanguage(language); err != nil {
		t.Fatalf("Error loading C++ grammar: %v", err)
	}
	tree := parser.Parse([]byte("int answer() { return 42; }"), nil)
	if tree == nil {
		t.Fatal("C++ parser did not return a tree")
	}
	defer tree.Close()
	if tree.RootNode().HasError() {
		t.Fatal("C++ parser returned an error node")
	}
}
