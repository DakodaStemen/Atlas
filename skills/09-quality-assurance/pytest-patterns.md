---
name: pytest-patterns
description: Deep-dive reference for pytest patterns covering fixture scopes, conftest.py organization, parametrize, built-in markers, monkeypatch, async testing with pytest-asyncio, factory fixtures, and key plugins.
domain: testing
category: python
tags: [pytest, fixtures, parametrize, conftest, markers, Python, testing, async, monkeypatch, hypothesis]
triggers: [pytest, python test, fixture, conftest, parametrize, pytest-asyncio, monkeypatch, pytest-cov, pytest-mock, xfail, skipif, tmp_path, capsys]
---

# pytest Patterns

## Fixture Scopes

Fixtures are created and destroyed at a scope boundary. The default is `function`; broaden scope only when setup is genuinely expensive and shared state won't pollute tests.

| Scope | Created | Destroyed |
| ----- | ------- | --------- |
| `function` (default) | before each test | after each test |
| `class` | before the first test in the class | after the last test in the class |
| `module` | before the first test in the module | after the last test in the module |
| `package` | before the first test in the package | after the last test in the package |
| `session` | once per entire test run | at the end of the session |

```python
# Expensive DB connection shared across an entire module
@pytest.fixture(scope="module")
def db_connection():
    conn = create_connection(DB_URL)
    yield conn
    conn.close()

# Heavy container; keep alive for the session unless --no-containers flag is set
def determine_scope(fixture_name, config):
    if config.getoption("--no-containers", default=False):
        return "function"
    return "session"

@pytest.fixture(scope=determine_scope)
def redis_container():
    container = start_redis_container()
    yield container
    container.stop()
```

Key rule: a fixture can only depend on fixtures of equal or wider scope. A `session`-scoped fixture cannot request a `function`-scoped one.

Fixtures at higher scope run first. Within the same scope, execution order follows declaration order in the test signature.

---

## Fixture Teardown

Prefer `yield` fixtures over `request.addfinalizer`. The code after `yield` is the teardown and always runs even if the test fails.

```python
@pytest.fixture
def temp_user(db):
    user = db.create_user(name="tmp")
    yield user
    db.delete_user(user.id)   # runs regardless of test outcome
```

Use `addfinalizer` only when you need to register cleanup dynamically (e.g., inside a loop):

```python
@pytest.fixture
def managed_resources(request):
    resources = []
    def cleanup():
        for r in reversed(resources):
            r.release()
    request.addfinalizer(cleanup)
    return resources
```

Finalizers run in LIFO order. Yield fixtures unwind in reverse dependency order automatically.

---

## conftest.py Organization

`conftest.py` files are automatically loaded by pytest; fixtures defined there are available to every test in the same directory and all subdirectories. No import is needed.

Recommended layout for a medium-sized project:

```text
tests/
    conftest.py            # session/module fixtures shared by all tests
    unit/
        conftest.py        # fixtures relevant only to unit tests
        test_services.py
    integration/
        conftest.py        # DB, HTTP client fixtures
        test_api.py
```

Rules:

- Fixtures defined in a child `conftest.py` shadow same-named fixtures from parent conftest files (useful for overriding).
- Keep `conftest.py` fixture-only. Do not put test logic or helper utilities there; put those in a `tests/helpers/` module.
- Group fixtures by lifetime: put session/module fixtures at the top of `conftest.py`, function-scoped ones below.

Example root `conftest.py`:

```python
import pytest
from myapp import create_app, db as _db

@pytest.fixture(scope="session")
def app():
    app = create_app({"TESTING": True, "SQLALCHEMY_DATABASE_URI": "sqlite://"})
    with app.app_context():
        _db.create_all()
        yield app
        _db.drop_all()

@pytest.fixture(scope="function")
def client(app):
    return app.test_client()
```

---

## parametrize: Table-Driven Tests

### Basic usage

```python
@pytest.mark.parametrize("x, expected", [
    (2, 4),
    (3, 9),
    (4, 16),
])
def test_square(x, expected):
    assert x ** 2 == expected
```

### Custom IDs for readability

```python
@pytest.mark.parametrize("value, label", [
    pytest.param(0, "zero", id="zero-input"),
    pytest.param(-1, "negative", id="negative-input"),
    pytest.param(100, "large", id="large-input"),
])
def test_label(value, label):
    assert classify(value) == label
```

Without `id=`, pytest generates IDs from parameter values, which can be unreadable for complex objects.

