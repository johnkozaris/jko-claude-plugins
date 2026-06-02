# Runtimes: Long-Running Python Processes

Covers what a Python process **does**: workers that drain queues, ETLs that read files, CLIs that run for minutes, scripts that call other programs, daemons that watch a directory. Not Docker, not systemd unit files, not container infra. Hexagonal architecture, FastAPI patterns, and request-lifecycle thinking from the rest of this skill don't apply here.

| Concern | A backend has | A runtime has |
| --- | --- | --- |
| Unit of work | HTTP request | A file, a job, a tick of a loop |
| Lifetime | Per-request | Process-long (minutes to forever) |
| Concurrency | asyncio + dependency graph | multiprocessing, threads, or a plain loop |
| Failure mode | Return 5xx | Crash, get restarted |
| State | Stateless between requests | Often holds state in memory, checkpoints to disk |
| Death | Worker recycled gracefully | SIGTERM, sometimes SIGKILL |

The patterns that survive production:

## Process Model

### Startup ordering

```python
def main() -> int:
    args = parse_args()
    settings = Settings()             # fail fast if env is wrong
    setup_logging(settings.log_level)
    install_signal_handlers()
    with open_resources(settings) as resources:
        return run(resources, args)

if __name__ == "__main__":
    sys.exit(main())
```

Args first, env/config validation second, logging third, signal handlers fourth, resources fifth. A misconfigured run fails *before* opening files or network connections, so partial state doesn't leak.

### Signal handlers set a flag, do nothing else

Python signal handlers run between bytecodes, not in the C-level handler. You **cannot** safely acquire locks, call `logging`, or shut down inside a handler. Set a flag. Exit the loop. Clean up in normal code.

```python
import signal, threading

_shutdown = threading.Event()

def _on_signal(signum, frame):
    _shutdown.set()

signal.signal(signal.SIGTERM, _on_signal)
signal.signal(signal.SIGINT,  _on_signal)

while not _shutdown.is_set():
    process_one_item()
```

For async, use `loop.add_signal_handler` (Unix only):

```python
async def main() -> None:
    shutdown = asyncio.Event()
    loop = asyncio.get_running_loop()
    loop.add_signal_handler(signal.SIGTERM, shutdown.set)
    loop.add_signal_handler(signal.SIGINT,  shutdown.set)
    task = asyncio.create_task(work())
    await shutdown.wait()
    task.cancel()
    await asyncio.gather(task, return_exceptions=True)

asyncio.run(main())
```

### Exit codes that mean something

- `0` success
- `1` general runtime error
- `2` usage / config error (matches `argparse`)
- `128 + N` for "killed by signal N" (shell convention; `130` = SIGINT, `143` = SIGTERM)

Set the exit code with `sys.exit(N)` or return it from `main()`. Never exit `0` on a logical failure: monitoring will silently miss broken jobs.

### Cleanup priority

- **`try/finally`** is the only cleanup guaranteed when an exception fires. Always wrap resources.
- **`atexit`** runs only on normal interpreter exit. `SIGKILL`, `os._exit`, segfault skip it.
- **`weakref.finalize`** is the modern replacement for `__del__`: explicit, debuggable, won't suppress exceptions silently.
- **`__del__`**: never reliable. Don't use it.

### Restart belongs to the supervisor

Crash-loop backoff is systemd's or k8s's job. Your code's job is to **start cleanly after an unexpected kill**: no permanent lock files that aren't cleared on startup, no half-written outputs, no assumption the previous run finished.

## Subprocess: Calling Other Programs

### The pipe deadlock

When `Popen` is started with `stdout=PIPE` and `stderr=PIPE`, the OS allocates a fixed pipe buffer (typically 64 KB). Once the child writes more than that, it blocks on write. If you then call `proc.wait()` without draining the pipes, both sides block forever. This is the most common subprocess bug.

