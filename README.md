# Goscript
Go specs implemented as a scripting language in Rust.

### The Goal
+ To be full compatible with Go, i.e. to be able to run any valid Go code.(But only a subset of features will be supported in version 0.1)

### Use Cases
+ As an embedded language like Lua.
+ As a glue language like Python.

### Rationale
+ A scripting language that is Rust friendly is needed.
+ Go is popular and easy(even as a scripting language).
+ If Go were an embedded language it would be way better than Lua.
+ If Go were a glue language it would be way better than Python, in terms of project maintainability.

### Implementation
+ The parser is a port of the official Go implementation.
+ The VM is based on that of Lua/Wren.

### Progress
+ The parser is basically finished, we still need to port the Type Checker.
+ You can take a look at [here](https://github.com/oxfeeefeee/goscript/tree/master/backend/tests/data) to see what it can run for now.
+ It should be able to run leetcode answers sooooon, with 4 major tasks left to do: TypeChecker, GC, Library, API. If anyone is interested in becoming a co-author, pick one(or more) of those above :)

### Join us
+ If you like the idea, and would like to help, please contact oxfeeefeee at gmail.