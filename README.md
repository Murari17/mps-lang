#  Makes Python Slow (MPS)

**Makes Python Slow (MPS)** is a high-performance, zero-overhead, compiled programming language designed from scratch in Rust. It transpiles to compliant **ISO C11** to achieve near-C/C++ native speed while embedding the **CPython C FFI Runtime** to allow seamless, zero-overhead interoperability with any standard Python library (e.g. `numpy`, `pandas`, `math`, `sys`).

By utilizing dynamic, compile-time host Python discovery and combining it with a spanned recursive descent parser, custom scope-based reference count tracking, compile-time AST optimization passes, and native cache-friendly matrix libraries, MPS represents a premium compiled language environment.

---

##  Compiler Architecture

The MPS compiler is divided into modular, zero-dependency stages written in Rust:

```
┌──────────────────────────────────────────────────────────────────────┐
│ hello.mps (MPS Source Code)                                         │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Stateful Lexer (src/lexer.rs)                                       │
│ ├─ INDENT/DEDENT token emission via indentation stack               │
│ ├─ Spanned tokens with line/col tracking                            │
│ └─ Keywords: fn, class, trait, match, async, await, try, raise ...  │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Recursive Descent Parser (src/parser.rs)                            │
│ ├─ Functions, classes, traits, match/case                           │
│ ├─ Pipe operator |> with left-associative AST rewriting             │
│ ├─ Lambda expressions, list/dict/tuple literals                     │
│ └─ try/catch/finally, raise, elif chains                            │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Scoped Type Checker (src/typechecker.rs)                            │
│ ├─ Symbol table with nested scope layers                            │
│ ├─ Type inference, param count validation                           │
│ ├─ Trait implementation completeness checking                       │
│ └─ Colored diagnostic error output with line/column spans           │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ AST Optimizer (src/optimizer.rs)                                    │
│ ├─ Constant folding (int/float arithmetic + comparisons)            │
│ ├─ Dead code elimination (unreachable branches, unused vars)        │
│ └─ Loop removal (while false: ...)                                  │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ C11 Code Generator (src/codegen.rs)                                 │
│ ├─ Struct-based OOP with vtable method dispatch                     │
│ ├─ Operator overloading via method call rewriting                   │
│ ├─ Lambda → static C functions with closure capture                 │
│ ├─ try/catch → setjmp/longjmp boundaries                           │
│ ├─ async/await → native Windows Thread Pool tasks                   │
│ └─ Automatic PyObject* ref-counting + scope cleanup                 │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ runtime.h (C11 Polymorphic Runtime)                                 │
│ ├─ _Generic print/cast macros (int, float, string, bool, PyObject*) │
│ ├─ CPython FFI: py_call, to_py, list/tuple/dict constructors        │
│ ├─ Native MPSMatrix with cache-optimized transposed multiplication  │
│ ├─ setjmp/longjmp error handling (MPS_Error, mps_raise)             │
│ ├─ Thread pool (CRITICAL_SECTION + CONDITION_VARIABLE)              │
│ └─ stdlib: math, casting, file I/O, string operations               │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ CLI Driver & C Compiler Linker (src/main.rs)                        │
│ ├─ Auto-detects gcc, clang, or MSVC cl.exe                          │
│ ├─ Auto-discovers Python include/lib paths for FFI linking          │
│ └─ Cleans up temp .c / .obj / runtime.h after compilation           │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────┐
                    │ Native Executable │
                    │ (hello.exe)       │
                    └───────────────────┘
```

---

##  Language Features

### Core Language
| Feature | Syntax | Status |
|---------|--------|--------|
| Variables | `let x = 10` / `const PI = 3.14` | ✅ |
| Functions | `fn add(a: int, b: int) -> int:` | ✅ |
| Classes & OOP | `class Dog(Animal):` | ✅ |
| Inheritance | `class Child(Parent):` with `super.method()` | ✅ |
| Traits | `trait Drawable:` with compile-time checks | ✅ |
| Operator Overloading | `fn add(self, other: Vec2) -> Vec2:` | ✅ |
| If / Elif / Else | `if x: ... elif y: ... else:` | ✅ |
| While Loops | `while condition:` | ✅ |
| For Loops | `for i in range(0, n):` | ✅ |
| Match / Case | `match x:` with `case 1:` / `case _:` | ✅ |
| Try / Catch / Finally | `try: ... catch err: ... finally:` | ✅ |
| Raise | `raise "error message"` | ✅ |
| Lambdas | `fn(x: int) -> int: x * 2` | ✅ |
| Pipe Operator | `x \|> double \|> print` | ✅ |
| List Literals | `[1, 2, 3]` | ✅ |
| Dict Literals | `{"key": value}` | ✅ |
| Tuple Literals | `(1, 2, 3)` | ✅ |
| Async / Await | `async fn compute():` / `await task` | ✅ |
| Python FFI | `pyimport numpy as np` | ✅ |