### Stacking for Cartesian product

```python
@pytest.mark.parametrize("fmt", ["json", "yaml", "toml"])
@pytest.mark.parametrize("mode", ["read", "write"])
def test_io(fmt, mode):
    # runs 6 times: all (mode, fmt) combinations
    ...
```

### Marking individual cases

```python
@pytest.mark.parametrize("n, expected", [
    (1, 2),
    pytest.param(9, 10, marks=pytest.mark.slow),
    pytest.param(99, 100, marks=pytest.mark.xfail(reason="known off-by-one")),
])
def test_increment(n, expected):
    assert increment(n) == expected
```

### Fixture parametrization (run all tests N times with each config)

```python
@pytest.fixture(params=["sqlite", "postgres"])
def database(request):
    db = connect(request.param)
    yield db
    db.close()

def test_query(database):
    # runs twice: once with sqlite, once with postgres
    assert database.execute("SELECT 1") == [(1,)]
```

### Indirect parametrize (pass args into a fixture from a test)

```python
@pytest.fixture
def user(request):
    role = request.param
    return create_user(role=role)

@pytest.mark.parametrize("user", ["admin", "viewer"], indirect=True)
def test_access(user):
    assert user.can_login()
```

### Dynamic parametrization via pytest_generate_tests

```python
# conftest.py
def pytest_generate_tests(metafunc):
    if "env_name" in metafunc.fixturenames:
        envs = metafunc.config.getoption("--envs", default="").split(",")
        metafunc.parametrize("env_name", [e for e in envs if e])
```

### Retrieving a fixture by name at runtime

When you need to use a fixture as a parametrized value, pass its name as a string and retrieve it via `request.getfixturevalue`:

```python
@pytest.fixture
def admin_user():
    return User(role="admin")

@pytest.fixture
def guest_user():
    return User(role="guest")

@pytest.mark.parametrize("fixture_name, can_delete", [
    ("admin_user", True),
    ("guest_user", False),
])
def test_delete_permission(fixture_name, can_delete, request):
    user = request.getfixturevalue(fixture_name)
    assert user.can_delete() is can_delete
```

---

## Built-in Markers

### Skip

```python
@pytest.mark.skip(reason="feature not implemented yet")
def test_new_feature():
    ...

@pytest.mark.skipif(sys.platform == "win32", reason="POSIX only")
def test_symlinks():
    ...
```

Skip conditionally at runtime:

```python
def test_needs_network():
    if not is_network_available():
        pytest.skip("no network")
    ...
```

### xfail (expected failure)

```python
@pytest.mark.xfail(reason="upstream bug #1234")
def test_parser():
    assert parse("bad input") == {}

# strict=True turns an unexpected pass into a failure — useful for tracking regressions
@pytest.mark.xfail(strict=True, reason="should fail until v2.0")
def test_future_behavior():
    assert new_api() == "expected"
```

An xfail test that actually passes is reported as `XPASS`. With `strict=True` it becomes an error.

### usefixtures

```python
@pytest.mark.usefixtures("clean_db", "mock_smtp")
class TestRegistration:
    def test_register(self):
        ...
```

Equivalent to requesting the fixture in every method but without cluttering signatures when you don't need the fixture's return value.

### Custom markers

Register markers to avoid `PytestUnknownMarkWarning`:

```ini
# pytest.ini or pyproject.toml [tool.pytest.ini_options]
[pytest]
markers =
    slow: tests that take > 1 s
    integration: requires external services
    smoke: subset run on every deploy
```

```python
@pytest.mark.slow
@pytest.mark.integration
def test_bulk_import():
    ...
```

Filter at runtime:

```bash
pytest -m "slow and not integration"
pytest -m smoke
```

---

## monkeypatch

`monkeypatch` is a function-scoped built-in fixture. All patches are automatically reverted after the test, even on failure.

```python
# Patch an attribute/method
def test_home_dir(monkeypatch, tmp_path):
    monkeypatch.setattr(Path, "home", lambda: tmp_path)
    assert getssh() == tmp_path / ".ssh"

# Patch a dict entry
def test_db_dsn(monkeypatch):
    monkeypatch.setitem(config, "db_host", "localhost")
    assert build_dsn() == "postgresql://localhost/mydb"

# Patch environment variables
def test_log_level(monkeypatch):
    monkeypatch.setenv("LOG_LEVEL", "DEBUG")
    assert get_log_level() == "DEBUG"

def test_missing_env(monkeypatch):
    monkeypatch.delenv("API_KEY", raising=False)
    with pytest.raises(EnvironmentError):
        connect_to_api()

# Scoped patching — safe for patching stdlib
def test_partial_patch(monkeypatch):
    with monkeypatch.context() as m:
        m.setattr(functools, "partial", lambda *a, **kw: None)
        result = code_that_uses_partial()
    # functools.partial is restored here
```

