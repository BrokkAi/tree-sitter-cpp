# tree-sitter-cpp

[![CI][ci]](https://github.com/tree-sitter/tree-sitter-cpp/actions/workflows/ci.yml)
[![discord][discord]](https://discord.gg/w7nTvsVJhm)
[![matrix][matrix]](https://matrix.to/#/#tree-sitter-chat:matrix.org)
[![crates][crates]](https://crates.io/crates/tree-sitter-cpp)
[![npm][npm]](https://www.npmjs.com/package/tree-sitter-cpp)
[![pypi][pypi]](https://pypi.org/project/tree-sitter-cpp)

C++ grammar for [tree-sitter](https://github.com/tree-sitter/tree-sitter).

This repository carries Brokk's Rust crate fork from upstream `tree-sitter/tree-sitter-cpp` at commit `f41e1a044c8a84ea9fa8577fdd2eab92ec96de02` (0.23.4). Its Rust package is `brokk-tree-sitter-cpp` 0.23.5; Rust applications may depend on it under the familiar `tree-sitter-cpp` key with a Cargo package alias.

To regenerate the ABI 14 parser, install the locked JavaScript dependencies (`npm ci`) and run `npx tree-sitter generate --abi 14`. The grammar extends the upstream C 0.23.1 grammar and uses Tree-sitter CLI 0.24.3. Run `npx tree-sitter test` to check the full corpus and highlighting suite.

## References

- [Hyperlinked C++ BNF Grammar](http://www.nongnu.org/hcb/)
- [EBNF Syntax: C++](http://www.externsoft.ch/download/cpp-iso.html)

[ci]: https://img.shields.io/github/actions/workflow/status/tree-sitter/tree-sitter-cpp/ci.yml?logo=github&label=CI
[discord]: https://img.shields.io/discord/1063097320771698699?logo=discord&label=discord
[matrix]: https://img.shields.io/matrix/tree-sitter-chat%3Amatrix.org?logo=matrix&label=matrix
[npm]: https://img.shields.io/npm/v/tree-sitter-cpp?logo=npm
[crates]: https://img.shields.io/crates/v/tree-sitter-cpp?logo=rust
[pypi]: https://img.shields.io/pypi/v/tree-sitter-cpp?logo=pypi&logoColor=ffd242