```python
# BAD: deadlocks when child writes >64 KB to stdout
proc = subprocess.Popen(["my-tool", "input"], stdout=PIPE, stderr=PIPE)
proc.wait()                      # never returns
out, err = proc.stdout.read(), proc.stderr.read()

# GOOD: communicate() reads pipes concurrently
proc = subprocess.Popen(["my-tool", "input"], stdout=PIPE, stderr=PIPE)
out, err = proc.communicate(timeout=300)

# BEST: subprocess.run() handles it all
result = subprocess.run(
    ["my-tool", "input"],
    capture_output=True, text=True, encoding="utf-8",
    timeout=300, check=True,
)
```

### Encoding: never trust the default

`text=True` uses `locale.getpreferredencoding(False)`, which on Windows is often `cp1252` and on Linux usually UTF-8. The same code produces different bytes on different machines. Always pass `encoding="utf-8"` explicitly.

```python
# BAD: encoding depends on the OS locale
result = subprocess.run(cmd, capture_output=True, text=True)

# GOOD: explicit and portable
result = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8")
```

### Shell injection

`shell=True` with anything user-controlled is a remote-code-execution primitive. The list form is safe because no shell parses the args.

```python
# RCE
subprocess.run(f"convert {user_filename} out.png", shell=True)

# Safe
subprocess.run(["convert", user_filename, "out.png"], check=True, timeout=60)
```

### Always set `timeout` and `check`

Every `subprocess.run` should set both. Without `timeout`, a hung subprocess freezes your process forever. Without `check`, you have to inspect `returncode` manually and most code forgets.

```python
result = subprocess.run(
    ["ffmpeg", "-i", str(input_path), str(output_path)],
    timeout=300, check=True, capture_output=True, text=True, encoding="utf-8",
)
```

### Timeouts don't kill the child

When `proc.communicate(timeout=N)` raises `TimeoutExpired`, the child is **still running**. You must kill it.

```python
try:
    out, err = proc.communicate(timeout=30)
except subprocess.TimeoutExpired:
    proc.kill()                          # send SIGKILL
    out, err = proc.communicate()         # collect what was buffered
    raise
```

### Kill the process group, not just the child

When you start a subprocess that spawns its own children (a shell script, ffmpeg, npm), `proc.kill()` only kills the direct child. The grandchildren leak.

```python
proc = subprocess.Popen([...], start_new_session=True)
try:
    proc.wait(timeout=300)
except subprocess.TimeoutExpired:
    os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
```

### Buffered child output hides progress

A child Python process prints to stdout, but you don't see it for minutes. That's stdio block-buffering when stdout isn't a terminal. Fix from the parent side by setting `PYTHONUNBUFFERED=1` in the child's env, or pass `-u` to Python.

```python
subprocess.Popen(
    ["python", "-u", "child.py"],         # or env={"PYTHONUNBUFFERED": "1", **os.environ}
    stdout=subprocess.PIPE,
)
```

### `CalledProcessError` loses stderr unless you captured it

```python
# BAD: logs returncode but not why it failed
try:
    subprocess.run(["my-tool"], check=True)
except CalledProcessError as e:
    log.error("failed", returncode=e.returncode)

# GOOD: capture stderr so the error message survives
try:
    subprocess.run(["my-tool"], check=True, capture_output=True, text=True)
except CalledProcessError as e:
    log.error("failed", returncode=e.returncode, stderr=e.stderr)
```

### Long-running children: `poll()` vs `wait()`

`wait()` blocks until the child exits. `poll()` returns `None` if still running, the exit code if done. Use `poll()` in a loop when you need to do other work while the child runs.

## Filesystem

### `os.replace()` is atomic on the same filesystem only

`os.replace` is the only atomic rename. **Across filesystems, `os.replace` raises `OSError` ([EXDEV])**; it does not fall back. That's actually what you want: failure is loud. `shutil.move` *does* fall back to copy-then-delete across filesystems, and that fallback is not atomic. The fix in both cases: create the temp file in the **same directory** as the destination.

```python
def atomic_write(dst: Path, data: bytes) -> None:
    # Same dir is required: rename across filesystems is NOT atomic
    fd, tmp = tempfile.mkstemp(dir=dst.parent, prefix=dst.name + ".", suffix=".tmp")
    try:
        with os.fdopen(fd, "wb") as f:
            f.write(data)
            f.flush()
            os.fsync(f.fileno())            # durability on crash
        os.replace(tmp, dst)                # atomic on same filesystem
    except Exception:
        os.unlink(tmp)
        raise
```

