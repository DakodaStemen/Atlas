---
name: profiling-performance-tools
description: Performance profiling across languages — pprof (Go), py-spy (Python), async-profiler (JVM), Instruments (Rust/macOS), flamegraphs, and continuous profiling.
domain: observability
category: profiling
tags: [profiling, flamegraph, pprof, py-spy, async-profiler, performance, perf, Rust-profiling, continuous-profiling]
triggers: profiling flamegraph, pprof, py-spy, async-profiler, performance profile, CPU profiling, memory profiling, heap profiling, continuous profiling Pyroscope
---

# Performance Profiling Tools

## When to Use

**Profiling vs. benchmarking:** Benchmarks (testing.B, criterion, pytest-benchmark) measure *how fast* a specific operation is. Profiling answers *why* — which functions are consuming CPU or memory. You benchmark first to detect a regression, then profile to find the cause.

### Sampling vs. instrumentation

- *Sampling profilers* (pprof, py-spy, async-profiler in CPU mode) interrupt the process periodically, capture a stack trace, and aggregate. Overhead is typically 1–5%. Safe in production.
- *Instrumentation profilers* wrap every function call with timing hooks. Precise but can add 10–100× overhead — use only in development or controlled test environments.

#### Production vs. dev

- In production: sampling only, short capture windows (15–60 s), route to continuous profiler or pull on-demand via pprof endpoint or py-spy against a running PID.
- In development: heavier modes are fine — trace, allocation tracking, DHAT, JFR flight recordings.

---

## Go: pprof

### Enabling the HTTP endpoint

```go
import _ "net/http/pprof"  // registers /debug/pprof/* handlers as a side-effect

// in main():
go http.ListenAndServe(":6060", nil)
```

The endpoint exposes:

- `/debug/pprof/profile?seconds=30` — 30 s CPU profile
- `/debug/pprof/heap` — heap allocation snapshot
- `/debug/pprof/goroutine` — all live goroutine stacks
- `/debug/pprof/mutex` — mutex contention (requires `runtime.SetMutexProfileFraction(1)`)
- `/debug/pprof/block` — blocking events (requires `runtime.SetBlockProfileRate(1)`)

### Capturing and inspecting

```bash
# Download and open interactively
go tool pprof http://localhost:6060/debug/pprof/profile?seconds=30

# Save to file
curl -o cpu.pprof "http://localhost:6060/debug/pprof/profile?seconds=30"
go tool pprof cpu.pprof

# Inside the pprof shell
(pprof) top10          # top 10 functions by cumulative time
(pprof) list MyFunc    # annotated source for MyFunc
(pprof) web            # open SVG call graph in browser (requires graphviz)

# Heap
go tool pprof http://localhost:6060/debug/pprof/heap
(pprof) top -inuse_space   # live allocations by size
(pprof) top -alloc_objects # by object count
```

### Flamegraph via web UI

```bash
# pprof has a built-in HTTP UI with flamegraph support since Go 1.11
go tool pprof -http=:8123 cpu.pprof
# Open http://localhost:8123/ui/flamegraph
```

### From tests

```bash
go test -cpuprofile=cpu.pprof -memprofile=mem.pprof -bench=. ./...
go tool pprof cpu.pprof
```

---

## Go: Benchmarking

```go
func BenchmarkMyFunc(b *testing.B) {
    b.ReportAllocs()        // show allocs/op and bytes/op
    for i := 0; i < b.N; i++ {
        MyFunc()
    }
}
```

```bash
go test -bench=BenchmarkMyFunc -benchmem -count=5 ./...
# -benchmem adds allocs/op and B/op columns
# -count=5 runs five times for stable numbers
```

**runtime.ReadMemStats** for inline allocation tracking:

```go
var before, after runtime.MemStats
runtime.ReadMemStats(&before)
doWork()
runtime.ReadMemStats(&after)
fmt.Printf("allocs: %d, bytes: %d\n",
    after.Mallocs-before.Mallocs,
    after.TotalAlloc-before.TotalAlloc)
```