For patching `requests` or `httpx`, prefer `pytest-mock`'s `mocker.patch` (below) since it integrates with `unittest.mock`'s call-count assertions.

---

## tmp_path and capsys

### tmp_path

Provides a `pathlib.Path` to a temporary directory unique per test. No cleanup needed.

```python
def test_write_config(tmp_path):
    config_file = tmp_path / "config.json"
    write_config(config_file, {"debug": True})
    assert json.loads(config_file.read_text()) == {"debug": True}
```

`tmp_path_factory` (session-scoped version) creates shared temp directories:

```python
@pytest.fixture(scope="session")
def shared_data_dir(tmp_path_factory):
    d = tmp_path_factory.mktemp("data")
    (d / "seed.csv").write_text("a,b\n1,2")
    return d
```

### capsys

Captures stdout/stderr written during a test:

```python
def test_cli_output(capsys):
    run_cli(["--version"])
    captured = capsys.readouterr()
    assert "1.2.3" in captured.out
    assert captured.err == ""
```

`capfd` is the file-descriptor level equivalent (captures output from C extensions and subprocesses).

---

## Factory Fixtures

Return a callable from the fixture when tests need multiple distinct instances:

```python
@pytest.fixture
def make_order():
    orders = []

    def _make(product, qty=1, status="pending"):
        order = Order.create(product=product, qty=qty, status=status)
        orders.append(order)
        return order

    yield _make

    for o in orders:
        o.delete()

def test_order_fulfillment(make_order):
    o1 = make_order("widget", qty=5)
    o2 = make_order("gadget", qty=2, status="paid")
    fulfill(o2)
    assert o2.status == "fulfilled"
    assert o1.status == "pending"
```

For generating objects with fake but realistic data, pair with `factory_boy`:

```python
import factory
from myapp.models import User

class UserFactory(factory.Factory):
    class Meta:
        model = User

    username = factory.Sequence(lambda n: f"user{n}")
    email = factory.LazyAttribute(lambda o: f"{o.username}@example.com")
    active = True

@pytest.fixture
def active_user():
    return UserFactory()

@pytest.fixture
def inactive_user():
    return UserFactory(active=False)
```

---

## pytest-asyncio: Async Tests

Install: `pip install pytest-asyncio`

### Auto mode (recommended for async-heavy projects)

```ini
# pytest.ini
[pytest]
asyncio_mode = auto
```

With `auto` mode, all `async def` test functions are collected as asyncio tests automatically. No decorator needed.

### Strict mode (explicit opt-in per test)

```python
import pytest

@pytest.mark.asyncio
async def test_fetch():
    result = await fetch_data()
    assert result["status"] == "ok"
```

Critical: without either `asyncio_mode = auto` or the `@pytest.mark.asyncio` decorator, an `async def` test will be collected and immediately pass without executing — it returns a coroutine object that is truthy.

### Async fixtures

```python
@pytest.fixture
async def async_client():
    async with AsyncClient(app=app, base_url="http://test") as client:
        yield client

async def test_get_user(async_client):
    resp = await async_client.get("/users/1")
    assert resp.status_code == 200
```

### AsyncMock

```python
from unittest.mock import AsyncMock, patch

async def test_send_email():
    with patch("myapp.email.send", new_callable=AsyncMock) as mock_send:
        mock_send.return_value = {"message_id": "abc"}
        result = await send_welcome_email("user@example.com")
        mock_send.assert_called_once_with(to="user@example.com", subject="Welcome")
        assert result["message_id"] == "abc"
```

`AsyncMock` is in `unittest.mock` since Python 3.8. For `side_effect`, pass an exception class or an async callable.

### Event loop scope

By default each test gets its own event loop (function scope). For shared async state:

```python
@pytest.fixture(scope="module")
async def shared_async_resource():
    resource = await create_resource()
    yield resource
    await resource.aclose()
```

pytest-asyncio creates a module-scoped event loop automatically when it sees a module-scoped async fixture.

---

