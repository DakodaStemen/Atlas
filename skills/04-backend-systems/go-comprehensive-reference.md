---
name: go-comprehensive-reference
description: Comprehensive Go reference — idiomatic patterns, web frameworks (Gin, Chi, Echo, stdlib), concurrency (goroutines, channels, errgroup), testing (table-driven, httptest, testify, fuzz), gRPC, generics, slog, iter, and critical gotchas.
domain: languages
category: go
tags: [Go, Golang, Gin, Chi, Echo, goroutines, channels, concurrency, testing, gRPC, generics, slog, iter]
triggers: Go best practices, Go web framework, Go concurrency, Go testing, Go gRPC, Go idiomatic, Go generics, slog, Go iter
---

# Go Comprehensive Reference

This document consolidates Go idiomatic patterns, web frameworks, concurrency, testing, and gRPC into a single high-density reference.

---

## Part 1: Idiomatic Patterns


## Context Propagation

`context.Context` is the first parameter of every function that does I/O, makes network calls, or can be cancelled. Never store a context in a struct; pass it explicitly.

```go
func FetchUser(ctx context.Context, id int) (*User, error) {
    req, err := http.NewRequestWithContext(ctx, http.MethodGet, userURL(id), nil)
    if err != nil {
        return nil, err
    }
    resp, err := http.DefaultClient.Do(req)
    // ...
}
```

### Cancellation and deadlines

```go
ctx, cancel := context.WithCancel(context.Background())
defer cancel() // always defer cancel to avoid goroutine leak

ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()

ctx, cancel := context.WithDeadline(context.Background(), time.Now().Add(5*time.Second))
defer cancel()
```

#### WithValue — use unexported key types to prevent collisions

```go
type ctxKey string

const requestIDKey ctxKey = "requestID"

func WithRequestID(ctx context.Context, id string) context.Context {
    return context.WithValue(ctx, requestIDKey, id)
}

func RequestIDFromContext(ctx context.Context) (string, bool) {
    id, ok := ctx.Value(requestIDKey).(string)
    return id, ok
}
```

Check `ctx.Err()` in long loops:

```go
for _, item := range items {
    if ctx.Err() != nil {
        return ctx.Err()
    }
    process(item)
}
```

---

## Error Handling

**Wrapping with `%w`** (Go 1.13+):

```go
if err := db.QueryRow(ctx, q, id).Scan(&u); err != nil {
    return nil, fmt.Errorf("fetchUser id=%d: %w", id, err)
}
```

### Unwrapping

```go
if errors.Is(err, sql.ErrNoRows) {
    // sentinel match through the chain
}

var pathErr *os.PathError
if errors.As(err, &pathErr) {
    fmt.Println("path:", pathErr.Path)
}
```

**Sentinel errors** — exported, unexported, or `errors.New`:

```go
var ErrNotFound = errors.New("not found")

// callers: errors.Is(err, ErrNotFound)
```

**Custom error types** for structured information:

```go
type ValidationError struct {
    Field   string
    Message string
}

func (e *ValidationError) Error() string {
    return fmt.Sprintf("validation failed on %s: %s", e.Field, e.Message)
}
```

Never discard errors silently. If you genuinely must, document why:

```go
_ = f.Close() // best-effort; real error already returned above
```

---

## Generics (Go 1.18+)

Type parameters let you write a function or type once and use it with any type that satisfies a constraint.

```go
// Constraint: any type that supports <
type Ordered interface {
    ~int | ~int8 | ~int16 | ~int32 | ~int64 |
        ~uint | ~uint8 | ~uint16 | ~uint32 | ~uint64 |
        ~float32 | ~float64 | ~string
}

func Min[T Ordered](a, b T) T {
    if a < b {
        return a
    }
    return b
}

// comparable constraint enables == and !=
func Contains[T comparable](slice []T, val T) bool {
    for _, v := range slice {
        if v == val {
            return true
        }
    }
    return false
}
```

### Generic data structure

```go
type Stack[T any] struct {
    items []T
}

func (s *Stack[T]) Push(v T) {
    s.items = append(s.items, v)
}

func (s *Stack[T]) Pop() (T, bool) {
    var zero T
    if len(s.items) == 0 {
        return zero, false
    }
    top := s.items[len(s.items)-1]
    s.items = s.items[:len(s.items)-1]
    return top, true
}
```

**`any` is an alias for `interface{}`** — use it in type parameter lists and ordinary code. Prefer specific constraints when possible; `any` loses type information.

Generic map keys utility (now in `maps` stdlib):

```go
func Keys[K comparable, V any](m map[K]V) []K {
    keys := make([]K, 0, len(m))
    for k := range m {
        keys = append(keys, k)
    }
    return keys
}
```

Go 1.21 introduced `slices`, `maps`, and `cmp` packages built on generics.

---

## slog (Go 1.21)

`log/slog` is the structured logging package in the standard library. It replaces ad-hoc `log.Printf` patterns with key-value pairs suitable for JSON or logfmt output.

### Basic usage

```go
import "log/slog"

slog.Info("server started", "addr", ":8080", "pid", os.Getpid())
slog.Warn("disk low", "free_gb", 2)
slog.Error("request failed", "err", err, "path", r.URL.Path)
```

#### Creating a logger with a handler