**Execution tracer** for scheduling and GC visibility:

```bash
go test -trace=trace.out ./...
go tool trace trace.out   # opens browser with goroutine timeline
```

---

## Python: py-spy

py-spy is written in Rust, runs outside the target process, and does not require modifying the application. It is safe for production use.

### Installation

```bash
pip install py-spy
# or with cargo:
cargo install py-spy
```

### Live top view

```bash
py-spy top --pid 12345
# Like `top` but for Python stack frames — updates every second
```

### Recording a flamegraph

```bash
# Attach to a running process and record for 30 s
py-spy record --pid 12345 --output profile.svg --duration 30

# Launch and profile a script
py-spy record --output profile.svg -- python myapp.py

# Speedscope format (for https://www.speedscope.app)
py-spy record --pid 12345 --format speedscope --output profile.speedscope

# Include native extension frames (C extensions, numpy internals)
py-spy record --pid 12345 --native --output profile.svg

# Include idle threads (useful to see blocking/sleep)
py-spy record --pid 12345 --idle --output profile.svg
```

### Async code

py-spy samples the actual OS stack, so it captures coroutine frames even if the event loop is blocked. For true async visibility, pass `--idle` to capture threads waiting on I/O. For asyncio-specific tracing, combine with `asyncio` debug mode (`PYTHONASYNCIODEBUG=1`) before recording.

### sudo requirement

On Linux, py-spy requires `sudo` or `CAP_SYS_PTRACE` to attach to another process. Running as root or adding the capability to the binary removes this:

```bash
sudo py-spy record --pid 12345 --output profile.svg
# or set the capability once:
sudo setcap cap_sys_ptrace=eip $(which py-spy)
```

---

## Python: cProfile + snakeviz

cProfile is stdlib — zero dependencies, deterministic (instruments every call).

```python
import cProfile
import pstats

# Profile a block
with cProfile.Profile() as pr:
    my_function()

stats = pstats.Stats(pr)
stats.sort_stats("cumulative")
stats.print_stats(20)          # top 20 by cumulative time
stats.dump_stats("profile.prof")
```

```bash
# From the command line
python -m cProfile -o profile.prof myapp.py

# Visualize interactively
pip install snakeviz
snakeviz profile.prof          # opens browser with icicle / sunburst view
```

cProfile overhead is noticeable (often 2–3× slowdown) — do not use it against production traffic.

---

## Python: memray

memray tracks every Python and native memory allocation. It generates flamegraphs, tables, and time-series charts of heap usage.

```bash
pip install memray
```

```bash
# Profile a script end-to-end
python -m memray run --output mem.bin myapp.py
python -m memray flamegraph mem.bin        # generates mem.flamegraph.html
python -m memray table mem.bin             # allocation table
python -m memray summary mem.bin           # peak memory summary
```

```python
# Tracker context manager for a specific block
import memray

with memray.Tracker("output.bin"):
    do_heavy_work()
```

memray requires Linux or macOS. It has native allocation tracking (malloc/free) enabled by default — disable with `--no-native` if you only care about Python-level allocations.

---

## JVM: async-profiler

async-profiler uses `AsyncGetCallTrace` + `perf_events` on Linux (or `SIGPROF` + `perf` on macOS). It avoids the safepoint bias that plagues earlier Java profilers.

### Modes

| Mode | What it samples |
| ------ | ---------------- |
| `cpu` | Threads on-CPU (via perf_events / SIGPROF) |
| `alloc` | Heap allocations (TLAB introspection) |
| `wall` | All threads regardless of state (blocked, sleeping, running) |
| `lock` | Lock contention |
| `nativemem` | Native (malloc) memory |

### Attaching to a running JVM

