# RNF Programming Language

<div align="center">

```
  ██████╗ ███╗   ██╗███████╗
  ██╔══██╗████╗  ██║██╔════╝
  ██████╔╝██╔██╗ ██║█████╗  
  ██╔══██╗██║╚██╗██║██╔══╝  
  ██║  ██║██║ ╚████║██║     
  ╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝     
```

**High Performance · Systems · Automation**
<!--
[![CI](https://github.com/risqinf/rnf/actions/workflows/ci.yml/badge.svg)](https://github.com/risqinf/rnf/actions)
-->
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
![Version](https://img.shields.io/badge/version-0.1.0-blue)

</div>

---

RNF is a statically-compiled, high-performance systems programming language designed for backend services, custom tooling, and system automation. It combines the best ideas from multiple languages into a single, focused tool.

| Feature | Inspiration |
|---------|------------|
| Variable syntax | Shell (no `let` required, just `name = value`) |
| Concurrency (goroutines, channels) | Go |
| Hardware/memory access, pointers | C |
| Function syntax, structs, impl | Rust |
| System command execution | Bash |
| Compiler backend | LLVM |

## Installation

### One-line install
```sh
curl -sSL https://raw.githubusercontent.com/risqinf/rnf/main/install.sh | bash
```

### Build from source
```sh
git clone https://github.com/risqinf/rnf
cd rnf
cargo build --release
sudo cp target/release/rnf /usr/local/bin/
```

### Build static musl binary (recommended for servers)
```sh
rustup target add x86_64-unknown-linux-musl
RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target x86_64-unknown-linux-musl
```

## Usage

```sh
rnf --run main.rnf                          # Interpret and run directly
rnf --release main.rnf                      # Build static binary (release/binary/main)
rnf --release --path /custom/path main.rnf  # Build to custom path
rnf check main.rnf                          # Syntax check only
rnf ir main.rnf                             # Emit LLVM IR
rnf tokens main.rnf                         # Debug: show token stream
rnf ast main.rnf                            # Debug: show AST
rnf init myproject                          # Create new project
```

### Build output structure

When `--path` is **not** specified:
```
release/
└── binary/
    └── myprogram       ← static, stripped binary
```

When `--path /custom/path` is specified:
```
/custom/path/
└── myprogram
```

## Language Reference

### Variables (Shell-style)

No type declaration required — just assign:

```rnf
name    = "Alice"
count   = 42
pi      = 3.14159
active  = true

// Typed (optional)
x: int   = 100
msg: str = "hello"
```

### Functions (Rust-style syntax)

```rnf
fn greet(name: str) -> str {
    return "Hello, " + name + "!"
}

fn add(a: int, b: int) -> int {
    return a + b
}

// No return value
fn log(msg: str) {
    print("[LOG] " + msg)
}
```

### Control Flow

```rnf
// if / else
if count > 0 {
    print("positive")
} else if count == 0 {
    print("zero")
} else {
    print("negative")
}

// while loop
while active {
    count = count - 1
    if count == 0 { active = false }
}

// loop … in range  (like Python for)
loop i in 0..10 {
    print(i)
}

// loop over array
fruits = ["apple", "banana", "mango"]
loop f in fruits {
    print(f)
}
```

### System Execution (Bash-style)

```rnf
// Execute a system command (fire and forget)
exec "ls -la"
exec "mkdir -p /tmp/mydir"

// Capture output
output = exec("uname -a")
print(output)

// Compose commands
dir = "/tmp/data"
exec("mkdir -p " + dir)
exec("echo hello > " + dir + "/file.txt")
```

### Go-style Concurrency

```rnf
fn worker(id: int) {
    print("Worker " + str(id) + " running")
    sleep_ms(100)
    print("Worker " + str(id) + " done")
}

fn main() -> int {
    // Goroutines — spawns threads
    go worker(1)
    go worker(2)
    go worker(3)

    // Channel
    ch = make_chan<int>()
    send(ch, 42)
    val = recv(ch)
    print("Got: " + str(val))

    sleep_ms(200)  // wait for goroutines
    return 0
}
```

### Structs & Impl (Rust-style)

```rnf
struct Point {
    x: int
    y: int
}

impl Point {
    fn new(x: int, y: int) -> Point {
        return Point { x: x, y: y }
    }

    fn distance(p: Point) -> float {
        return p.x * p.x + p.y * p.y
    }
}

fn main() -> int {
    p = Point { x: 3, y: 4 }
    print("x=" + str(p.x) + " y=" + str(p.y))
    return 0
}
```

### Pointers & Hardware (C-style)

```rnf
fn main() -> int {
    x = 42
    ptr p = &x       // address-of
    val  = *p        // dereference
    *p   = 100       // write through pointer

    // Raw memory access (for hardware registers)
    // Only meaningful in --release mode
    gpio_reg = raw_mem(0x40020014)

    // Inline assembly (--release only)
    asm { "nop" }

    return 0
}
```

### Built-in Functions

| Function | Description |
|----------|-------------|
| `print(args...)` | Print with newline |
| `exec("cmd")` | Run shell command, return stdout |
| `exec_silent("cmd")` | Run command, discard output |
| `len(x)` | Length of array or string |
| `push(arr, val)` | Append to array |
| `pop(arr)` | Remove and return last element |
| `int(x)` | Convert to int |
| `float(x)` | Convert to float |
| `str(x)` | Convert to string |
| `bool(x)` | Convert to bool |
| `exit(code)` | Exit with code |
| `sleep_ms(ms)` | Sleep milliseconds |
| `env_get(key)` | Get env variable |
| `env_set(key, val)` | Set env variable |
| `args()` | Get CLI args as array |
| `read_file(path)` | Read file to string |
| `write_file(path, data)` | Write string to file |
| `file_exists(path)` | Check if file exists |
| `split(s, sep)` | Split string → array |
| `join(arr, sep)` | Join array → string |
| `trim(s)` | Strip whitespace |
| `to_upper(s)` | Uppercase |
| `to_lower(s)` | Lowercase |
| `contains(s, sub)` | String contains |
| `starts_with(s, p)` | String starts with |
| `ends_with(s, p)` | String ends with |
| `replace(s, from, to)` | String replace |
| `format(tmpl, args...)` | String format with `{}` |
| `send(ch, val)` | Send to channel |
| `recv(ch)` | Receive from channel |
| `make_chan<T>()` | Create channel |

## Build System

RNF compiles via the LLVM pipeline:

```
.rnf source
    ↓  Lexer
    ↓  Parser (AST)
    ↓  LLVM IR codegen
    ↓  llc  (IR → object)
    ↓  musl-gcc / clang  (static link, strip)
    ↓
static binary (no deps, stripped, musl libc)
```

### Requirements for `--release`

```sh
# Ubuntu/Debian
sudo apt install llvm clang musl-tools musl-dev

# Arch Linux
sudo pacman -S llvm clang musl

# macOS (via Homebrew)
brew install llvm
```

## Examples

See the `examples/` directory:

| File | Description |
|------|-------------|
| `hello.rnf` | Basic syntax, variables, loops |
| `system.rnf` | OS automation, exec, file operations |
| `concurrent.rnf` | Goroutines, channels, parallel work |
| `hardware.rnf` | Pointers, structs, raw memory, inline ASM |
| `backend.rnf` | Backend services, request routing, benchmarks |

## License

MIT — see [LICENSE](LICENSE)

## Author

**risqinf** — [github.com/risqinf](https://github.com/risqinf)