```go
// Text output (key=value)
logger := slog.New(slog.NewTextHandler(os.Stdout, nil))

// JSON output
logger := slog.New(slog.NewJSONHandler(os.Stderr, &slog.HandlerOptions{
    Level: slog.LevelDebug,
}))

slog.SetDefault(logger) // replaces the package-level default
```

#### LogAttrs — zero-allocation fast path

```go
slog.LogAttrs(ctx, slog.LevelInfo, "user login",
    slog.String("user", username),
    slog.Int("attempt", attempt),
    slog.Duration("latency", elapsed),
)
```

#### Logger.With — attach common fields once

```go
reqLog := logger.With(
    slog.String("request_id", requestID),
    slog.String("method", r.Method),
)
reqLog.Info("handler started")
reqLog.Error("handler failed", "err", err)
```

#### WithGroup — namespace a set of attributes

```go
logger.WithGroup("http").Info("request",
    slog.String("method", r.Method),
    slog.Int("status", 200),
)
// JSON: {"http":{"method":"GET","status":200}}
```

#### Custom Handler interface

```go
type Handler interface {
    Enabled(context.Context, Level) bool
    Handle(context.Context, Record) error
    WithAttrs(attrs []Attr) Handler
    WithGroup(name string) Handler
}
```

Use `slog.LevelVar` for runtime-adjustable log levels without restarting the process.

#### LogValue — redact sensitive fields

```go
type Token string

func (t Token) LogValue() slog.Value {
    return slog.StringValue("REDACTED")
}
```

---

## iter (Go 1.22/1.23)

Go 1.22 introduced range-over-function (experimental), finalized in Go 1.23. The `iter` package defines two standard iterator types.

```go
type Seq[V any] func(yield func(V) bool)
type Seq2[K, V any] func(yield func(K, V) bool)
```

The iterator calls `yield` for each element. If `yield` returns `false`, the iterator must stop (break, return, or panic inside the range body does this).

### Custom collection iterator

```go
type Set[E comparable] struct {
    m map[E]struct{}
}

func (s *Set[E]) All() iter.Seq[E] {
    return func(yield func(E) bool) {
        for v := range s.m {
            if !yield(v) {
                return
            }
        }
    }
}

// Usage — reads exactly like ranging over a built-in
for v := range mySet.All() {
    fmt.Println(v)
}
```

#### Binary tree in-order traversal

```go
func (t *Tree[E]) All() iter.Seq[E] {
    return func(yield func(E) bool) {
        t.push(yield)
    }
}

func (t *Tree[E]) push(yield func(E) bool) bool {
    if t == nil {
        return true
    }
    return t.left.push(yield) && yield(t.val) && t.right.push(yield)
}
```

#### Adapter — Filter

```go
func Filter[V any](f func(V) bool, s iter.Seq[V]) iter.Seq[V] {
    return func(yield func(V) bool) {
        for v := range s {
            if f(v) && !yield(v) {
                return
            }
        }
    }
}
```

**Pull iterators** — for pairwise or manual stepping:

```go
func EqSeq[E comparable](s1, s2 iter.Seq[E]) bool {
    next1, stop1 := iter.Pull(s1)
    defer stop1()
    next2, stop2 := iter.Pull(s2)
    defer stop2()
    for {
        v1, ok1 := next1()
        v2, ok2 := next2()
        if !ok1 {
            return !ok2
        }
        if ok1 != ok2 || v1 != v2 {
            return false
        }
    }
}
```

Standard library additions: `slices.All`, `slices.Values`, `slices.Collect`, `maps.Keys`, `maps.Values`, `maps.Collect`.

---

## Concurrency Patterns

### Pipeline

```go
func generate(nums ...int) <-chan int {
    out := make(chan int)
    go func() {
        for _, n := range nums {
            out <- n
        }
        close(out)
    }()
    return out
}

func square(in <-chan int) <-chan int {
    out := make(chan int)
    go func() {
        for n := range in {
            out <- n * n
        }
        close(out)
    }()
    return out
}
```

#### Fan-out / fan-in

```go
func merge(cs ...<-chan int) <-chan int {
    var wg sync.WaitGroup
    out := make(chan int)
    output := func(c <-chan int) {
        for n := range c {
            out <- n
        }
        wg.Done()
    }
    wg.Add(len(cs))
    for _, c := range cs {
        go output(c)
    }
    go func() {
        wg.Wait()
        close(out)
    }()
    return out
}
```

**errgroup** — structured concurrency with error collection:

```go
import "golang.org/x/sync/errgroup"

g, ctx := errgroup.WithContext(context.Background())

g.Go(func() error {
    return fetchUsers(ctx)
})
g.Go(func() error {
    return fetchOrders(ctx)
})

if err := g.Wait(); err != nil {
    return err
}
```

#### sync.Once — one-time initialization

```go
var (
    instance *DB
    once     sync.Once
)

func GetDB() *DB {
    once.Do(func() {
        instance = openDB()
    })
    return instance
}
```

**sync.Map — concurrent map** (use sparingly; prefer a mutex-protected map when the key set is stable):

```go
var m sync.Map

m.Store("key", "value")
v, ok := m.Load("key")
m.LoadOrStore("key", "default")
m.Delete("key")
m.Range(func(k, v any) bool {
    fmt.Println(k, v)
    return true // continue
})
```

Always use `go vet -race` or run tests with `-race`. A mutex-guarded map is usually clearer and faster than `sync.Map` for non-pathological access patterns.

---

## Table-Driven Tests