`shutil.move` falls back to copy-and-delete across filesystems and is **not** atomic.

### `flush()` ≠ `fsync()` ≠ durable

- `f.flush()` flushes Python's buffer to the kernel. The OS may still have it in page cache.
- `os.fsync(f.fileno())` tells the OS to flush to disk.
- For a truly durable atomic write you need fsync on the **file** AND on the **parent directory** (the rename entry lives in the directory and isn't durable until the directory is fsynced).

```python
with open(path, "wb") as f:
    f.write(data); f.flush(); os.fsync(f.fileno())

dir_fd = os.open(str(path.parent), os.O_RDONLY)
try:
    os.fsync(dir_fd)
finally:
    os.close(dir_fd)
```

For most apps, fsync on the file alone is acceptable. For database-like durability, fsync the directory too.

### NFS and remote filesystems break assumptions

Code that works on local disk silently misbehaves on NFS:
- `fcntl.flock` may not be respected by all NFS clients.
- File mtimes have coarser granularity.
- Rename atomicity depends on NFS server version.
- `os.fsync()` may return before the data is on the server's disk.

Treat NFS as an integration boundary. Test on the actual target filesystem.

### TOCTOU: don't `exists()` then `open()`

The file may be deleted between the check and the open. Just try the open and handle the exception (EAFP, "easier to ask forgiveness than permission").

```python
# BAD: race window
if path.exists():
    with open(path) as f: ...

# GOOD: atomic
try:
    with open(path) as f: ...
except FileNotFoundError:
    ...
```

### Symlinks: don't enable `followlinks=True` without cycle detection

`os.walk` does NOT follow symlinks by default (`followlinks=False`). That default is safe. Setting `followlinks=True` on a tree that contains a symlink to an ancestor will loop forever; if you must follow links, do your own cycle detection with `os.path.realpath`. `pathlib.Path.rglob` does follow symlinks; use `Path.is_symlink()` to filter them out manually if needed.

### File descriptor leaks

Always use `with`. `subprocess.Popen` closes inherited FDs by default (since 3.2 on POSIX, since 3.7 on Windows), but custom code that opens files in a loop without `with` will hit the `ulimit -n` ceiling within hours.

```python
# LEAK
def read_logs(paths):
    return [open(p) for p in paths]      # never closed

# CORRECT
def read_logs(paths):
    for p in paths:
        with open(p) as f:
            yield f.read()
```

`ulimit -n` defaults vary widely: traditional Linux is 1024 soft / 1M hard; systemd-managed services on modern distros are often 65536-524288. macOS still defaults to 256-2560. Don't assume; check `ulimit -n` on the target machine and detect FD leaks with `psutil.Process().num_fds()` over time or `lsof -p <pid> | wc -l`.

### `os.scandir` is faster than `os.listdir + stat`

`scandir` returns `DirEntry` objects whose `is_dir()`, `is_file()`, and `is_symlink()` come from the original directory read on most platforms (POSIX uses `d_type` when supported; Windows reads the attributes inline). That avoids a `stat()` syscall per entry, which is the dominant cost on directories with thousands of files. The original PEP 471 motivation cited measured speedups of 2-20× depending on filesystem and entry count.

```python
# SLOW: one extra syscall per entry
for name in os.listdir(d):
    p = os.path.join(d, name)
    if os.path.isfile(p):
        ...

# FAST: type comes from the readdir syscall
with os.scandir(d) as it:
    for entry in it:
        if entry.is_file():
            ...
```

Caveats: on some filesystems and NFS mounts, `d_type` is not available and `is_dir/is_file` will silently fall back to a `stat()` syscall. `DirEntry.stat()` is cached only the first time you call it. `os.walk` was rewritten on top of `scandir` in Python 3.5 and is already fast. `pathlib.Path.iterdir` is implemented on `scandir` in modern CPython.

### Glob performance: `**` is expensive

`Path.rglob("**/*.py")` walks the entire tree. `Path.glob("*.py")` only looks at one directory. Don't reach for `**` unless you actually need recursion.

### Stale PID files lie

A PID file says "process 12345 is running". But PID 12345 may have died and been reused by another unrelated process. Verify before trusting:

```python
import os, errno

def pid_is_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)               # signal 0 = "are you there"
    except OSError as e:
        return e.errno == errno.EPERM  # exists but we can't signal it
    return True
```

Better: use `fcntl.flock` on a lockfile and keep the FD open for the process lifetime. The lock releases automatically when the process dies.

### Path encoding on Linux

Linux filesystems are byte sequences. Most are UTF-8 but you'll encounter mojibake. `os.fsdecode` handles this safely; passing raw `bytes` paths into `Path` works too.

## Concurrency

Decision matrix for non-web Python:

| Workload | Use | Why |
| --- | --- | --- |
| I/O-bound, many concurrent connections | `asyncio` | Single thread, no GIL contention; native cancellation, timeouts |
| I/O-bound, library has no async API | `ThreadPoolExecutor` | GIL releases during blocking I/O |
| CPU-bound, separable tasks | `ProcessPoolExecutor` | True parallelism, one process per core |
| CPU-bound + shared state | `multiprocessing.Pool` with `Manager` | When you need an explicit shared dict/list |
| CPU-bound + shared numpy arrays | `multiprocessing.shared_memory` (3.8+) | Zero-copy across processes |
| CPU-bound, 3.14+ | `InterpreterPoolExecutor` (PEP 734) | True parallelism without process spawn cost |
| Free-threaded build available | `ThreadPoolExecutor` on `python3.14t` (PEP 779) | Only if your extensions support it; measure first |

### Pickling rules for `ProcessPoolExecutor`

Tasks are pickled and sent to workers. **Top-level functions only**. No lambdas, no closures, no instance methods on classes defined in a notebook.

Default start method changed in Python 3.14:
- **`forkserver`** on Linux/POSIX (was `fork` through 3.13). The change was made because `fork` is unsafe in any process that has started threads, which Python increasingly does internally.
- **`spawn`** on macOS (since 3.8) and Windows.

Workers re-import your module under both `forkserver` and `spawn`, so `if __name__ == "__main__":` is mandatory; module-level side effects run per worker. The "load a model in the parent, share via copy-on-write" trick only works if you explicitly set `mp.set_start_method("fork")`, which now requires a documented justification (and is unsafe in threaded code).

```python
def process_one(path: Path) -> Result:    # top-level
    ...

if __name__ == "__main__":
    with ProcessPoolExecutor() as pool:
        for result in pool.map(process_one, paths):
            ...
```

### Free-threaded Python for runtimes

PEP 779 made free-threaded Python officially supported in 3.14. For batch CPU workloads where you've measured GIL contention AND your dependencies have free-threading support, `python3.14t` + threading can be a real win. The single-threaded overhead from the fine-grained locking is roughly 3-8% on pyperformance benchmarks, plus extra per-object memory; both vary heavily by allocation pattern. Most runtimes don't see GIL contention because they're I/O-bound or already use multiprocessing. Don't switch by default. Measure. Check `py-free-threading.github.io/tracking/` for ecosystem support. Detect at runtime with `sys._is_gil_enabled()`.

### asyncio outside the web

Same primitives, no FastAPI scaffolding. `asyncio.run(main())` at the entry point, `asyncio.TaskGroup` for fan-out, `asyncio.timeout()` for bounded waits, `add_signal_handler` for shutdown. Don't reach for `nest_asyncio`.

### `asyncio.gather` does NOT cancel siblings

This is the most dangerous concurrency myth. With the default `return_exceptions=False`, when the first coroutine raises, `gather` propagates that exception to the caller immediately, but **sibling coroutines keep running**. They become orphaned background tasks consuming resources, possibly raising unobserved exceptions later. Quote from the docs: *"Other awaitables in the aws sequence won't be cancelled and will continue to run."*

`TaskGroup` is the structured-concurrency primitive that actually cancels siblings. Use it for anything where partial failure should abort the batch:

```python
# DANGEROUS: if a() raises, b() and c() keep running orphaned
results = await asyncio.gather(a(), b(), c())

# SAFE: TaskGroup cancels siblings, raises ExceptionGroup
async with asyncio.TaskGroup() as tg:
    ta = tg.create_task(a())
    tb = tg.create_task(b())
    tc = tg.create_task(c())
# results: ta.result(), tb.result(), tc.result()
```

Use `gather` only when you want to collect every result or every exception in one shot, paired with `return_exceptions=True`.

### `concurrent.futures` silent exceptions

A future that raises is fine, but if you never call `future.result()` or `future.exception()` the exception is silently swallowed. Iterate `as_completed(futures)` or call `.result()` on each.

## IPC and Large Data

### Queue vs Pipe vs SharedMemory

- **`multiprocessing.Queue`**: FIFO, pickles every message, slow for high-volume binary data. Use for job distribution.
- **`multiprocessing.Pipe`**: faster 1-to-1, no built-in sync for many readers.
- **`multiprocessing.shared_memory.SharedMemory`** (3.8+): zero-copy. Use for numpy arrays across processes. Call `close()` in every process that attaches and `unlink()` exactly once (in the creator) on POSIX, or the segment persists until reboot. Python 3.13+ adds a resource tracker process that auto-cleans leaked segments; older versions don't have this safety net. On Windows, `unlink()` is a no-op: segments are freed when the last handle closes.
- **`Manager.dict()` / `Manager.list()`**: convenient shared state via a proxy process. Much slower than the others; only for low-frequency shared config.

### `mmap` for files larger than RAM

```python
import mmap

with open("huge.bin", "rb") as f:
    with mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as mm:
        header = mm[:16]                  # OS pages in as you access
        ...
```

`mmap` lets you treat a file like a `bytes` object without loading it. For sequential reading, prefer streaming (`for line in f`). For random access into a large binary file, `mmap` wins.

### Tracing across `ProcessPoolExecutor`

`contextvars` propagate within a task but **not across `submit`**. Pass the trace ID as a function argument.

## Streaming and Large Payloads

### Iterate files, don't read them whole

```python
# WRONG: blows up on a 10 GB file
data = open("huge.csv").read()

# RIGHT: file objects are iterators
with open("huge.csv") as f:
    for line in f:
        ...
```

### CSV: open with `newline=""`

Otherwise on Windows you get blank rows because the CSV module and Python's universal newlines fight over `\r\n`.

```python
with open("data.csv", newline="", encoding="utf-8") as f:
    for row in csv.reader(f):
        ...
```

### `csv.field_size_limit` defaults to 128 KiB

Large fields raise `_csv.Error: field larger than field limit`. The exact default is 131072 bytes (128 KiB). Fix at startup if you process larger fields:

```python
import csv, sys
csv.field_size_limit(sys.maxsize)
```

### Pandas memory: `read_csv` loads everything by default

```python
# Whole file in RAM
df = pd.read_csv("huge.csv")

# Stream in chunks
for chunk in pd.read_csv("huge.csv", chunksize=10000):
    process(chunk)
```

### NDJSON edge cases

Trailing newlines, lines that are pure whitespace, BOM at the start. Defensive read:

```python
with open("data.ndjson", "r", encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        yield json.loads(line)
```

### Parquet: stream row groups

```python
import pyarrow.parquet as pq

pf = pq.ParquetFile("huge.parquet")
for batch in pf.iter_batches(batch_size=10000):
    df = batch.to_pandas()
    process(df)
```

### Generator pipelines vs accidental list materialization

```python
# Materializes the whole intermediate list
results = [transform(x) for x in huge_iter]

# Streams
results = (transform(x) for x in huge_iter)
for r in results:
    ...
```

`list(generator)` collapses the stream. `sum`, `max`, `min` over a generator are streaming. `len(generator)` doesn't work; that itself is a tell that someone materialized the iterator.

## CLI

### Pick the right framework, know the cost

- **`argparse`** (stdlib): simple flags, lowest startup cost.
- **`click`**: stable, mature, decorator-driven.
- **`typer`** (built on click): type-annotation-driven, pairs naturally with Pydantic.

`argparse` is fastest to import; `click` adds a measurable hit; `typer` adds more because it walks function annotations at import time. For a CLI invoked once an hour the difference doesn't matter. For one invoked thousands of times in a build pipeline, run `python -X importtime cli.py 2> imp.txt` and decide based on your actual numbers.

```python
import typer

app = typer.Typer()

@app.command()
def ingest(path: Path, dry_run: bool = False) -> None:
    ...

@app.command()
def reconcile(date: str) -> None:
    ...

if __name__ == "__main__":
    app()
```

### Business logic does not live in the CLI module

The CLI parses arguments, calls a function in `service.py`, prints results, sets the exit code. Put the actual work in `service.py` so it's importable from tests, notebooks, and other entry points.

### Startup time: lazy imports

The "20-line script takes 2 seconds to start" debug pattern:

```bash
python -X importtime myscript.py 2> imp.txt
# sort by cumulative time, look at the top
```

Heavy imports like `pandas`, `numpy`, `torch`, `pydantic` cost 100-500 ms each. If only one subcommand needs them, import them **inside the function**:

```python
@app.command()
def predict(input: Path) -> None:
    import torch                          # deferred until /predict is hit
    model = torch.load(MODEL_PATH)
    ...
```

PEP 810 lazy imports (accepted, shipping in 3.15) will make this less manual once available.

### Distribution: `[project.scripts]` + `uv tool install`

```toml
[project.scripts]
myjob = "myjob.cli:app"
```

`uv tool install myjob` installs it as a global command. For one-off scripts use **PEP 723 inline metadata**:

```python
# /// script
# requires-python = ">=3.12"
# dependencies = ["httpx", "rich"]
# ///
import httpx
...
```

Then `uv run script.py` and uv builds an ephemeral env.

## Scheduling

| You have | Use |
| --- | --- |
| One machine, one job, runs hourly | **systemd timer** (or cron if you must) |
| Many jobs, dependencies, retries, monitoring UI | **Prefect** or **Dagster** |

APScheduler is rarely the right answer in 2026; in-process scheduling has the wrong failure modes for production.

### Idempotency for scheduled jobs

Every scheduled job must answer: "if you ran me twice, would anything bad happen?" The safe answer is no.

- **Natural keys**: `INSERT ... ON CONFLICT DO NOTHING`.
- **Persistent checkpoints**: write the last processed ID/timestamp to a state file or DB row after each batch. On restart, resume from there.
- **Single-instance lock**: `fcntl.flock` on a lockfile FD held for the process lifetime.

### DST and midnight batch jobs

A job scheduled at "every day at midnight" in a timezone that observes DST will run twice on one day per year and zero times on another. Two fixes: schedule in UTC (cron and systemd both support this), or store the schedule in a DST-aware library that handles the transition explicitly.

## Long-running process memory and resources

### Reference cycles + C extensions

Pure-Python reference cycles get cleaned up by the cyclic GC. Cycles that hold C extension memory often don't (the C code may register custom finalizers that the GC can't run). The smoking gun: `gc.collect()` doesn't free the memory. Diagnose with `memray` (allocation tracing) or `tracemalloc` (allocation source).

