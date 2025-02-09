# EmJay

EmJay is a very simple JIT interpreter that can execute a _very_ simple programming language. It is a toy project, created only for learning purposes and has absolutely zero practical applications.

The implemented language has no control flow statements (if, loop, similar), so it is not Turing-complete. It also has just one type: `i64`. It allows for variable declarations, basic math, and function calls. This is an example of some valid syntax:

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

The JIT supports multiple backends, but the only one fully implemented is aarch64 on macOS (i.e. Apple Silicon). There is a half-baked implementation of x64 on Linux, which I abandoned and does not implement all the functionalities.

I've relied on the [`pest`](https://pest.rs/) to implement the grammar and parser, but all the IR, optimizations, and code generations are done manually. This was very interesting to do, but I ended up spending a lot more time than I cared for on correctly encoding aarch64 instructions. I am considering, for my next project, to uses something like [`cranelift`](https://cranelift.dev/), [`llvm`](https://llvm.org/), [`mir`](https://github.com/vnmakarov/mir), or to output [`webassembly`](https://webassembly.org/), because I would like to focus more on the stuff that happens in the frontend of the compiler.

For more details, check out my blog post on https://andreabergia.com (TODO: replace with blog post link once written)