```go
func TestAdd(t *testing.T) {
    tests := []struct {
        name string
        a, b int
        want int
    }{
        {"positive", 1, 2, 3},
        {"negative", -1, -2, -3},
        {"zero", 0, 0, 0},
    }

    for _, tc := range tests {
        t.Run(tc.name, func(t *testing.T) {
            t.Parallel() // safe when tc is loop-local (Go 1.22+ always; pre-1.22 capture tc explicitly)
            got := Add(tc.a, tc.b)
            if got != tc.want {
                t.Errorf("Add(%d, %d) = %d, want %d", tc.a, tc.b, got, tc.want)
            }
        })
    }
}
```

**testdata directory** — place input/golden files in `testdata/`; Go tooling ignores it during builds.

```go
data, err := os.ReadFile("testdata/input.json")
```

**`testing/fstest.MapFS`** for in-memory file system in tests:

```go
fsys := fstest.MapFS{
    "config.yaml": {Data: []byte("key: value")},
}
```

---

## Package Design

- Package names: single lowercase word, no underscores, no mixedCase. `bufio`, not `buf_io` or `bufIo`.
- Exported names already carry the package prefix — don't repeat it: `ring.New()` not `ring.NewRing()`.
- `internal/` packages are importable only by code rooted at the parent of `internal`. Use for implementation details that must not become public API.
- Avoid `init()` except for registrations that genuinely cannot happen elsewhere (e.g., `database/sql` driver registration). Hidden initialization order is hard to test.
- Zero values should be usable without explicit initialization wherever possible (`var b bytes.Buffer` is ready immediately).

---

## Functional Options

The functional options pattern lets you build configurable structs without a growing constructor signature and without requiring callers to set every field.

```go
type Server struct {
    addr    string
    timeout time.Duration
    maxConn int
    tls     *tls.Config
}

type Option func(*Server)

func WithTimeout(d time.Duration) Option {
    return func(s *Server) { s.timeout = d }
}

func WithMaxConn(n int) Option {
    return func(s *Server) { s.maxConn = n }
}

func WithTLS(cfg *tls.Config) Option {
    return func(s *Server) { s.tls = cfg }
}

func NewServer(addr string, opts ...Option) *Server {
    s := &Server{
        addr:    addr,
        timeout: 30 * time.Second, // sensible defaults
        maxConn: 100,
    }
    for _, o := range opts {
        o(s)
    }
    return s
}

// Usage
srv := NewServer(":8080",
    WithTimeout(10*time.Second),
    WithTLS(tlsCfg),
)
```

---

## embed Package

`//go:embed` embeds files or directory trees at compile time. Requires `import _ "embed"` (or the package itself when using `fs.FS`).

```go
import (
    "embed"
    "html/template"
)

//go:embed templates/*.html
var templateFS embed.FS

var tmpl = template.Must(template.ParseFS(templateFS, "templates/*.html"))

// Single file into a string
//go:embed version.txt
var version string

// Single file into a []byte
//go:embed logo.png
var logoPNG []byte
```

`embed.FS` implements `fs.FS`, `fs.ReadFileFS`, and `fs.ReadDirFS` — pass it to any function that accepts `fs.FS`.

---

## go:generate and Code Generation

`go generate` runs commands specified in `//go:generate` comments. It is not automatic; you invoke it explicitly.

```go
//go:generate stringer -type=Direction
//go:generate mockgen -source=store.go -destination=mocks/store_mock.go
//go:generate protoc --go_out=. api.proto
```

Run with:

```bash
go generate ./...
```

**stringer** generates `String() string` for iota enums. **mockgen** (from `google/mock`) generates mock implementations from interfaces. Commit generated files to source control; generation is for developers, not CI build steps.

---

## go vet and staticcheck

`go vet` is built into the toolchain. Run it in CI:

```bash
go vet ./...
```

Common checks caught by `go vet`:

- `printf` format string mismatches (`%d` with a string argument)
- Unreachable code after `return`
- Incorrect mutex copy (passing `sync.Mutex` by value)
- `//go:build` constraint mistakes
- `slog` key-value alternation errors (Go 1.21+)

**golangci-lint** bundles `go vet`, `staticcheck`, `errcheck`, `gosimple`, and many others:

```yaml
# .golangci.yml
linters:
  enable:
    - errcheck
    - govet
    - staticcheck
    - gosimple
    - ineffassign
    - unused
linters-settings:
  govet:
    enable-all: true
```

Run:

```text
golangci-lint run ./...
```

`staticcheck` (SA category) catches things `go vet` misses: deprecated API usage, impossible conditions, redundant type assertions.

---

## Critical Rules / Gotchas

**Goroutine leaks** — every goroutine that reads from a channel must have a way to exit. Close the channel when the producer is done, or use a `done` channel/context cancellation:

```go
// Bad: goroutine blocks forever if nobody reads out
go func() {
    out <- compute() // leaks if caller returns early
}()

// Good: use ctx or close
go func() {
    select {
    case out <- compute():
    case <-ctx.Done():
    }
}()
```

**Loop variable capture (pre-Go 1.22)** — in Go 1.21 and earlier, loop variables are shared across iterations:

```go
// Bug in Go <=1.21
for _, v := range items {
    go func() { fmt.Println(v) }() // all goroutines print the last v
}

// Fix
for _, v := range items {
    v := v // shadow with a new variable
    go func() { fmt.Println(v) }()
}
```