### `__slots__` for high-volume objects

A `dict`-backed Python object costs ~150 bytes overhead. `__slots__` removes the `__dict__` and saves it. For an object you create millions of, that's hundreds of megabytes.

```python
@dataclass(slots=True)
class Trade:
    ts: float
    price: Decimal
    qty: int
```

### `lru_cache(maxsize=None)` is a slow leak

Unbounded cache + per-instance method = every instance ever called is kept alive forever. Set a `maxsize`, or use `cached_property` for per-instance cases (see `ai-slop.md` CODE-02).

### Logger handler accumulation

Calling `logging.getLogger("x")` returns the same logger object every time, but each `addHandler` call adds another handler. Configure logging **once** at startup (via `logging.config.dictConfig`); never `addHandler` from inside business code.

### Long-lived HTTP clients

A 2-week worker uptime exhausts the connection pool of an `httpx.Client` if you let it accumulate stale sockets. Use one client per process, opened at startup, closed at exit. Set explicit `Limits(max_connections=, max_keepalive_connections=)`. Set `timeout=` on every call.

### File descriptor exhaustion

`psutil.Process().num_fds()` monitored over time will reveal a leak. Common causes: opening files in a loop without `with`, sockets not closed, subprocess pipes left dangling.

## When Python is slow

### Quadratic loops