```bash
# Download release from https://github.com/async-profiler/async-profiler/releases
# Extract, then:

./asprof -e cpu -d 30 -f cpu.jfr <PID>       # 30 s CPU profile, JFR output
./asprof -e alloc -d 30 -f alloc.html <PID>   # allocation flamegraph HTML
./asprof -e wall -d 30 -f wall.html <PID>     # wall-clock profile

# JFR output can be opened in JDK Mission Control
./asprof -e cpu -d 60 -f recording.jfr <PID>
```

### As a JVM agent

```bash
java -agentpath:/path/to/libasyncProfiler.so=start,event=cpu,file=cpu.jfr,duration=30 \
     -jar myapp.jar
```

### IntelliJ integration

IntelliJ IDEA Ultimate bundles async-profiler under the Profiler tool window. Run any configuration with "Profile" instead of "Run" to get an inline flamegraph without leaving the IDE.

---

## JVM: JFR + JMC

Java Flight Recorder (JFR) is a low-overhead (< 2%) profiling framework built into the JVM since JDK 11. JDK Mission Control (JMC) is the GUI for analysing `.jfr` files.

### Capturing

```bash
# Enable at startup
java -XX:StartFlightRecording=duration=60s,filename=recording.jfr \
     -XX:FlightRecorderOptions=stackdepth=256 \
     -jar myapp.jar

# Trigger on a running process via jcmd
jcmd <PID> JFR.start duration=60s filename=recording.jfr
jcmd <PID> JFR.stop name=1
jcmd <PID> JFR.dump name=1 filename=recording.jfr
```

### Useful JVM event types in JMC

- `jdk.CPUSample` — periodic CPU samples (method profiler)
- `jdk.ObjectAllocationInNewTLAB` / `jdk.ObjectAllocationOutsideTLAB` — allocation sites
- `jdk.GarbageCollection` — GC pause duration and cause
- `jdk.MonitorEnter` / `jdk.MonitorWait` — lock contention
- `jdk.ThreadSleep` — where threads are sleeping
- `jdk.NativeMethodSample` — native call sites

Open `recording.jfr` in JMC, then use the "Method Profiling" page for flamegraph-style analysis.

---

## Rust: cargo flamegraph

cargo flamegraph wraps `perf` on Linux and `dtrace` on macOS/FreeBSD. It compiles your binary with debug symbols, runs it, and produces an SVG flamegraph.

### Installation (Rust: cargo flamegraph)

```bash
# Linux: ensure perf is installed
sudo apt-get install linux-tools-common linux-tools-generic

# macOS: DTrace is available by default (SIP may need partial disable)

cargo install flamegraph
```

### Usage

```bash
# Profile the default binary
cargo flamegraph

# Profile a specific binary with arguments
cargo flamegraph --bin myserver -- --port 8080

# Profile a benchmark (integrates with criterion)
cargo flamegraph --bench mybench

# Output file
cargo flamegraph --output profile.svg
```

**Critical:** build with debug symbols enabled in release mode. Add to `Cargo.toml`:

```toml
[profile.release]
debug = true        # embed DWARF symbols; does not affect optimization
```

Without this, frames will appear as `[unknown]` or mangled addresses.

### Symbol resolution

```bash
# On Linux, if frames are missing, ensure kernel.perf_event_paranoid allows user-space profiling:
sudo sysctl -w kernel.perf_event_paranoid=1
# Or run as root:
sudo cargo flamegraph --bin myserver
```

---

## Rust: criterion benchmarks + DHAT

### criterion

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "my_bench"
harness = false
```

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_my_func(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_func");
    for size in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
            b.iter(|| my_func(s));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_my_func);
criterion_main!(benches);
```

```bash
cargo bench                          # runs all benchmarks
cargo bench -- my_func               # runs matching benchmarks
# HTML report generated at target/criterion/report/index.html
```

### DHAT (heap profiling)

DHAT is part of Valgrind's tool suite but also ships as a standalone Rust crate via the `dhat` crate.

