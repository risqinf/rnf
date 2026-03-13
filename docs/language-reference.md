# RNF Language Reference

Referensi lengkap bahasa pemrograman RNF v0.1.0.

---

## Quick Start

```sh
# Install
curl -sSL https://raw.githubusercontent.com/risqinf/rnf/main/install.sh | bash

# Buat project
rnf init myproject
cd myproject

# Jalankan
rnf --run src/main.rnf

# Compile ke binary
rnf --release src/main.rnf
```

---

## Variabel

RNF menggunakan gaya shell: tidak perlu keyword wajib.

```rnf
// Shell-style (direkomendasikan)
name    = "Budi"
umur    = 25
tinggi  = 170.5
aktif   = true

// Typed (opsional, lebih eksplisit)
let score: int   = 100
let ratio: float = 0.75
let msg:   str   = "halo"
let ok:    bool  = false

// Nil
data = nil

// Reassign
name = "Andi"     // OK — RNF mutable by default
```

---

## Tipe Data

| Tipe | Ukuran | Contoh |
|------|--------|--------|
| `int` | 64-bit signed | `42`, `-100`, `0xFF` (hex) |
| `float` | 64-bit double | `3.14`, `-0.001`, `1e9` |
| `str` | null-term string | `"hello"`, `"line\n"` |
| `bool` | 1-bit | `true`, `false` |
| `nil` | null pointer | `nil` |
| `ptr<T>` | raw pointer | `ptr p = &x` |
| `chan<T>` | channel | `make_chan<int>()` |
| `[T]` | array | `[1, 2, 3]` |
| `StructName` | struct | `Point { x: 0, y: 0 }` |

---

## Fungsi

```rnf
// Fungsi biasa
fn tambah(a: int, b: int) -> int {
    return a + b
}

// Tanpa return value
fn cetak(msg: str) {
    print(msg)
}

// Rekursif
fn faktorial(n: int) -> int {
    if n <= 1 { return 1 }
    return n * faktorial(n - 1)
}

// Fungsi dengan banyak return path
fn kategori(nilai: int) -> str {
    if nilai >= 90 { return "A" }
    if nilai >= 80 { return "B" }
    if nilai >= 70 { return "C" }
    return "D"
}
```

---

## Control Flow

### if / else

```rnf
x = 42

if x > 100 {
    print("besar")
} else if x > 50 {
    print("sedang")
} else {
    print("kecil")
}

// Single line (dengan brace tetap wajib)
if aktif { print("ON") }
```

### while

```rnf
i = 0
while i < 10 {
    print(str(i))
    i = i + 1
}

// Loop dengan break
while true {
    input = exec("cat /dev/urandom | head -c1 | od -An -tu1 | tr -d ' '")
    if int(trim(input)) > 200 { break }
}
```

### loop (for-range)

```rnf
// Range numerik: 0..N  (0 sampai N-1)
loop i in 0..10 {
    print(str(i))
}

// Loop array
buah = ["apel", "mangga", "jeruk"]
loop b in buah {
    print("Buah: " + b)
}

// Dengan continue / break
loop i in 0..100 {
    if i % 2 == 0 { continue }  // skip genap
    if i > 10 { break }          // stop di 11
    print(str(i))                // output: 1 3 5 7 9
}
```

---

## Struct

```rnf
// Definisi
struct Mahasiswa {
    nama:  str
    nim:   int
    ipk:   float
    aktif: bool
}

// Implementasi method
impl Mahasiswa {
    fn new(nama: str, nim: int) -> Mahasiswa {
        return Mahasiswa {
            nama:  nama,
            nim:   nim,
            ipk:   0.0,
            aktif: true,
        }
    }

    fn to_string(m: Mahasiswa) -> str {
        return m.nama + " (" + str(m.nim) + ") IPK=" + str(m.ipk)
    }

    fn lulus(m: Mahasiswa) -> bool {
        return m.ipk >= 2.0
    }
}

fn main() -> int {
    s = Mahasiswa {
        nama:  "Andi",
        nim:   12345,
        ipk:   3.5,
        aktif: true,
    }

    print(s.nama)
    print(str(s.ipk))

    if s.aktif {
        print("Masih kuliah")
    }

    return 0
}
```

---

## Pointer

```rnf
fn main() -> int {
    x = 100

    // Deklarasi pointer
    ptr p = &x       // p menunjuk ke x

    // Dereference (baca)
    val = *p          // val = 100

    // Dereference (tulis)
    *p = 999          // x sekarang = 999

    print(str(x))     // output: 999

    // Struct pointer
    n = 42
    ptr np = &n
    *np = *np * 2     // n = 84

    return 0
}
```

---

## Sistem & Otomasi

```rnf
fn main() -> int {
    // Jalankan perintah (tidak capture output)
    exec "mkdir -p /tmp/mydir"
    exec "chmod 755 /tmp/mydir"

    // Jalankan dan capture output
    hostname = trim(exec("hostname"))
    kernel   = trim(exec("uname -r"))
    uptime   = trim(exec("uptime -p"))

    print("Host:   " + hostname)
    print("Kernel: " + kernel)
    print("Uptime: " + uptime)

    // Komposisi perintah
    dir = "/tmp/backup"
    exec("mkdir -p " + dir)
    exec("cp -r /etc/nginx " + dir + "/nginx_backup")

    // Cek exit code
    result = exec("ping -c1 8.8.8.8 2>&1")
    if contains(result, "1 received") {
        print("Internet: OK")
    } else {
        print("Internet: TIDAK TERHUBUNG")
    }

    // Loop dengan exec
    services = ["nginx", "mysql", "redis"]
    loop s in services {
        status = trim(exec("systemctl is-active " + s + " 2>/dev/null || echo inactive"))
        print(s + ": " + status)
    }

    return 0
}
```