```python
# O(n^2): each {**out, **d} copies the whole dict
out = {}
for d in dicts:
    out = {**out, **d}                    # O(len(out)) per iteration

# O(n)
out = {}
for d in dicts:
    out.update(d)

# String concat: CPython has a fragile in-place optimization for `s += x` when
# the string has only one reference. As soon as it's stored in a list or closed
# over, the optimization fails silently and you get O(n^2) total. Just use join.
parts = []
for line in lines:
    parts.append(transform(line))
text = "".join(parts)
```

### `re.compile()` inside a hot loop

```python
# BAD: recompiles every call
def normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text)

# GOOD: compile once at module load
_WHITESPACE = re.compile(r"\s+")
def normalize(text: str) -> str:
    return _WHITESPACE.sub(" ", text)
```

Same applies to `datetime.strptime` format strings.

### `pickle` for IPC: slow and dangerous

`pickle` is fine for short-lived intra-process objects, but for cross-process or cross-machine data it's slow and unsafe (loading an untrusted pickle executes arbitrary code). For JSON, use `orjson`. For tabular data, `pyarrow`. For typed structs, `msgspec`.

### `json.loads` is slow on large payloads

`orjson.loads` is typically 2-10× faster than stdlib `json` on deserialisation, depending on payload shape (string-heavy JSON sees the most gain). `orjson.dumps` is 1.5-5× faster on serialisation. For decode-and-validate in one pass with a known schema, `msgspec.Struct` is faster still. Measure on your own data before assuming.

