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

The 0.24.x line tracks upstream commit
`c009222808634c1014f82438d4883753516a2c24` and uses parser ABI 15. Loading
this grammar requires Tree-sitter 0.25 or newer. It adds upstream's module,
reflection, lambda, explicit object parameter, and operator-call changes
while retaining Brokk's quoted Windows include-path and split conditional
`extern "C"` guard fixes.

Version 0.24.2 retains declarations following array bounds built from an
`#ifdef` or `#ifndef` prefix and an unconditional tail expression. The
`preproc_array_size` node owns that complete bound. General `#if`, `#else`,
and nested directives inside array bounds remain unsupported. This version
also recognizes qualified defaulted constructors, including namespace-qualified
owners and exception specifications, both at file scope and inside namespaces.

Generation uses the pinned C grammar 0.24.1 and Tree-sitter CLI 0.26.3.
The 0.23.x line remains based on upstream 0.23.4 with parser ABI 14.

## Installation

Add the Brokk-maintained Rust crate to your project:

```sh
cargo add brokk-tree-sitter-cpp@=0.24.2
```

Or add it directly to `Cargo.toml`:

```toml
[dependencies]
tree-sitter-cpp = { package = "brokk-tree-sitter-cpp", version = "=0.24.2" }
```

The crate prefixes its native parser symbols so it can coexist with the
upstream `tree-sitter-cpp` crate in one executable.

To regenerate the parser, install the locked JavaScript dependencies (`npm
ci`) and run `npx tree-sitter generate --abi 15`. Run `npx tree-sitter test` to
check the full corpus and highlighting suite.

## References

- [Hyperlinked C++ BNF Grammar](http://www.nongnu.org/hcb/)
- [EBNF Syntax: C++](http://www.externsoft.ch/download/cpp-iso.html)

[ci]: https://img.shields.io/github/actions/workflow/status/BrokkAi/tree-sitter-cpp/ci.yml?logo=github&label=CI
[crates]: https://img.shields.io/crates/v/brokk-tree-sitter-cpp?logo=rust
[docs]: https://img.shields.io/docsrs/brokk-tree-sitter-cpp?logo=docs.rs