```toml
[dev-dependencies]
dhat = "0.3"
```

```rust
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    let _profiler = dhat::Profiler::new_heap();
    run_program();
    // profile saved to dhat-heap.json on drop
}
```

```bash
cargo run --release
# Open dhat-heap.json at https://nnethercote.github.io/dh_view/dh_view.html
```

DHAT shows peak heap size, total allocations, and which allocation sites are live at peak — useful for finding allocation-heavy hot paths before they show up in CPU profiles.

---

## Flamegraph Interpretation

Flamegraphs come in two orientations:

- **Top-down (flame):** root at the bottom, callees stacked above callers. The top edge shows where CPU time is spent.
- **Icicle (bottom-up):** root at top, callees below. Some tools (speedscope) default to this; semantics are the same.

### Reading rules

1. **Width = time.** A wide bar means that function (and everything it calls) consumed a large share of samples. Narrower bars are cheaper.
2. **The top of a plateau is the hot path.** If a bar at the top of a stack is wide and has no tall children, that function itself is where CPU is being spent — it is the optimization target.
3. **Color is arbitrary.** Flamegraph colors are randomized to improve readability, not to indicate anything about the function.
4. **Sorted alphabetically per level, not by call order.** Adjacent bars at the same level that look like one wide bar may be multiple distinct callers all calling the same function.
5. **Async gaps:** In async runtimes (tokio, asyncio, Go goroutines), a flat profile may show the runtime scheduler as the widest bar. Use wall-clock mode (py-spy `--idle`, async-profiler `wall`) to expose time spent blocked on I/O or awaiting.
6. **Inlined functions:** compilers inline aggressively at `-O2`/release. If a frame is absent, it may be inlined into its parent. Rust: `debug = true` in profile. JVM: async-profiler re-inflates inlined frames from debug info when available.

---

## Continuous Profiling

Continuous profiling runs a sampling profiler at low overhead 24/7 in production, stores profiles centrally, and lets you correlate performance spikes with deployments or incidents.

### Grafana Pyroscope (formerly Phlare + Pyroscope)

Pyroscope (Grafana-acquired 2023, now Grafana Pyroscope 1.0+) is the dominant open-source option. Architecture mirrors Grafana Mimir/Loki/Tempo:

- **Agents/SDKs** run in-process (Go, Python, Java, Rust, Ruby, .NET) or as sidecars, push profiles to the Pyroscope server over gRPC/HTTP.
- **Server** is horizontally scalable, stores profiles in object storage (S3/GCS), supports multi-tenancy.
- **UI** integrates with Grafana Explore — diff profiles across time ranges, overlay with metrics/traces.

```bash
# Go SDK example
import "github.com/grafana/pyroscope-go"

pyroscope.Start(pyroscope.Config{
    ApplicationName: "myapp",
    ServerAddress:   "http://pyroscope:4040",
    ProfileTypes: []pyroscope.ProfileType{
        pyroscope.ProfileCPU,
        pyroscope.ProfileAllocObjects,
        pyroscope.ProfileAllocSpace,
    },
})
```

Overhead: < 1% CPU in sampling mode (default 100 Hz). Memory: ~10–50 MB resident depending on stack depth and label cardinality.

### Polar Signals / parca

Polar Signals Cloud (and the open-source `parca`) uses eBPF for zero-instrumentation profiling — no SDK, no code change, kernel-level stack unwinding. Works for any language in a containerized environment.

```bash
# parca-agent as a DaemonSet on Kubernetes — profiles all pods automatically
kubectl apply -f https://github.com/parca-dev/parca-agent/releases/latest/download/kubernetes-manifest.yaml
```

---

## Linux perf

`perf` is the Linux kernel's built-in performance counter interface. It works at the hardware and OS level and does not require language-specific tooling.