### `time.time()` for measuring duration

```python
# BAD: NTP can adjust the clock backwards mid-call
start = time.time()
do_thing()
duration = time.time() - start            # may be negative

# GOOD: monotonic clock, never goes backward
start = time.monotonic()
do_thing()
duration = time.monotonic() - start
```

### `key in list(d.keys())` is O(n)

```python
# O(n): silently builds a list and scans it
if key in list(d.keys()):
    ...

# O(1)
if key in d:
    ...
```

The first form looks weird written like that, but `list(...)` shows up in the wild often via `.keys()` returning a view that someone wraps "to be safe".

## asyncio / threading / GIL footguns

### `threading.Timer` keeps the process alive

```python
# Process won't exit because the Timer thread is non-daemon
threading.Timer(60, do_something).start()

# Mark it daemon so the interpreter can exit
t = threading.Timer(60, do_something); t.daemon = True; t.start()
```

### Forgetting to `await`

```python
# Schedules nothing; emits "coroutine was never awaited"
asyncio.create_task(coro)                  # GOOD
loop.call_later(5, coro)                   # BAD: passes coro, not callable
loop.call_later(5, lambda: asyncio.create_task(coro))  # GOOD
```

### Mixing threads and asyncio

`await asyncio.to_thread(sync_fn, ...)` is the bridge for calling a blocking function from an async context. `loop.run_in_executor` is the older spelling.