Go 1.22 changed the semantics: each iteration gets its own variable. New code on 1.22+ does not need the workaround.

### nil interface != nil value

```go
func returnsError() error {
    var p *os.PathError = nil
    return p // returns a non-nil error interface wrapping a nil pointer
}

err := returnsError()
fmt.Println(err == nil) // false — interface has type, nil value
```

Return `nil` explicitly, never return a typed nil pointer as an interface.

**Map concurrency** — plain `map` is not safe for concurrent read+write. Use a mutex or `sync.Map`:

```go
var mu sync.RWMutex
var cache = map[string]string{}

func get(key string) string {
    mu.RLock()
    defer mu.RUnlock()
    return cache[key]
}

func set(key, val string) {
    mu.Lock()
    defer mu.Unlock()
    cache[key] = val
}
```

**defer in a loop** — deferred calls run at function exit, not loop iteration exit. For cleanup inside a loop, use a named function or an IIFE:

```go
for _, path := range paths {
    func() {
        f, _ := os.Open(path)
        defer f.Close() // closes at end of this IIFE, not the outer function
        process(f)
    }()
}
```

**Shadowed `err` across `:=`** — multiple `:=` in the same scope that include `err` reuse the existing variable only if at least one new variable is introduced. Verify with `go vet`.

**Slice aliasing** — `append` may or may not allocate. Two slices sharing an underlying array can silently overwrite each other:

```go
a := []int{1, 2, 3}
b := a[:2]
b = append(b, 99)
fmt.Println(a) // [1 2 99] — a[2] was overwritten
```

Use `a[lo:hi:hi]` (three-index slice) to cap capacity and force allocation on append.

---

## References