---

## Concurrency (Goroutines & Channels)

```rnf
fn worker(id: int, ch: chan<int>) {
    print("Worker " + str(id) + " mulai")

    // Simulasi kerja
    i = 0
    while i < 1000000 {
        i = i + 1
    }

    // Kirim hasil ke channel
    send(ch, id * 100)
    print("Worker " + str(id) + " selesai")
}

fn main() -> int {
    // Buat channel
    results = make_chan<int>()

    // Spawn goroutines
    go worker(1, results)
    go worker(2, results)
    go worker(3, results)

    // Tunggu hasil
    r1 = recv(results)
    r2 = recv(results)
    r3 = recv(results)

    total = r1 + r2 + r3
    print("Total: " + str(total))

    return 0
}
```

---

## Hardware & Low-Level

```rnf
fn main() -> int {
    // Pointer arithmetik
    x = 42
    ptr p = &x
    *p = 100

    // Memory-mapped I/O (--release only)
    // Contoh: baca register GPIO pada embedded system
    gpio_base = 0x40020000
    gpio_out  = raw_mem(gpio_base + 0x14)  // output register

    // Inline assembly (--release only)
    // Akan dicompile menjadi LLVM inline asm
    asm { "nop" }
    asm { "cpuid" }

    // Bitwise operations via aritmatika
    flags     = 0xFF
    mask      = 0x0F
    low_nibble = flags % 16   // flags & 0x0F
    high_bit  = flags / 128   // flags >> 7

    print("low nibble: " + str(low_nibble))
    print("high bit: " + str(high_bit))

    return 0
}
```

---

## Built-in Functions Lengkap

### I/O
```rnf
print("hello")               // cetak dengan newline
print("a", "b", "c")        // cetak semua args dipisah spasi
eprint("error msg")          // cetak ke stderr
```

### Konversi
```rnf
str(42)          // "42"
str(3.14)        // "3.14"
str(true)        // "true"
int("42")        // 42
int(3.14)        // 3
float("3.14")    // 3.14
float(42)        // 42.0
bool(1)          // true
bool(0)          // false
bool("")         // false
bool("x")        // true
```

### Array
```rnf
arr = [10, 20, 30]
len(arr)                     // 3
push(arr, 40)                // [10, 20, 30, 40]
last = pop(arr)              // last=40, arr=[10, 20, 30]
arr[0]                       // 10
arr[1] = 99                  // set index
```

### String
```rnf
len("hello")                 // 5
trim("  hello  ")            // "hello"
to_upper("hello")            // "HELLO"
to_lower("HELLO")            // "hello"
contains("hello", "ell")     // true
starts_with("hello", "he")   // true
ends_with("hello", "lo")     // true
replace("hello", "l", "r")   // "herro"
split("a,b,c", ",")          // ["a", "b", "c"]
join(["a","b","c"], "-")      // "a-b-c"
format("Hello {}!", "World") // "Hello World!"
```

### Sistem
```rnf
exec("ls -la")               // jalankan, tampilkan output
out = exec("uname -a")       // capture output ke string
exec_silent("rm -f /tmp/x")  // jalankan, buang output
exit(0)                      // keluar dengan kode
exit(1)                      // keluar dengan error
sleep_ms(1000)               // tidur 1 detik
args()                       // array argumen CLI
env_get("HOME")              // baca env variable
env_set("MY_VAR", "value")   // set env variable
```

### File
```rnf
content = read_file("/etc/hostname")     // baca file
write_file("/tmp/out.txt", "hello\n")   // tulis file
file_exists("/etc/passwd")              // true/false
```

---

## Error Handling Pattern

RNF saat ini tidak punya exception — gunakan nilai return dan exec exit codes:

```rnf
fn baca_config(path: str) -> str {
    if !file_exists(path) {
        eprint("Config tidak ditemukan: " + path)
        exit(1)
    }
    return read_file(path)
}

fn cek_service(name: str) -> bool {
    result = exec("systemctl is-active " + name + " 2>/dev/null")
    return contains(result, "active")
}

fn main() -> int {
    config = baca_config("/etc/myapp/config.toml")
    print("Config loaded: " + str(len(config)) + " bytes")

    if !cek_service("nginx") {
        eprint("nginx tidak berjalan!")
        exec "systemctl start nginx"
    }

    return 0
}
```

---

## Idiom & Best Practices

```rnf
// ✅ Gunakan string concat untuk logging
fn log(level: str, msg: str) {
    prefix = "[" + to_upper(level) + "]"
    print(prefix + " " + msg)
}

// ✅ Wrap exec dalam fungsi
fn run(cmd: str) -> str {
    return trim(exec(cmd))
}

fn run_or_fail(cmd: str) {
    result = exec(cmd + " 2>&1")
    if contains(result, "error") || contains(result, "Error") {
        eprint("Command failed: " + cmd)
        eprint("Output: " + result)
        exit(1)
    }
}

// ✅ Struct untuk konfigurasi
struct Config {
    host:  str
    port:  int
    debug: bool
}

fn default_config() -> Config {
    return Config { host: "localhost", port: 8080, debug: false }
}

// ✅ Return value dari main selalu int
fn main() -> int {
    log("info", "Starting…")
    cfg = default_config()
    log("info", "Port: " + str(cfg.port))
    return 0
}
```