### Optimizations
| Feature | Description | Status |
|---------|-------------|--------|
| Constant Folding | `5 * 2 + 10` → compiled as `20` | ✅ |
| Dead Code Elimination | Removes unreachable branches and unused vars | ✅ |
| Loop Removal | Eliminates `while false:` and similar | ✅ |
| Cache-Optimized Matrix | Transposed B multiplication for linear reads | ✅ |

### Standard Library (`runtime.h`)
| Category | Functions / Constants |
|----------|-----------------------|
| **I/O** | `print()`, `mps_input()` |
| **Casting** | `mps_to_int()`, `mps_to_float()`, `mps_to_string()`, `mps_to_bool()` |
| **Math Constants** | `MPS_PI` (~3.14159), `MPS_E` (~2.71828) |
| **Math Functions** | `mps_abs()`, `mps_sqrt()`, `mps_pow()`, `mps_floor()`, `mps_ceil()`, `mps_round()`, `mps_min()`, `mps_max()`, `mps_clamp()`, `mps_sin()`, `mps_cos()`, `mps_tan()` |
| **String Functions** | `mps_str_len(s)`, `mps_str_upper(s)`, `mps_str_lower(s)`, `mps_str_trim(s)`, `mps_str_contains(s, sub)`, `mps_str_starts_with(s, pre)`, `mps_str_ends_with(s, suf)`, `mps_str_replace(s, old, new)` |
| **Files** | `mps_file_read()`, `mps_file_write()`, `mps_file_append()`, `mps_file_exists()` |
| **System** | `mps_env(key)`, `mps_exit()`, `mps_sleep(ms)` |
| **Matrix** | `Matrix(rows, cols)`, `.get()`, `.set()`, `.mul()` |

---

## 📝 Syntax Specification (EBNF Grammar)

```ebnf
Program            ::= Statement*

Statement          ::= FunctionDecl | AsyncFunctionDecl | ClassDecl | TraitDecl
                     | PyImport | VariableDecl | AssignStmt
                     | IfStmt | WhileStmt | ForStmt
                     | TryCatchStmt | RaiseStmt | MatchStmt
                     | ExprStmt | ReturnStmt

FunctionDecl       ::= "fn" Identifier "(" ParamList? ")" ( "->" Type )? ":" Block
AsyncFunctionDecl  ::= "async" FunctionDecl
ClassDecl          ::= "class" Identifier ( "(" Identifier ")" )? ":" Newline Indent Statement* Dedent
TraitDecl          ::= "trait" Identifier ":" Newline Indent TraitMethodDecl* Dedent
TraitMethodDecl    ::= "fn" Identifier "(" ParamList? ")" ( "->" Type )? Newline
PyImport           ::= "pyimport" Identifier ( "as" Identifier )? Newline

VariableDecl       ::= ( "let" | "const" ) Identifier ( ":" Type )? ( "=" Expression )? Newline
AssignStmt         ::= LValue "=" Expression Newline
IfStmt             ::= "if" Expression ":" Block ( "elif" Expression ":" Block )* ( "else" ":" Block )?
WhileStmt          ::= "while" Expression ":" Block
ForStmt            ::= "for" Identifier "in" Expression ":" Block
ReturnStmt         ::= "return" Expression? Newline
ExprStmt           ::= Expression Newline

TryCatchStmt       ::= "try" ":" Block "catch" Identifier ":" Block ( "finally" ":" Block )?
RaiseStmt          ::= "raise" Expression Newline
MatchStmt          ::= "match" Expression ":" Newline Indent MatchCase+ Dedent
MatchCase          ::= "case" MatchPattern ":" Block
MatchPattern       ::= Literal | "_"

Block              ::= Newline Indent Statement* Dedent

LValue             ::= Identifier
                     | Primary "." Identifier
                     | Primary "[" Expression "]"

Expression         ::= PipeExpr
PipeExpr           ::= Comparison ( "|>" Comparison )*
Comparison         ::= Additive ( ( "==" | "!=" | "<" | "<=" | ">" | ">=" ) Additive )*
Additive           ::= Multiplicative ( ( "+" | "-" ) Multiplicative )*
Multiplicative     ::= Unary ( ( "*" | "/" | "%" ) Unary )*
Unary              ::= Primary

Primary            ::= Literal
                     | Identifier
                     | "await" Expression
                     | "super" ( "." Identifier "(" ArgList? ")" )?
                     | "fn" "(" ParamList? ")" ( "->" Type )? ":" Expression    (* Lambda *)
                     | "[" ArgList? "]"                                          (* List Literal *)
                     | "{" DictPairList? "}"                                     (* Dict Literal *)
                     | "(" ArgList? ")"                                          (* Tuple / Paren *)
                     | Primary "." Identifier ( "(" ArgList? ")" )?             (* Member Access/Call *)
                     | Primary "[" Expression "]"                                (* Subscript *)
                     | Identifier "(" ArgList? ")"                               (* Function Call *)

Type               ::= "int" | "float" | "string" | "bool" | "void" | Identifier
Literal            ::= IntLiteral | FloatLiteral | StringLiteral | "true" | "false"
```