- [Effective Go](https://go.dev/doc/effective_go)
- [Structured Logging with slog](https://go.dev/blog/slog) (Go 1.21)
- [Range Over Function Types](https://go.dev/blog/range-functions) (Go 1.23)
- [An Introduction To Generics](https://go.dev/blog/intro-generics)
- [Generic Interfaces](https://go.dev/blog/generic-interfaces)
- [All your comparable types](https://go.dev/blog/comparable)
- [Go specification](https://go.dev/ref/spec)
- [log/slog package docs](https://pkg.go.dev/log/slog)
- [iter package docs](https://pkg.go.dev/iter)
- [slices package docs](https://pkg.go.dev/slices)
- [maps package docs](https://pkg.go.dev/maps)

## Part 2: Web Frameworks & Concurrency


## HTTP Middleware Patterns

Middleware wraps a handler to execute logic before and/or after it. The pattern is the same across all three frameworks and in stdlib.

### Chi / stdlib style

```go
func authMiddleware(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        token := r.Header.Get("Authorization")
        if !isValid(token) {
            http.Error(w, "unauthorized", http.StatusUnauthorized)
            return
        }
        // Store values with typed context keys, not strings
        ctx := context.WithValue(r.Context(), userIDKey, extractUserID(token))
        next.ServeHTTP(w, r.WithContext(ctx))
    })
}
```

### Gin style

```go
func authMiddleware() gin.HandlerFunc {
    return func(c *gin.Context) {
        token := c.GetHeader("Authorization")
        if !isValid(token) {
            c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "unauthorized"})
            return
        }
        c.Set("userID", extractUserID(token))
        c.Next()
    }
}
```

Use `c.Abort()` in Gin middleware to stop the chain. Calling `c.Next()` is optional at the end but required if you need to run logic *after* the downstream handler.

---

## Goroutines and Channels

### Goroutine basics

A goroutine costs ~2 KB of stack (versus ~1 MB for an OS thread) and grows dynamically. The Go scheduler multiplexes goroutines onto OS threads, so millions can exist concurrently.

```go
go func() {
    result := heavyComputation()
    resultCh <- result
}()
```

The only safe ways to get a result out of a goroutine are channels and shared memory with explicit synchronization. Never access shared state from multiple goroutines without coordination.

### Channel fundamentals

```go
// Unbuffered: sender blocks until receiver is ready
ch := make(chan int)

// Buffered: sender blocks only when buffer is full
ch := make(chan int, 100)

// Directional types for function signatures
func producer(out chan<- int) { out <- 42 }
func consumer(in <-chan int)  { v := <-in }

// Closing signals "no more values"
close(ch)

// Range over channel—stops when channel closed
for v := range ch {
    process(v)
}
```

Closing a channel twice panics. Only the sender should close. Never close from the receiver side.

### Select statement

`select` is the primary tool for multiplexing channels and implementing timeouts.

```go
select {
case msg := <-dataCh:
    handle(msg)
case err := <-errCh:
    return err
case <-ctx.Done():
    return ctx.Err()
default:
    // Non-blocking: execute if no case is ready
}
```

When multiple cases are ready, Go picks one at random—this is deliberate and prevents starvation. Use the `default` case for non-blocking checks, but avoid it in tight loops (it becomes a busy-wait spin).

---

## Worker Pool Pattern

A worker pool bounds the number of goroutines processing a queue, preventing unbounded memory growth under load.

```go
func workerPool(ctx context.Context, jobs <-chan Job, numWorkers int) <-chan Result {
    results := make(chan Result, numWorkers)

    var wg sync.WaitGroup
    for i := 0; i < numWorkers; i++ {
        wg.Add(1)
        go func() {
            defer wg.Done()
            for {
                select {
                case job, ok := <-jobs:
                    if !ok {
                        return // jobs channel closed
                    }
                    results <- process(ctx, job)
                case <-ctx.Done():
                    return
                }
            }
        }()
    }

    // Close results after all workers finish
    go func() {
        wg.Wait()
        close(results)
    }()

    return results
}
```

Close `jobs` from the sender side to signal workers to drain and exit. The `ctx.Done()` case handles external cancellation.

### Semaphore for bounded concurrency (simpler alternative)

When you don't need a persistent pool, a buffered channel acts as a semaphore:

```go
sem := make(chan struct{}, maxConcurrent)

for _, item := range items {
    sem <- struct{}{}  // acquire slot
    go func(item Item) {
        defer func() { <-sem }()  // release slot
        process(item)
    }(item)
}
```

For variable-weight resources, use `golang.org/x/sync/semaphore` instead.

---

## Fan-Out / Fan-In

Fan-out distributes work across multiple goroutines. Fan-in collects their results into a single channel.

```go
// Fan-out: send same input to N goroutines
func fanOut(ctx context.Context, input <-chan Item, n int) []<-chan Result {
    outputs := make([]<-chan Result, n)
    for i := 0; i < n; i++ {
        outputs[i] = worker(ctx, input)
    }
    return outputs
}

// Fan-in: merge N channels into one
func fanIn(ctx context.Context, channels ...<-chan Result) <-chan Result {
    merged := make(chan Result)
    var wg sync.WaitGroup

    forward := func(ch <-chan Result) {
        defer wg.Done()
        for {
            select {
            case v, ok := <-ch:
                if !ok {
                    return
                }
                merged <- v
            case <-ctx.Done():
                return
            }
        }
    }

    wg.Add(len(channels))
    for _, ch := range channels {
        go forward(ch)
    }

    go func() {
        wg.Wait()
        close(merged)
    }()

    return merged
}
```

### errgroup for structured fan-out

`golang.org/x/sync/errgroup` is the standard library for coordinating goroutines that return errors. The first error cancels all others via a derived context.

```go
g, ctx := errgroup.WithContext(ctx)

g.Go(func() error { return fetchUser(ctx, userID) })
g.Go(func() error { return fetchOrders(ctx, userID) })
g.Go(func() error { return fetchInventory(ctx) })

if err := g.Wait(); err != nil {
    return fmt.Errorf("parallel fetch: %w", err)
}
```

Use `errgroup.WithContext` over `errgroup.Group` whenever the goroutines accept a context—cancellation propagates automatically on first failure.

---

## sync Primitives

### Mutex

```go
type SafeCounter struct {
    mu    sync.Mutex
    value int
}

func (c *SafeCounter) Inc() {
    c.mu.Lock()
    defer c.mu.Unlock()
    c.value++
}
```

Use `sync.RWMutex` when reads dominate: `RLock`/`RUnlock` for reads, `Lock`/`Unlock` for writes.

### WaitGroup

```go
var wg sync.WaitGroup
for _, task := range tasks {
    wg.Add(1)
    go func(t Task) {
        defer wg.Done()
        run(t)
    }(task)
}
wg.Wait()
```

Always call `wg.Add` before launching the goroutine, not inside it—there is a race if the goroutine starts and calls `Done` before `Add` registers.

### Once

`sync.Once` guarantees a function runs exactly once across concurrent callers, regardless of how many reach it simultaneously. Use for lazy initialization of shared resources.

```go
var (
    instance *DB
    once     sync.Once
)

func GetDB() *DB {
    once.Do(func() {
        instance = connectDB()
    })
    return instance
}
```

### atomic

For simple counters and flags without mutex overhead:

```go
var requestCount atomic.Int64

func handler(w http.ResponseWriter, r *http.Request) {
    requestCount.Add(1)
    // ...
}
```

---

## Context Propagation and Cancellation

### The three constructors

```go
// Manual cancellation
ctx, cancel := context.WithCancel(parent)
defer cancel()

// Relative timeout
ctx, cancel := context.WithTimeout(parent, 5*time.Second)
defer cancel()

// Absolute deadline
ctx, cancel := context.WithDeadline(parent, time.Now().Add(5*time.Second))
defer cancel()
```

Always `defer cancel()` immediately after creation. Omitting it leaks the timer goroutine and the context resources until the parent is cancelled.

Child deadlines cannot extend a parent's deadline—`WithTimeout(parent, 10*time.Second)` when the parent already has a 3-second deadline gives the child 3 seconds, not 10.

### Checking cancellation

```go
func doWork(ctx context.Context) error {
    for {
        select {
        case <-ctx.Done():
            return ctx.Err() // context.Canceled or context.DeadlineExceeded
        default:
        }
        // ... one unit of work
    }
}
```

For blocking I/O, pass the context directly—all standard library I/O accepts it:

```go
rows, err := db.QueryContext(ctx, query, args...)
resp, err := http.NewRequestWithContext(ctx, "GET", url, nil)
```

### Context values: typed keys only

```go
// Unexported type prevents collisions with other packages
type ctxKey string

const (
    requestIDKey ctxKey = "request-id"
    userIDKey    ctxKey = "user-id"
)

// Set in middleware
ctx = context.WithValue(r.Context(), requestIDKey, id)

// Read in handler or service
id, ok := ctx.Value(requestIDKey).(string)
```

Store only request-scoped metadata—trace IDs, user IDs, request IDs. Never store dependencies (DB connections, loggers) in context; pass them as function arguments or struct fields.

### Go 1.20+ cancellation cause

```go
ctx, cancel := context.WithCancelCause(parent)
cancel(errors.New("quota exceeded"))

// Downstream:
if err := context.Cause(ctx); err != nil {
    log.Printf("cancelled because: %v", err)
}
```

### WithoutCancel (Go 1.21+)

For work that must complete even after the request context is cancelled (audit writes, committed payment records):

```go
auditCtx := context.WithoutCancel(r.Context())
go writeAuditLog(auditCtx, event)
```

---

## Graceful Shutdown

The pattern: stop accepting new requests, wait for in-flight requests to finish, cancel background work.

```go
func run() error {
    srv := &http.Server{
        Addr:    ":8080",
        Handler: buildRouter(),
    }

    // Background context for long-running workers
    ctx, cancel := context.WithCancel(context.Background())
    defer cancel()

    // Start background workers
    go backgroundWorker(ctx)

    // Listen for OS signals
    quit := make(chan os.Signal, 1)
    signal.Notify(quit, os.Interrupt, syscall.SIGTERM)

    // Start server in background
    serverErr := make(chan error, 1)
    go func() {
        if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
            serverErr <- err
        }
    }()

    select {
    case err := <-serverErr:
        return fmt.Errorf("server error: %w", err)
    case <-quit:
        log.Println("shutting down")
    }

    // Give in-flight requests up to 30 seconds to complete
    shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 30*time.Second)
    defer shutdownCancel()

    cancel() // Stop background workers

    return srv.Shutdown(shutdownCtx)
}
```

`srv.Shutdown` stops the listener, waits for active connections to close, then returns. It does not interrupt WebSocket or hijacked connections—handle those separately.

---

## Common Concurrency Bugs

**Goroutine leak** — A goroutine blocked on a channel that is never written to or closed. Detect with `go.uber.org/goleak` in tests. Monitor with `runtime.NumGoroutine()`.

**Closing a nil or already-closed channel** — Panics. Guard channel creation carefully; only close from the producer, once.

**Loop variable capture** — Pre-Go 1.22, loop variables were shared across goroutines:

```go
// Bug (Go < 1.22): all goroutines capture the same 'v'
for _, v := range items {
    go func() { process(v) }()
}

// Fix: pass as argument
for _, v := range items {
    go func(v Item) { process(v) }(v)
}
```

Go 1.22+ fixes this by giving each iteration its own copy.

**Data race on map** — `map` is not safe for concurrent reads and writes. Use `sync.RWMutex` or `sync.Map` for concurrent access.

**Context not propagated** — Passing `context.Background()` deep in a call stack instead of the request context loses cancellation and deadline propagation. Always thread the context through every layer.

**Forgetting `defer cancel()`** — Leaks the context's resources and any associated timer goroutine until the parent context is done.

---

## Sources

- [Best Go Backend Frameworks — Encore](https://encore.dev/articles/best-go-backend-frameworks)
- [Goroutines and Channels: Concurrency Patterns — DEV Community](https://dev.to/trapajim/goroutines-and-channels-concurrency-patterns-in-go-1dia)
- [Go Context in Depth — BackendBytes](https://backendbytes.com/articles/go-context-resilient-microservices/)
- [Gin vs Echo — Mattermost](https://mattermost.com/blog/choosing-a-go-framework-gin-vs-echo/)
- [Go Context package — pkg.go.dev](https://pkg.go.dev/context)
- [Canceling in-progress operations — go.dev](https://go.dev/doc/database/cancel-operations)

## Part 3: Testing Patterns


## Subtests: t.Run

`t.Run(name, func)` creates a named sub-scope. Key behaviors:

- `t.Fatal` inside a subtest stops only that subtest, not siblings.
- The parent blocks until all subtests finish (including parallel ones).
- Names form a hierarchy: `TestFoo/bar/baz`, selectable with `-run`.
- Spaces in names are converted to underscores in the selector; use `//` to match a literal `/` in a name (e.g., `America/New_York` → `-run=TestTime//New_York`).

Setup and teardown wrap naturally around the loop:

```go
func TestFoo(t *testing.T) {
    db := setupDB(t)       // runs once
    t.Run("insert", func(t *testing.T) { ... })
    t.Run("query", func(t *testing.T) { ... })
    db.Close()             // runs after all subtests complete
}
```

---

## Test Helpers and t.Helper()

Mark a function as a helper so failures report the caller's file/line, not the helper's internals.

```go
func assertNoError(t *testing.T, err error) {
    t.Helper()
    if err != nil {
        t.Fatalf("unexpected error: %v", err)
    }
}

func assertEqual[T comparable](t *testing.T, got, want T) {
    t.Helper()
    if got != want {
        t.Errorf("got %v; want %v", got, want)
    }
}
```

Without `t.Helper()`, stack frames inside the helper appear in the error output, obscuring which test line failed. Call it at the very top of every helper.

---

## Parallel Tests: t.Parallel()

Call `t.Parallel()` at the start of a subtest to let it run concurrently with other parallel tests. Always capture the loop variable first (before Go 1.22 loop-variable semantics apply automatically).

```go
for _, tc := range tests {
    tc := tc // capture — unnecessary in Go 1.22+, but harmless
    t.Run(tc.name, func(t *testing.T) {
        t.Parallel()
        // test body
    })
}
```

To run cleanup after all parallel subtests finish, nest the parallel group:

```go
func TestAll(t *testing.T) {
    resource := acquire(t)
    t.Run("group", func(t *testing.T) {
        t.Run("A", func(t *testing.T) { t.Parallel(); ... })
        t.Run("B", func(t *testing.T) { t.Parallel(); ... })
    })
    resource.Release() // runs after the nested group, i.e., after A and B
}
```

`-parallel N` controls the maximum concurrent tests; default is `GOMAXPROCS`.

---

## TestMain: Package-Level Setup and Teardown

Use `TestMain` only when you need global setup/teardown (e.g., starting a database container, setting environment variables). It runs in the main goroutine and must call `os.Exit(m.Run())`.

```go
func TestMain(m *testing.M) {
    flag.Parse()

    // global setup
    pool, resource := startContainer()

    code := m.Run()

    // global teardown
    pool.Purge(resource)
    os.Exit(code)
}
```

Do not skip `os.Exit`—without it the process exit code is always 0 regardless of test failures.

---

## HTTP Testing with httptest

### Testing a handler in-process (no network)

Use `httptest.NewRecorder` when the code under test accepts an `http.Handler` or when you call handler functions directly.

```go
func TestGreetHandler(t *testing.T) {
    req := httptest.NewRequest(http.MethodGet, "/greet?name=world", nil)
    rec := httptest.NewRecorder()

    GreetHandler(rec, req)

    res := rec.Result()
    defer res.Body.Close()

    if res.StatusCode != http.StatusOK {
        t.Errorf("status = %d; want %d", res.StatusCode, http.StatusOK)
    }
    body, _ := io.ReadAll(res.Body)
    if got := string(body); got != "Hello, world\n" {
        t.Errorf("body = %q; want %q", got, "Hello, world\n")
    }
}
```

### Testing a real HTTP client (full round-trip)

Use `httptest.NewServer` when the code under test makes outbound HTTP calls and you want to intercept them.

```go
func TestFetchUser(t *testing.T) {
    ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        w.Header().Set("Content-Type", "application/json")
        fmt.Fprintln(w, `{"id":1,"name":"Alice"}`)
    }))
    defer ts.Close()

    client := &UserClient{BaseURL: ts.URL}
    user, err := client.FetchUser(1)
    if err != nil {
        t.Fatalf("FetchUser: %v", err)
    }
    if user.Name != "Alice" {
        t.Errorf("name = %q; want Alice", user.Name)
    }
}
```

`httptest.NewTLSServer` starts a TLS server; use `ts.Client()` to get a pre-configured client that trusts the self-signed cert.

---

## Testify: assert vs require

`github.com/stretchr/testify` is the most widely used test-assertion library in Go.

| Package | Failure behavior |
| --------- | ----------------- |
| `assert` | Records failure, continues test execution |
| `require` | Calls `t.FailNow()`, stops the test immediately |

**Rule of thumb:** use `require` when subsequent assertions would panic or be meaningless if the current one fails (e.g., checking for a non-nil pointer before dereferencing it). Use `assert` for independent checks.

```go
import (
    "github.com/stretchr/testify/assert"
    "github.com/stretchr/testify/require"
)

func TestParse(t *testing.T) {
    result, err := Parse(input)
    require.NoError(t, err)          // stop immediately on error
    require.NotNil(t, result)        // stop if nil to avoid nil-deref below

    assert.Equal(t, "expected", result.Field)
    assert.Len(t, result.Items, 3)
}
```

Common assertions:

```go
assert.Equal(t, expected, actual)
assert.NoError(t, err)
assert.Error(t, err)
assert.ErrorIs(t, err, ErrNotFound)
assert.ErrorAs(t, err, &target)
assert.Contains(t, slice, element)
assert.ElementsMatch(t, listA, listB) // order-independent slice equality
assert.Eventually(t, func() bool { ... }, 5*time.Second, 100*time.Millisecond)
```

Use `assert.New(t)` to avoid repeating `t` on every call when making many assertions:

```go
a := assert.New(t)
a.Equal("alice", user.Name)
a.Equal(30, user.Age)
```

---

## Mocking with testify/mock

Define a mock by embedding `mock.Mock` and wiring each method to `Called`.

```go
type MockEmailSender struct {
    mock.Mock
}

func (m *MockEmailSender) Send(to, subject, body string) error {
    args := m.Called(to, subject, body)
    return args.Error(0)
}
```

Set up expectations in the test:

```go
func TestNotifyUser(t *testing.T) {
    sender := &MockEmailSender{}
    sender.On("Send",
        "alice@example.com",
        mock.AnythingOfType("string"), // subject can be anything
        mock.Anything,                 // body can be anything
    ).Return(nil).Once()

    svc := &NotificationService{Sender: sender}
    err := svc.NotifyUser("alice@example.com", "welcome")

    require.NoError(t, err)
    sender.AssertExpectations(t) // verifies all On calls were actually made
}
```

Key modifiers:

```go
.Once()         // must be called exactly once
.Times(n)       // must be called exactly n times
.Maybe()        // call is optional (won't fail AssertExpectations if not called)
.Run(func(args mock.Arguments) { ... }) // side-effect before returning
.After(time.Second)                     // simulate latency
```

Use `MatchedBy` for complex argument matching:

```go
sender.On("Send", mock.MatchedBy(func(req SendRequest) bool {
    return strings.HasSuffix(req.To, "@example.com")
})).Return(nil)
```

---

## Benchmarks

### Classic pattern (b.N)

```go
func BenchmarkJSON(b *testing.B) {
    data := loadFixture()
    b.ResetTimer() // exclude setup from measurement
    for range b.N {
        _ = json.Marshal(data)
    }
}
```

### Modern pattern (b.Loop, Go 1.24+)

`b.Loop()` manages setup/teardown exclusion automatically and runs the benchmark function only once per measurement:

```go
func BenchmarkJSON(b *testing.B) {
    data := loadFixture() // not timed
    for b.Loop() {
        _ = json.Marshal(data)
    }
    // cleanup here also not timed
}
```

Useful methods:

```go
b.ResetTimer()          // zero the timer after setup
b.ReportAllocs()        // enable malloc stats (same as -benchmem)
b.ReportMetric(n, "ns/op") // custom metric
```

Parallel benchmark:

```go
func BenchmarkHandler(b *testing.B) {
    b.RunParallel(func(pb *testing.PB) {
        for pb.Next() {
            callHandler()
        }
    })
}
```

Run benchmarks: `go test -bench=. -benchmem -benchtime=5s`.

Table-driven benchmarks with `b.Run`:

```go
func BenchmarkEncode(b *testing.B) {
    for _, size := range []int{64, 256, 1024} {
        b.Run(fmt.Sprintf("size=%d", size), func(b *testing.B) {
            data := make([]byte, size)
            for b.Loop() {
                Encode(data)
            }
        })
    }
}
```

---

## Fuzzing (Go 1.18+)

Fuzz tests use coverage-guided mutation to discover inputs that crash or produce wrong output.

```go
func FuzzParseDate(f *testing.F) {
    // Seed corpus: known-good or interesting inputs
    f.Add("2024-01-15")
    f.Add("1970-01-01")
    f.Add("")

    f.Fuzz(func(t *testing.T, s string) {
        // Must not panic; you can also assert invariants
        d, err := ParseDate(s)
        if err == nil {
            // Roundtrip invariant
            if got := d.Format(DateLayout); got != s {
                t.Errorf("roundtrip: got %q; want %q", got, s)
            }
        }
    })
}
```

Rules for fuzz targets:

- No persistent state between invocations.
- Must be deterministic for a given input.
- Supported argument types: `string`, `[]byte`, `bool`, integer types, float types.
- Failing inputs are written to `testdata/fuzz/FuzzName/` and become permanent regression tests.

Running:

```bash
# Run only the seed corpus (part of normal go test)
go test ./...

# Enable the fuzzing engine
go test -fuzz=FuzzParseDate -fuzztime=60s

# Reproduce a specific failing input
go test -run=FuzzParseDate/abc123
```

---

## Golden Files

Golden files store expected output on disk, making it easy to update snapshots and review diffs in version control.

```go
var update = flag.Bool("update", false, "update golden files")

func TestRenderTemplate(t *testing.T) {
    got, err := RenderTemplate("header", templateData)
    require.NoError(t, err)

    goldenPath := filepath.Join("testdata", "TestRenderTemplate.golden")
    if *update {
        os.WriteFile(goldenPath, []byte(got), 0644)
        t.Logf("updated %s", goldenPath)
        return
    }

    want, err := os.ReadFile(goldenPath)
    require.NoError(t, err, "golden file missing; run with -update to create")

    assert.Equal(t, string(want), got)
}
```

Update golden files: `go test -run TestRenderTemplate -update`. Commit updated golden files with the code change so reviewers can see the diff.

---

## Temporary Files and Directories

Prefer `t.TempDir()` over manual `os.MkdirTemp`. The returned directory is automatically removed when the test finishes, even on failure.

```go
func TestWriteConfig(t *testing.T) {
    dir := t.TempDir()
    path := filepath.Join(dir, "config.json")

    err := WriteConfig(path, cfg)
    require.NoError(t, err)

    data, err := os.ReadFile(path)
    require.NoError(t, err)
    assert.JSONEq(t, expectedJSON, string(data))
}
```

---

## Cleanup: t.Cleanup

`t.Cleanup` registers a function to run when the test and all its subtests finish. Registered functions run in last-in-first-out order, mirroring `defer`.

```go
func setupDB(t *testing.T) *sql.DB {
    t.Helper()
    db, err := sql.Open("postgres", testDSN)
    require.NoError(t, err)
    t.Cleanup(func() { db.Close() })
    return db
}
```

This pattern lets helper functions own their cleanup without requiring callers to call a teardown function explicitly.

---

## Common Pitfalls

**Failing to call `AssertExpectations`** — mock expectations are silently ignored without it.

**Not capturing the range variable** in pre-Go-1.22 code — all parallel subtests see the last value of `tc`.

**Calling `t.Parallel()` after doing I/O** — parallel state should be set before any test-specific work.

**Using `os.Exit` in test helpers** — always use `t.FailNow` or `t.Fatal`; `os.Exit` bypasses deferred cleanup and `t.Cleanup`.

**Omitting `t.Helper()`** — error lines point inside the helper instead of the test that called it.

**Shared mutable state in parallel tests** — each parallel subtest needs its own copy of any state it modifies.

## Part 4: gRPC with Go

