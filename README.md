# EmJay

EmJay is a very simple JIT interpreter that can execute a _very_ simple programming language. It is a **toy project**, created only for learning purposes and has absolutely zero practical applications.

The language has the following limitations and features:

- it only has one data type - `i64`;
- it supports only the basic algebraic operations;
- it has no control flow statements (i.e. no `if` or loops);
- it allows variable declaration, nested scopes, and function calls;
- it supports aarch64 as a backend (i.e. Apple silicon). There's also an x64_linux backend, but it's not complete and basically not maintained.

It's a glorified calculator, basically. ☺️ But it does it in a pretty complicated way:

- it parses the input string and generates an [AST](https://en.wikipedia.org/wiki/Abstract_syntax_tree);
- it processes the AST and generates and [IR](https://en.wikipedia.org/wiki/Intermediate_representation) in [SSA](https://en.wikipedia.org/wiki/Static_single-assignment_form);
- it the performs some basic optimization on the IR;
- it then generates machine code from the IR;
- and finally it executes it and performs the computation.

It uses the [`pest`](https://pest.rs/) parser library for the parsing, but the rest was all implemented by hand - I did not rely on things like [`llvm`](https://llvm.org/) or [`cranelift`](https://cranelift.dev/). As a result, it generates pretty mediocre machine code, but it was a very useful learning exercise.

This is an example of some valid syntax:

```
fn main() {
    let v = 1000;
    return v + f(3, 2, 1);
}

fn f(x, y, z) {
    return x * 100 + y * 10 + (g(z) + z) * 2;
}

fn g(z) {
    return z + 1;
}
```

For more details, check out my blog post on https://andreabergia.com/blog/2025/02/emjay-a-simple-jit-that-does-math/