```bash
# CPU event summary for a command
perf stat -e cycles,instructions,cache-misses,branch-misses -- ./myprogram

# Record a CPU profile (default: hardware cycles)
perf record -g ./myprogram                # -g = collect call graphs
perf record -F 999 -g -- ./myprogram      # 999 Hz sampling rate

# Report
perf report --stdio                        # text report
perf report                                # interactive TUI

# Generate flamegraph with FlameGraph scripts
perf script | ./stackcollapse-perf.pl | ./flamegraph.pl > out.svg

# Profile a running process
perf record -F 99 -g -p <PID> -- sleep 30
```

Useful hardware events:

```bash
perf list | grep -i cache      # list available cache events
perf stat -e L1-dcache-load-misses,LLC-load-misses ./myprogram
```

perf requires `perf_event_paranoid <= 1` for unprivileged use:

```bash
echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid
```

---

## Critical Rules / Gotchas

**Debug symbols (Rust):** Without `debug = true` in `[profile.release]`, cargo flamegraph produces unreadable `[unknown]` frames. Always add it before profiling release builds. Symbols do not affect runtime performance — only binary size.

**JIT compilation effects (JVM):** The JVM spends the first few seconds (warm-up) interpreting bytecode, then JIT-compiles hot paths. Profiles taken during warm-up look very different from steady-state. Always discard the first 5–10 s, or use `-XX:+TieredCompilation` with a forced warm-up before capturing.

**Sampling bias:** Sampling profilers are accurate over many samples. A 30 s capture at 99 Hz gives ~3000 samples — generally enough. Short captures (< 5 s) on fast functions can mislead. Increase sampling rate (`-F 999` in perf, `--rate 1000` in py-spy) for short-lived programs.

**Async gaps in Go:** pprof CPU profiles do not show goroutines blocked in channel operations or syscalls. Use the blocking profile (`/debug/pprof/block`) and the goroutine profile (`/debug/pprof/goroutine`) in combination. The execution tracer (`go tool trace`) shows scheduling latency.

**cProfile in asyncio:** `cProfile` and `asyncio` interact badly — coroutine switches look like function returns, so cumulative times are understated. Use py-spy or `yappi` (aware of greenlets/asyncio) instead.

**async-profiler on containers:** By default, Linux containers restrict `perf_events`. Add `--cap-add SYS_ADMIN` (or `--privileged`) to the container, or use `ctimer` mode which does not require perf_events:

```bash
./asprof -e ctimer -d 30 -f cpu.html <PID>
```

**Overhead in production:** Never run instrumentation profilers (cProfile, JFR with all events enabled) at production load. Sampling at 99–100 Hz is generally safe. If in doubt, route profiling traffic to a single replica rather than all nodes.

**perf_event_paranoid:** Many CI environments set this to 3 (deny all). `perf record` and async-profiler CPU mode will fail silently or with a permissions error. Fall back to `ctimer` (async-profiler) or `itimer` modes, or use py-spy which does not need perf_events.

---

## References

- Go diagnostics: <https://go.dev/doc/diagnostics>
- pprof flamegraph blog (Julia Evans): <https://jvns.ca/blog/2017/09/24/profiling-go-with-pprof/>
- py-spy GitHub: <https://github.com/benfred/py-spy>
- async-profiler GitHub: <https://github.com/async-profiler/async-profiler>
- async-profiler manual by use cases: <https://krzysztofslusarski.github.io/2022/12/12/async-manual.html>
- cargo-flamegraph GitHub: <https://github.com/flamegraph-rs/flamegraph>
- The Rust Performance Book: <https://nnethercote.github.io/perf-book/profiling.html>
- Grafana Pyroscope: <https://grafana.com/oss/pyroscope/>
- Grafana Pyroscope docs: <https://grafana.com/docs/pyroscope/latest/>
- Brendan Gregg flamegraph scripts: <https://github.com/brendangregg/FlameGraph>
- parca (eBPF continuous profiling): <https://github.com/parca-dev/parca>