## Hypothesis: Property-Based Testing

Install: `pip install hypothesis`

Hypothesis generates input data that satisfies declared strategies and shrinks failing examples to minimal reproducers.

```python
from hypothesis import given, settings, assume
from hypothesis import strategies as st

@given(st.lists(st.integers()))
def test_sort_idempotent(lst):
    assert sorted(sorted(lst)) == sorted(lst)

@given(st.text(min_size=1), st.integers(min_value=1, max_value=100))
def test_repeat(s, n):
    assert len(s * n) == len(s) * n
```

Combining with pytest parametrize:

```python
@pytest.mark.parametrize("encoding", ["utf-8", "latin-1"])
@given(st.text())
def test_roundtrip(encoding, text):
    assume(text.isprintable())
    assert text.encode(encoding, errors="replace").decode(encoding, errors="replace")
```

Use `@settings(max_examples=500)` to increase thoroughness on critical paths, `@settings(deadline=None)` when the code under test is slow.

---

## Key Plugins

### pytest-cov

Coverage reporting integrated into the pytest run.

```bash
pip install pytest-cov
pytest --cov=myapp --cov-report=term-missing --cov-report=html
```

```ini
# pytest.ini
[pytest]
addopts = --cov=myapp --cov-fail-under=80
```

Use `# pragma: no cover` on lines that cannot be meaningfully tested (e.g., `if __name__ == "__main__"`).

### pytest-mock

Thin wrapper around `unittest.mock` that exposes a `mocker` fixture.

```python
def test_api_call(mocker):
    mock_get = mocker.patch("myapp.client.requests.get")
    mock_get.return_value.json.return_value = {"ok": True}
    result = fetch_status()
    assert result is True
    mock_get.assert_called_once_with("https://api.example.com/status")
```

`mocker.patch` auto-reverts after the test. Prefer it over `monkeypatch.setattr` when you need Mock's assertion helpers (`assert_called_once_with`, `call_count`, etc.).

Async variant:

```python
def test_async_call(mocker):
    mock_fetch = mocker.AsyncMock(return_value={"data": []})
    mocker.patch("myapp.service.fetch", mock_fetch)
    ...
```

### pytest-xdist

Parallel test execution:

```bash
pip install pytest-xdist
pytest -n auto          # one worker per CPU core
pytest -n 4             # exactly 4 workers
```

Tests must be independent (no shared mutable state on disk or DB). Use `tmp_path` and per-test DB names to avoid worker collisions.

### pytest-randomly

Randomizes test execution order to surface order-dependent failures:

```bash
pip install pytest-randomly
pytest --randomly-seed=12345   # reproducible seed for CI
```

### pytest-timeout

Fails tests that exceed a time limit:

```bash
pip install pytest-timeout
pytest --timeout=30
```

```python
@pytest.mark.timeout(5)
def test_fast_operation():
    ...
```

---

## pyproject.toml Configuration

Consolidate pytest settings to avoid scattered ini files:

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
asyncio_mode = "auto"
addopts = [
    "--strict-markers",
    "--tb=short",
    "-q",
]
markers = [
    "slow: tests that take more than 1 second",
    "integration: requires external services",
    "smoke: fast subset for pre-deploy checks",
]
filterwarnings = [
    "error",                                  # treat all warnings as errors
    "ignore::DeprecationWarning:third_party",  # except known noisy deps
]
```

`--strict-markers` makes unregistered markers an error, preventing silent typos in `@pytest.mark.integrtion`.

---

## Patterns Summary

- **Scope fixtures correctly**: use `session` for containers and connections, `module` for read-only DB state, `function` for anything that mutates.
- **Yield over finalizer**: teardown code in `yield` fixtures is always clear and always runs.
- **Factory fixtures for multi-instance tests**: return a callable, collect created objects, clean up in the yield tail.
- **parametrize with `id=`**: unambiguous test names in CI output save debugging time.
- **`indirect=True` to push parametrize args into a fixture**: keeps complex setup out of the test body.
- **`request.getfixturevalue`**: allows fixture name as a string parameter — the cleanest way to parametrize which fixture is used.
- **Register all custom markers**: `--strict-markers` in addopts converts typos from silent passes to errors.
- **`asyncio_mode = auto`**: in async-heavy projects eliminates marker boilerplate and the silent-pass trap.
- **`mocker` over `monkeypatch` for mocks**: use monkeypatch for env vars, dicts, sys.path; use mocker when you need call assertions.