---

##  Getting Started

###  Prerequisites
To build and run MPS, you will need:
1.  **Rust Toolchain** (Cargo & `rustc`): [Install Rust](https://www.rust-lang.org/tools/install)
2.  **C Compiler**: MSVC `cl.exe` (via Visual Studio Build Tools), GCC, or Clang.
3.  **Python 3.x** *(optional)*: Only required if your `.mps` file uses `pyimport`. Programs without Python FFI compile without any Python dependency.

###  Building the Compiler
Navigate to the directory and compile the compiler in release mode:
```powershell
cargo build --release
```
The executable will be generated at `./target/release/mps.exe` (or `./target/release/mps` on Linux/macOS).

###  Running Tests
To execute all 30+ comprehensive parser, lexer, typechecker, and optimizer unit tests:
```powershell
cargo test
```

---

##  Usage

### Running an MPS Script
Compile and run any `.mps` script immediately:
```powershell
./target/release/mps.exe hello.mps --run
```

### Inspecting Generated C Code
Emit the transpiled C11 output without invoking the C compiler:
```powershell
./target/release/mps.exe hello.mps --emit-c
```

### Debugging the Parser
Print the parsed AST to stdout:
```powershell
./target/release/mps.exe hello.mps --emit-ast
```

### CLI Reference
```
Makes Python Slow (MPS) Compiler v0.1.0
Usage:
  mps <source_file.mps> [options]

Options:
  --run                Compile and run the program immediately
  --emit-c             Output transpiled C code (no compilation)
  --emit-ast           Print parsed AST to stdout
  -o, --output <path>  Specify output binary path
  -h, --help           Show this help message
```

---

##  Example Programs

### 1. Hello World with Control Flow
```python
# hello.mps
fn fibonacci(n: int) -> int:
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

let result = fibonacci(30)
print(result)
```

### 2. OOP with Inheritance & Operator Overloading
```python
# oop_demo.mps
class Vec2:
    let x: float
    let y: float

    fn __init__(self, x: float, y: float):
        self.x = x
        self.y = y

    fn add(self, other: Vec2) -> Vec2:
        return Vec2(self.x + other.x, self.y + other.y)

let a = Vec2(1.0, 2.0)
let b = Vec2(3.0, 4.0)
let c = a + b    # Uses operator overloading → calls a.add(b)
print(c.x)       # 4.0
```

### 3. Pattern Matching & Error Handling
```python
# advanced.mps
fn classify(n: int) -> string:
    match n:
        case 0:
            return "zero"
        case 1:
            return "one"
        case _:
            return "many"

try:
    let x = classify(42)
    print(x)
catch err:
    print(err)
finally:
    print("done")
```

### 4. Pipe Operator & Lambdas
```python
# functional.mps
fn double(x: int) -> int:
    return x * 2

fn add_one(x: int) -> int:
    return x + 1

# Pipe chains: value flows left-to-right
let result = 5 |> double |> add_one
print(result)    # 11
```

### 5. Async / Await
```python
# async.mps
async fn compute(n: int) -> int:
    return n * n

let task = compute(42)
let result = await task
print(result)    # 1764
```

### 6. Python Interoperability
```python
# numpy_demo.mps
pyimport numpy as np

let arr = np.arange(0, 10, 1)
print(arr)

let mean = np.mean(arr)
print(mean)
```

### 7. Native Matrix Performance
```python
# matrix_perf.mps
let size = 200
let a = Matrix(size, size)
let b = Matrix(size, size)

for i in range(0, size):
    for j in range(0, size):
        a.set(i, j, 1.5)
        b.set(i, j, 2.0)

let c = a.mul(b)
print(c)
```

---

##  Package Manager (`mps_pkg`)

MPS includes a lightweight, built-in package manager for dependency resolution:

```powershell
# Initialize a new MPS project in the current directory (creates mps.toml)
mps_pkg init

# Add a dependency from a GitHub repository
mps_pkg add https://github.com/user/math_utils

# Install all dependencies specified in mps.toml
mps_pkg install
```

### Dependency Resolution
Running `mps_pkg install` will:
1. Parse `mps.toml` and read all registered Git repository URLs.
2. Clone each repository dynamically using `git clone --depth 1` into the global cache directory: `~/.mps/packages/`.
3. Create an `mps.lock` lockfile to lock and track installed dependency hashes.

---

##  Project Structure

```
new lang/
├── Cargo.toml              # Rust project manifest (mps + mps_pkg binaries)
├── Cargo.lock              # Dependency lock file
├── README.md               # This file
├── src/
│   ├── main.rs             # CLI driver, C compiler auto-detection, Python path discovery
│   ├── lexer.rs            # Stateful lexer with INDENT/DEDENT + span tracking
│   ├── parser.rs           # Recursive descent parser (1000+ lines)
│   ├── ast.rs              # AST node type definitions
│   ├── typechecker.rs      # Scoped type checker with colored diagnostics
│   ├── optimizer.rs        # Constant folding + dead code elimination
│   ├── codegen.rs          # C11 code generator (~2000 lines)
│   ├── runtime.h           # C11 polymorphic runtime (conditionally includes Python.h)
│   └── mps_pkg.rs          # Package manager binary
├── hello.mps               # Hello world demo
├── oop_demo.mps            # OOP demonstration
├── matrix_perf.mps         # Matrix benchmark
├── py_interop.mps          # Python interop demo
├── phase2_demo.mps         # Full Phase 2 comprehensive language core integration test
├── phase2_test1.mps        # Test suite 1: OOP + Traits + Super Delegation + Operator Overloading
├── phase2_test2.mps        # Test suite 2: Match/Case + Pipe + Lambdas
├── phase2_test3.mps        # Test suite 3: Try/Catch/Finally & Raise Exception Handling
├── phase2_test4.mps        # Test suite 4: Extended String & Math standard library
└── phase2_test5.mps        # Test suite 5: Async/Await + FFI Collections (CPython Interop)
```

---

##  Technical Details

### 1. Conditional Python Dependency
`runtime.h` conditionally includes `<Python.h>` only when `MPS_USE_PYTHON` is defined. The codegen automatically emits this define when the program contains `pyimport` statements. Programs without Python FFI compile without any Python installation or libraries on the system.

### 2. Memory Safety & Reference Counting
- **Scope-based cleanup**: The compiler tracks `PyObject*` and `MPSMatrix*` allocations per block scope and emits `Py_XDECREF()` / `matrix_free()` calls on scope exit.
- **Ownership transfer**: Temporary return values assigned to variables are moved (not copied), preventing use-after-free.
- **String pool**: File read results and string operations use a rotating thread-local string pool to avoid leaks while avoiding explicit allocation/deallocation in user code.

### 3. Type Coercing String Concatenation
The string concatenation operator `+` supports dynamic coercion of non-string operands. When transpiling an addition where at least one side is a string:
- Non-string operands are automatically wrapped in `mps_to_string()`.
- The codegen emits a call to `mps_str_concat()`.
- `mps_str_concat()` dynamically allocates the merged buffer, populates it, and stores the pointer in a rotating, thread-local string pool to prevent memory leaks.

### 4. Lambdas & Function Pointer Casts
Anonymous lambdas are transpiled into static global C functions with a unique counter-based name suffix (`_lambda_N`). Calls to variables holding lambda values (e.g. `let square = fn(y: int) -> int: y * y`) are transpiled into correctly cast standard C11 function pointers:
```c
((int (*)(int))square)(5)
```
This guarantees compile-time safety and absolute type compatibility in the transpiled C code.

### 5. Error Handling Implementation
`try`/`catch`/`finally` is implemented using C `setjmp`/`longjmp`:
- `try:` → `if (setjmp(mps_err_buf) == 0)`
- `raise "msg"` → `mps_raise("msg")` which calls `longjmp`
- Thread-local error context (`THREAD_LOCAL jmp_buf*`) ensures thread safety.
- `finally:` blocks are guaranteed to execute on normal exit or exception recovery.
- The special catch variable `err` is of type `Custom("Error")` (mapped to stack-allocated `MPS_Error` struct), allowing users to access `err.message` in C directly.

### 6. Async/Await Implementation
Native Windows thread pool using `CreateThread`, `CRITICAL_SECTION`, and `CONDITION_VARIABLE`:
- `async fn` → task struct + launcher function + thread pool submission
- `await` → blocks on condition variable until task completion
- Fixed-size pool of 4 worker threads

---

## ⚖️ License

MPS is open-source software licensed under the **MIT License**.
