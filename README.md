# Brokk's C++ Grammar for Tree-sitter

[![CI][ci]](https://github.com/BrokkAi/tree-sitter-cpp/actions/workflows/ci.yml)
[![crates][crates]](https://crates.io/crates/brokk-tree-sitter-cpp)
[![docs.rs][docs]](https://docs.rs/brokk-tree-sitter-cpp)

This is the **Brokk-owned and independently maintained fork** of
[`tree-sitter/tree-sitter-cpp`](https://github.com/tree-sitter/tree-sitter-cpp),
a C++ grammar for [Tree-sitter](https://tree-sitter.github.io/tree-sitter/).
Brokk maintains this fork for its code-intelligence tooling and publishes the
Rust package as
[`brokk-tree-sitter-cpp`](https://crates.io/crates/brokk-tree-sitter-cpp).
The grammar and bindings remain available under the upstream MIT license; see
[`LICENSE`](LICENSE) for the retained copyright and license notice.

Try the grammar in [Brokk's web playground](https://brokkai.github.io/tree-sitter-cpp/).

The npm and Python bindings retain their upstream-compatible package names but
are not published by this fork. The Go, Swift, and C bindings are available
from source in this repository.

The 0.23.x line is intentionally based on upstream commit
`f41e1a044c8a84ea9fa8577fdd2eab92ec96de02` (upstream 0.23.4). It preserves
parser ABI 14, the upstream C 0.23.1 dependency, and Tree-sitter CLI 0.24.3,
while carrying Brokk's fix for quoted Windows include paths. Later upstream
grammar changes use a newer ABI and are not silently mixed into this line.

## Installation

Add the Brokk-maintained Rust crate to your project:

```sh
cargo add brokk-tree-sitter-cpp@=0.23.5
```

Or add it directly to `Cargo.toml`:

```toml
[dependencies]
tree-sitter-cpp = { package = "brokk-tree-sitter-cpp", version = "=0.23.5" }
```

The crate prefixes its native parser symbols so it can coexist with the
upstream `tree-sitter-cpp` crate in one executable.

To regenerate the parser, install the locked JavaScript dependencies (`npm
ci`) and run `npx tree-sitter generate --abi 14`. Run `npx tree-sitter test` to
check the full corpus and highlighting suite.

## References

- [Hyperlinked C++ BNF Grammar](http://www.nongnu.org/hcb/)
- [EBNF Syntax: C++](http://www.externsoft.ch/download/cpp-iso.html)

[ci]: https://img.shields.io/github/actions/workflow/status/BrokkAi/tree-sitter-cpp/ci.yml?logo=github&label=CI
[crates]: https://img.shields.io/crates/v/brokk-tree-sitter-cpp?logo=rust
[docs]: https://img.shields.io/docsrs/brokk-tree-sitter-cpp?logo=docs.rs
