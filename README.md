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

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Backend](https://img.shields.io/badge/backend-LLVM-purple)

</div>

---

RNF adalah bahasa pemrograman yang dikompilasi secara statis, dirancang untuk backend berkinerja tinggi, sistem kustom, dan otomasi — dengan kekuatan C dan gaya Go.

| Fitur | Terinspirasi dari |
|-------|------------------|
| Sintaks variabel | **Shell** — `name = value` |
| Concurrency | **Go** — goroutines, channels |
| Hardware & memory | **C** — pointer, raw_mem, asm |
| Sintaks fungsi & struct | **Rust** — `fn`, `struct`, `impl` |
| Eksekusi perintah | **Bash** — `exec "cmd"` |
| Backend kompilasi | **LLVM** |

---

## Compilation Pipeline

```
  source.rnf
      │
      ├──── rnf --run ────► Interpreter (langsung, tanpa llvm)
      │
      └──── rnf --release ─► LLVM IR Codegen
                                  │
                                  ▼
                             llc -O2 (.o)
                                  │
                                  ▼
                        musl-gcc / gcc / clang
                        -static -s (stripped)
                                  │
                                  ▼
                          release/binary/<name>
```

📖 **Diagram lengkap:** [docs/architecture.md](docs/architecture.md)

---

## Install

### One-line
```sh
curl -sSL https://raw.githubusercontent.com/risqinf/rnf/main/install.sh | bash
```

### Build dari source
```sh
git clone https://github.com/risqinf/rnf
cd rnf
cargo build --release
sudo cp target/release/rnf /usr/local/bin/
```

### Dependensi untuk `--release`

| Distro | Perintah |
|--------|----------|
| AlmaLinux / RHEL / Fedora | `sudo dnf install llvm clang glibc-static` |
| Ubuntu / Debian | `sudo apt install llvm clang musl-tools` |
| Arch Linux | `sudo pacman -S llvm clang musl` |
| macOS | `brew install llvm` |

> `rnf --run` (interpreter) bekerja tanpa llvm/clang.

---

## Penggunaan

```sh
rnf --run main.rnf                          # jalankan langsung
rnf --release main.rnf                      # build → release/binary/main
rnf --release --path /usr/local/bin main.rnf  # build ke path custom

rnf check  main.rnf    # cek syntax
rnf tokens main.rnf    # debug token stream
rnf ast    main.rnf    # debug AST
rnf ir     main.rnf    # emit LLVM IR

rnf init myproject     # buat project baru
rnf version            # info versi
```

---

## Sintaks

### Variabel (Shell-style)
```rnf
name  = "Andi"
umur  = 25
pi    = 3.14
aktif = true

let score: int = 100    // opsional tipe eksplisit
```

### Fungsi (Rust-style)
```rnf
fn tambah(a: int, b: int) -> int {
    return a + b
}

fn log(msg: str) {
    print("[LOG] " + msg)
}
```

### Control Flow
```rnf
// if / else if / else
if x > 0 { print("positif") }
else      { print("negatif") }

// while
while aktif { i = i + 1 }

// loop range (0..N)
loop i in 0..100 {
    if i % 2 == 0 { continue }
    print(str(i))
}

// loop array
loop item in ["a", "b", "c"] { print(item) }
```

### Struct & Impl
```rnf
struct Server { host: str; port: int }

impl Server {
    fn addr(s: Server) -> str {
        return s.host + ":" + str(s.port)
    }
}
```

### Otomasi Sistem
```rnf
exec "mkdir -p /tmp/dir"
output = trim(exec("uname -a"))
print("OS: " + output)
```

### Concurrency
```rnf
ch = make_chan<int>()
go worker(1)
send(ch, 42)
val = recv(ch)
```

### Hardware & Low-Level
```rnf
ptr p = &x          // pointer
*p = 100            // dereference
gpio = raw_mem(0x40020014)  // memory-mapped IO
asm { "nop" }       // inline assembly
```

---

## Benchmark

```
rnf --run examples/bench.rnf
```

```
════════════════════════════════════════
  RNF BENCHMARK SUITE
════════════════════════════════════════

── Loop Arithmetic (1M iterations) ──
sum(0..1M) = 499999500000

── Function Call Overhead (100K calls) ──
Result: 100000

── Fibonacci Recursive (n=30) ──
fib(30) = 832040

── Float Arithmetic (1M iterations) ──
Result: 1.105...

── Array Operations (100K elements) ──
sum = 4950000

── Fast Power (10K calls) ──
2^10 = 1024

── System Exec (20 calls) ──
Exec benchmark done
════════════════════════════════════════
```

---

## Contoh

| File | Deskripsi |
|------|-----------|
| [`examples/hello.rnf`](examples/hello.rnf) | Hello world, variabel, loop |
| [`examples/system.rnf`](examples/system.rnf) | Otomasi OS, exec, file |
| [`examples/backend.rnf`](examples/backend.rnf) | Backend service, routing |
| [`examples/hardware.rnf`](examples/hardware.rnf) | Pointer, raw memory, ASM |
| [`examples/concurrent.rnf`](examples/concurrent.rnf) | Goroutines, channels |
| [`examples/bench.rnf`](examples/bench.rnf) | Suite benchmark lengkap |

---

## Dokumentasi

| Dokumen | Isi |
|---------|-----|
| [docs/architecture.md](docs/architecture.md) | Pipeline, diagram Mermaid, type system, ASI, IR, concurrency model |
| [docs/language-reference.md](docs/language-reference.md) | Referensi bahasa lengkap, idiom, best practices |

---

## Built-in Functions (30+)

`print` · `eprint` · `exec` · `exec_silent` · `exit` · `sleep_ms`
`str` · `int` · `float` · `bool` · `len` · `push` · `pop`
`split` · `join` · `trim` · `to_upper` · `to_lower`
`contains` · `starts_with` · `ends_with` · `replace` · `format`
`read_file` · `write_file` · `file_exists`
`args` · `env_get` · `env_set`
`make_chan` · `send` · `recv`

---

## Lisensi

MIT — lihat [LICENSE](LICENSE)

**Author:** [risqinf](https://github.com/risqinf)
