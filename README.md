
# Rust JSONPath Plus

[![crates.io](https://img.shields.io/crates/v/jsonpath-plus.svg)](https://crates.io/crates/jsonpath-plus)
[![Documentation](https://docs.rs/rebound/badge.svg)](https://docs.rs/jsonpath-plus)
[![MIT/Apache-2 licensed](https://img.shields.io/crates/l/rebound.svg)](./LICENSE-APACHE)

An implementation of the JSONPath A spec in Rust, with several extensions added on.

This library also supports retrieving AST analysis of compiled paths, so users
may implement syntax highlighting or nice error reporting on top of it.

## Extensions

- Parent selector `^`, used as `$.a.b.^` or `$['a']['b'][^]`. Matches the parent of
  the currently selected object.
- Subpath selectors, used as `$['a'][$.b.id]` or `$['a'][@.sum.id]`. Evaluates the
  subpath, then selects items with keys same as the result of the subpath.
- ID selector `~`, used at the end of the path as `@.a.b~` or `$['a']['b']~`. Can be
  used in filters to compare against the ID of a matched item. Doesn't yet work at
  the top level.