## Common production failures

### "Worked locally"

The usual suspects, by frequency:
- Missing env var (forgot to update the deploy config)
- Different default encoding (CP1252 on Windows dev box, UTF-8 in production)
- File path case sensitivity (macOS HFS+ vs Linux ext4)
- Different timezone (UTC in container, local on dev)
- Different `ulimit -n` (1024 on dev, 65535 in production, or vice versa)

### Logging that hides the exception

```python
# Loses the traceback
try:
    ...
except Exception as e:
    log.error("failed", error=str(e))

# Captures the traceback
try:
    ...
except Exception:
    log.exception("failed")            # exception() = error() with exc_info=True
```

structlog: `log.exception("failed")` or `log.error("failed", exc_info=True)`.

### Off-by-one in chunked iteration

```python
# CORRECT
for i in range(0, len(items), chunk_size):
    chunk = items[i : i + chunk_size]

# BAD: overlaps
chunk = items[i : i + chunk_size + 1]
```

### Sentinel collisions with `None`

When `None` is a valid value, `None` can't double as "not set".

```python
MISSING = object()

def get(d: dict, key: str, default=MISSING):
    if (v := d.get(key, MISSING)) is MISSING:
        return default
    return v
```

### Path traversal in user input

```python
def safe_read(base: Path, user_input: str) -> bytes:
    base = base.resolve()
    full = (base / user_input).resolve()             # resolve BOTH sides
    if not full.is_relative_to(base):
        raise ValueError("path traversal attempt")
    return full.read_bytes()
```

The pattern that fails: `(base / user_input).is_relative_to(base.resolve())` resolves only the base. A symlink `base/evil -> /etc` inside the candidate path will then pass the check while reading outside the base. Always resolve the candidate first. `Path.is_relative_to` is 3.9+; on older versions, `os.path.commonpath([base, full]) == str(base)` is the equivalent.

## Review Checklist: The Ten That Catch Most Bugs

When reviewing a Python runtime, run through these ten. They're the highest-signal items; ruff and bandit catch the others.

1. **`asyncio.gather` instead of `TaskGroup`.** `grep -rn "asyncio.gather" src/`. Siblings keep running after the first exception, becoming orphaned. Use `TaskGroup` when partial failure should abort the batch.
2. **`subprocess` without `timeout` and `check=True`.** `grep -rn "subprocess.run\|Popen" src/ | grep -v timeout`. A hung child with no timeout blocks the worker forever; missing `check` means a non-zero exit is silently ignored.
3. **`Popen.wait()` without draining pipes.** `grep -rn "proc.wait\|\.stdout.read\|\.stderr.read" src/`. Deadlocks once the child writes more than the pipe buffer (~64 KB). Use `communicate()` or `subprocess.run`.
4. **`shell=True` with anything dynamic.** `grep -rn "shell=True" src/`. RCE. Use the list form.
5. **`os.replace` or `shutil.move` across filesystems.** `grep -rn "os.replace\|shutil.move" src/`. Temp file must live in the destination's directory; otherwise `os.replace` raises and `shutil.move` silently goes non-atomic.
6. **`open()` without `with` (or without `encoding=`).** `grep -rn "= open(" src/ | grep -v "with "`. FD leak. Also: missing `encoding="utf-8"` is locale-dependent.
7. **`time.time()` used to measure elapsed time.** `grep -rn "time.time(" src/`. NTP can move the wall clock backward. Use `time.monotonic()` for durations.
8. **Unobserved futures.** `grep -rn "executor.submit\|pool.submit" src/`. Exceptions on a future you never `.result()` on are silently swallowed.
9. **`@lru_cache(maxsize=None)` on a method.** `grep -rn "lru_cache(maxsize=None)\|@cache" src/`. Unbounded cache + per-instance method keeps every instance alive forever.
10. **`is_relative_to` for path-traversal check, without resolving the candidate.** `grep -rn "is_relative_to" src/`. Pattern must be `candidate.resolve().is_relative_to(base.resolve())`. Symlinks in the candidate escape otherwise.

Anything else (`time` → `monotonic`, `mutable default args`, `bare except`, `assert` for prod, `requests` in `async def`, Pydantic v1 ghosts, `datetime.utcnow`, etc.) is **ruff's job**. Configure ruff with `B`, `S`, `DTZ`, `UP`, `SIM`, `RUF`, and let it catch them every commit. The 10 above are things ruff can't see for you.

## The three most common runtimes

If you're not sure which you're building, this is the breakdown:

**1. Worker that drains a queue / processes a directory.** Single-process script. `signal.signal` shutdown flag, single-instance file lock, atomic writes, checkpoint progress to disk. The whole thing is ~150 lines.

**2. Batch ETL that runs nightly.** `typer` CLI with a `run` subcommand. Read-stream-transform-write generator pipeline. Idempotent via natural key or last-processed-timestamp.

**3. CPU-bound batch processor.** `ProcessPoolExecutor` with `pool.map` over a top-level function. `if __name__ == "__main__":` guard mandatory. Workers don't share state. Pass trace IDs as function arguments because contextvars don't cross process boundaries.

Everything else is a variation. Resist hexagonal, repository pattern, or DI containers in a 200-line worker. The runtime is a function that processes things in a loop until told to stop.
