---
name: wiremock-mockserver
description: This skill should be used when the user asks about mocking HTTP APIs in tests, setting up WireMock or MockServer stubs, simulating faults or delays, recording/playback of real API interactions, stateful mock behavior, Docker-based mock servers, or deciding between API mocking and real service containers. Triggers on "WireMock", "MockServer", "API mock", "stub server", "HTTP mock", "contract stub", "fault simulation", "record and replay".
domain: testing
category: api-mocking
tags: [WireMock, MockServer, API-mocking, stub, contract-testing, integration, Docker, Testcontainers, fault-simulation, response-templating]
triggers: WireMock, MockServer, API mock, stub server, HTTP mock, stub out, record and replay, fault simulation, integration test mock, contract stub
---

# WireMock and MockServer: API Mocking Patterns

Comprehensive patterns for mocking HTTP APIs in integration tests, covering WireMock (the dominant JVM/Docker option), MockServer (the other main Java contender), and decision criteria for when to mock versus use real services via Testcontainers.

---

## When to Use This Skill

- Setting up WireMock or MockServer stubs for integration or component tests.
- Simulating third-party APIs (payment gateways, identity providers, partner APIs) without hitting real endpoints.
- Recording real service interactions for later playback.
- Testing resilience to network faults, timeouts, and malformed responses.
- Running mock servers in Docker/CI environments or via Testcontainers.
- Deciding between API mocking and running real services in containers.
- Integrating with contract testing (Spring Cloud Contract, Pact stub runners).

---

## WireMock

### Modes of Operation

| Mode | Use case |
| --- | --- |
| Embedded (JUnit rule / extension) | Unit and integration tests within JVM process |
| Standalone JAR | Shared mock for local dev or cross-service testing |
| Docker (`wiremock/wiremock`) | CI pipelines, multi-service compose environments |
| Testcontainers module | Docker-backed but lifecycle-managed per test class |
| WireMock Cloud | SaaS mock management with team collaboration |

### Stub Setup

A stub pairs a request matcher with a response definition. Stubs can be defined in Java DSL, JSON files under `mappings/`, or posted to the `/__admin/mappings` endpoint at runtime.

#### Java DSL

```java
import static com.github.tomakehurst.wiremock.client.WireMock.*;

stubFor(get(urlPathEqualTo("/api/orders"))
    .withQueryParam("status", equalTo("pending"))
    .withHeader("Authorization", matching("Bearer .+"))
    .willReturn(aResponse()
        .withStatus(200)
        .withHeader("Content-Type", "application/json")
        .withBodyFile("responses/orders-pending.json")));
```

**JSON mapping file** (`mappings/orders-pending.json`):

```json
{
  "request": {
    "method": "GET",
    "urlPath": "/api/orders",
    "queryParameters": {
      "status": { "equalTo": "pending" }
    },
    "headers": {
      "Authorization": { "matches": "Bearer .+" }
    }
  },
  "response": {
    "status": 200,
    "headers": { "Content-Type": "application/json" },
    "bodyFileName": "responses/orders-pending.json"
  }
}
```

Body files live under `__files/`. The `bodyFileName` path is relative to that directory.

### Request Matching

WireMock matches on any combination of: HTTP method, URL/path, path parameters (RFC 6570 templates), query parameters, headers, cookies, and request body.

#### URL strategies

- `urlEqualTo("/path?q=val")` — exact path + query string
- `urlPathEqualTo("/path")` — path only; use separate `withQueryParam()` for params
- `urlMatching("/path/[a-z]+")` — regex on full URL
- `urlPathMatching("/path/[a-z]+")` — regex on path only
- `urlPathTemplate("/users/{id}")` with `.withPathParam("id", equalTo("42"))` — named path params

#### Body matchers

- `equalToJson(...)` — semantic JSON equality; ignores whitespace; supports JsonUnit placeholders (`${json-unit.ignore}`, `${json-unit.regex}[0-9]+`)
- `matchingJsonPath("$.order.id")` — JSONPath existence check
- `matchingJsonPath("$.order.total", greaterThan(0.0))` — JSONPath + sub-matcher
- `matchingJsonSchema(schemaJson)` — JSON Schema validation
- `equalToXml(...)` — semantic XML equality with XMLUnit placeholders
- `matchingXPath("//order/status/text()", equalTo("pending"))` — XPath + sub-matcher

#### Logical operators

```java
.withHeader("X-Feature", matching("[a-z]+").and(containing("beta")))
.withQueryParam("sort", matching("asc").or(matching("desc")))
.withHeader("X-Debug", not(absent()))
```

### Priority

When multiple stubs match the same request, WireMock selects the one added most recently by default. Set explicit priority with `.atPriority(n)` where 1 is highest. Use this to set a low-priority catch-all that returns 401 while specific stubs return real data:

```java
stubFor(any(anyUrl()).atPriority(10)
    .willReturn(unauthorized()));

stubFor(get(urlPathEqualTo("/api/users")).atPriority(1)
    .willReturn(ok().withBody("[]")));
```

### Response Definition

```java
aResponse()
    .withStatus(200)
    .withStatusMessage("OK")
    .withHeader("Content-Type", "application/json")
    .withBody("{\"id\": 1}")           // inline body
    .withBodyFile("data/user.json")   // file under __files/
    .withFixedDelay(300)              // milliseconds
```

Shorthand builders: `ok()`, `okJson(body)`, `noContent()`, `badRequest()`, `unauthorized()`, `serverError()`, `status(n)`.

### Response Templating

Enables dynamic responses using Handlebars. Must be activated either per-stub or globally.

#### Per-stub

```json
{
  "response": {
    "status": 200,
    "body": "{\"id\": \"{{request.pathSegments.[1]}}\", \"timestamp\": \"{{now format='yyyy-MM-dd'}}\"}",
    "transformers": ["response-template"]
  }
}
```

#### Global at startup (Java)

```java
WireMockServer wm = new WireMockServer(options().globalTemplating(true));
```

#### Global in Docker

```bash
docker run wiremock/wiremock:3.13.2 --global-response-templating
```

#### Key helpers

| Helper | Example |
| --- | --- |
| Request path segment | `{{request.pathSegments.[2]}}` |
| Query param | `{{request.query.userId}}` |
| Header value | `{{request.headers.X-Tenant-Id}}` |
| JSONPath from body | `{{jsonPath request.body '$.orderId'}}` |
| Current timestamp | `{{now format='epoch'}}` |
| Offset date | `{{now offset='7 days' format='yyyy-MM-dd'}}` |
| Random UUID-style | `{{randomValue length=36 type='UUID'}}` |
| Random integer | `{{randomInt lower=1000 upper=9999}}` |
| Pick from list | `{{pickRandom 'ACTIVE' 'PENDING' 'CLOSED'}}` |
| Conditional | `{{#if (contains request.path 'admin')}}ADMIN{{else}}USER{{/if}}` |
| Base64 encode | `{{base64 request.body}}` |

### Stateful Behavior (Scenarios)

Scenarios are state machines. The initial state is always `Scenario.STARTED`. Stubs declare which state they require and optionally which state to transition to after matching.

```java
// Step 1: initial GET returns empty list
stubFor(get(urlPathEqualTo("/todos"))
    .inScenario("todo-lifecycle")
    .whenScenarioStateIs(STARTED)
    .willReturn(okJson("[]")));

// Step 2: POST creates an item and advances state
stubFor(post(urlPathEqualTo("/todos"))
    .inScenario("todo-lifecycle")
    .whenScenarioStateIs(STARTED)
    .willSetStateTo("HAS_ITEMS")
    .willReturn(status(201)));

// Step 3: subsequent GET returns populated list
stubFor(get(urlPathEqualTo("/todos"))
    .inScenario("todo-lifecycle")
    .whenScenarioStateIs("HAS_ITEMS")
    .willReturn(okJson("[{\"id\":1,\"text\":\"Buy milk\"}]")));
```

Reset all scenarios between tests: `WireMock.resetAllScenarios()`. Set a specific state: `WireMock.setScenarioState("todo-lifecycle", "HAS_ITEMS")`.

### Recording and Playback

Recording proxies live traffic through WireMock and writes matching stub JSON files.

**Via Admin UI** (standalone mode): navigate to `http://localhost:8080/__admin/recorder`, enter the target base URL, click Record, exercise the API, click Stop. Stubs land in `mappings/`.

#### Via Java API

```java
WireMock.startRecording(recordSpec()
    .forTarget("https://api.stripe.com")
    .onlyRequestsMatching(getRequestedFor(urlPathMatching("/v1/charges.*")))
    .captureHeader("Authorization")
    .extractBinaryBodiesOver(10_240)  // bytes threshold → moves to __files
    .makeStubsPersistent(true));

// ... exercise the system under test ...

SnapshotRecordResult result = WireMock.stopRecording();
```

**Snapshotting** (retroactive): if WireMock was already proxying and journaling requests, call `WireMock.snapshotRecord()` to convert logged requests into stubs without re-running anything.

Use `transformerParameters` in the record spec to enable response templating on captured stubs, so replayed responses can still echo back request data.

### Fault Simulation

```java
// Hard timeout — connection reset immediately
stubFor(get("/flaky-service")
    .willReturn(aResponse().withFault(Fault.CONNECTION_RESET_BY_PEER)));

// Returns nothing at all
stubFor(get("/silent")
    .willReturn(aResponse().withFault(Fault.EMPTY_RESPONSE)));

// Sends OK header then garbage bytes
stubFor(get("/corrupt")
    .willReturn(aResponse().withFault(Fault.MALFORMED_RESPONSE_CHUNK)));

// Fixed delay (tests timeout handling)
stubFor(get("/slow")
    .willReturn(ok().withFixedDelay(5000)));

// Random lognormal delay — mimics real latency distribution
stubFor(get("/realistic-latency")
    .willReturn(ok().withLogNormalRandomDelay(90, 0.1)));

// Uniform random delay with bounds
stubFor(get("/jittery")
    .willReturn(ok().withUniformRandomDelay(100, 400)));

// Chunked dribble — sends body in N chunks over duration (ms)
stubFor(get("/streaming")
    .willReturn(ok()
        .withBody("Hello world!")
        .withChunkedDribbleDelay(5, 2000)));
```

Global delays apply to all unmatched responses: `WireMock.setGlobalFixedDelay(500)`.

### Request Verification

```java
// Exact count
verify(1, postRequestedFor(urlPathEqualTo("/payments"))
    .withRequestBody(matchingJsonPath("$.amount", equalTo("99.99"))));

// Range checks
verify(moreThan(0), getRequestedFor(urlPathEqualTo("/health")));
verify(lessThan(3), postRequestedFor(anyUrl()));
verify(exactly(0), deleteRequestedFor(anyUrl()));  // assert nothing was deleted

// Find all matching events
List<LoggedRequest> posts = findAll(postRequestedFor(urlPathMatching("/api/.*")));

// Unmatched requests (useful for detecting extra unexpected calls)
List<LoggedRequest> unmatched = WireMock.findUnmatchedRequests().getRequests();

// Near-miss debugging — why didn't a request match?
List<NearMiss> nearMisses = WireMock.findNearMissesForAllUnmatched();
// nearMisses tells you which stub almost matched and what differed
```

### Docker and Standalone

#### Directory layout on host

```text
project/
  wiremock/
    mappings/        ← stub JSON files
    __files/         ← response body files
    extensions/      ← custom extension JARs
```

#### Run standalone

```bash
docker run -it --rm \
  -p 8080:8080 \
  -v "$PWD/wiremock:/home/wiremock" \
  wiremock/wiremock:3.13.2 \
  --global-response-templating \
  --verbose
```

#### HTTPS

```bash
docker run -it --rm \
  -p 8443:8443 \
  wiremock/wiremock:3.13.2 \
  --https-port 8443
```

**Docker Compose** (multi-service local dev):

```yaml
services:
  wiremock:
    image: wiremock/wiremock:3.13.2
    ports:
      - "8080:8080"
    volumes:
      - ./wiremock/mappings:/home/wiremock/mappings
      - ./wiremock/__files:/home/wiremock/__files
    entrypoint: ["/docker-entrypoint.sh", "--global-response-templating", "--verbose"]
```

**Custom image** (bake stubs in for CI):

```dockerfile
FROM wiremock/wiremock:3.13.2
COPY mappings /home/wiremock/mappings
COPY __files /home/wiremock/__files
ENTRYPOINT ["/docker-entrypoint.sh", "--global-response-templating"]
```

Pass options via env var (v3.2.0-2+):

```bash
docker run -e WIREMOCK_OPTIONS='--global-response-templating --disable-gzip' wiremock/wiremock:3.13.2
```

### Testcontainers Integration

```xml
<dependency>
    <groupId>org.wiremock.integrations.testcontainers</groupId>
    <artifactId>wiremock-testcontainers-module</artifactId>
    <version>1.0-alpha-14</version>
    <scope>test</scope>
</dependency>
```

```java
@Testcontainers
class PaymentClientTest {

    @Container
    static WireMockContainer wiremock = new WireMockContainer("3.13.2")
        .withMapping("create-charge",
            PaymentClientTest.class,
            "wiremock/create-charge.json");  // resource on classpath

    @DynamicPropertySource
    static void configure(DynamicPropertyRegistry registry) {
        registry.add("payment.base-url", wiremock::getBaseUrl);
    }

    @Test
    void createsCharge() {
        // exercise system under test; WireMock container handles the stub
    }
}
```

Use a static `@Container` field (shared across all tests in the class) for speed. Use an instance field if each test must start with a clean stub set.

### WireMock Cloud

WireMock Cloud is the hosted SaaS layer on top of the open-source engine. It adds:

- Team-shared mock definitions with a web UI
- Mock API lifecycle management (versioning, environments)
- Automatic OAuth/API-key injection
- No infrastructure to run

For local dev and CI, the open-source Docker image covers all core features. Use WireMock Cloud when multiple teams share mocks across projects or when you want a shared mock registry without managing servers.

---

## MockServer

MockServer takes a similar approach — start a server, register expectations, verify calls — but uses a different API style and has first-class support for proxying and forwarding.

### Core Concept: Expectations

An expectation = request matcher + action (respond / forward / callback / drop) + optional timing controls.

```java
new MockServerClient("localhost", 1080)
    .when(
        request()
            .withMethod("POST")
            .withPath("/api/sessions")
            .withBody(json("{\"username\": \"alice\"}",
                          MatchType.ONLY_MATCHING_FIELDS)),
        Times.exactly(1)
    )
    .respond(
        response()
            .withStatusCode(200)
            .withCookie("sessionId", "abc123")
            .withBody("{\"token\": \"xyz\"}")
    );
```

`MatchType.ONLY_MATCHING_FIELDS` (partial JSON match) vs `MatchType.STRICT` (exact). Default is partial.

### Running MockServer

#### Via Testcontainers

```java
@Container
static MockServerContainer mockServer =
    new MockServerContainer(DockerImageName.parse("mockserver/mockserver:5.15.0"));
```

Add dependency: `org.testcontainers:mockserver`.

#### Standalone Docker

```bash
docker run -d --rm -p 1080:1080 mockserver/mockserver:5.15.0
```

**Spring Boot test:** use `@MockServerTest` or `MockServerExtension` (JUnit5) from the `mockserver-junit-jupiter` artifact.

### Request Matching in MockServer

Similar scope to WireMock: method, path, query params, headers, cookies, body. Key difference: MockServer uses Java regex syntax throughout (not WireMock's glob/Java hybrid). Multi-value headers/params default to `SUB_SET` mode (any matching value satisfies); use `KeysAndValuesMatchingKey` for strict all-values matching.

Body matchers: JSON (JsonUnit placeholders), XML (XMLUnit), XPath, JsonPath, plain text (exact/regex), form parameters.

### Forwarding and Proxying

MockServer can forward matching requests to a real backend, making it useful as a selective proxy during integration tests:

```java
mockServerClient
    .when(request().withPath("/api/external/.*"))
    .forward(forward()
        .withHost("real-api.example.com")
        .withPort(443)
        .withScheme(HttpForward.Scheme.HTTPS));
```

This is useful when you want to intercept only certain paths while letting others through to a real (or Testcontainers) service.

### Verification

```java
mockServerClient.verify(
    request().withMethod("DELETE").withPath("/api/cache"),
    VerificationTimes.exactly(1)
);

// Verify sequence (calls happened in order)
mockServerClient.verifyZeroInteractions();
mockServerClient.verify(
    request().withPath("/auth"),
    request().withPath("/data")
);
```

---

## WireMock vs MockServer: When to Choose

| Criterion | WireMock | MockServer |
| --- | --- | --- |
| Ecosystem | Larger, more tutorials and extensions | Smaller but solid |
| Stub definition | JSON files or Java DSL; files are portable | Java API primary; JSON via REST API |
| Response templating | Built-in Handlebars, rich helpers | Not built-in; requires custom callbacks |
| Stateful scenarios | First-class scenario state machines | No equivalent; requires custom callbacks |
| Recording/playback | Built-in recorder + snapshots | Built-in recording |
| Fault simulation | Rich (faults, delays, chunked dribble) | Basic (delays, drop connection) |
| Proxying/forwarding | Supported but secondary | First-class, selective forwarding |
| Spring Cloud Contract | Native integration (stub generator, stub runner) | Not integrated |
| Testcontainers module | Official certified module | Official Testcontainers module |
| Admin API | Full REST + web UI at `/__admin` | Full REST API |

**Choose WireMock** when: you need response templating, stateful scenarios, rich fault injection, Spring Cloud Contract integration, or portable JSON stub files checked into source.

**Choose MockServer** when: selective forwarding/proxying is central to your test design, or your team prefers a pure-Java-API style without JSON files.

---

## WireMock vs Testcontainers with Real Services

Both patterns test your application against an HTTP endpoint. The question is whether that endpoint is a stub or the real service.

| | WireMock stub | Real service in Testcontainers |
| --- | --- | --- |
| Setup cost | Low — write JSON | Higher — pull and start real image |
| Test speed | Fast | Slower (container startup) |
| Accuracy | Only as good as your stubs | Exercises real service behavior |
| Fault injection | Built-in | Hard; need to inject failures at infra level |
| Works without network | Yes | Requires image pull (first run) |
| Suitable for | Third-party/external APIs you don't control | Owned services (databases, message brokers, your own microservices) |

### Rule of thumb

- External APIs you don't control (Stripe, Twilio, GitHub): mock with WireMock; record real interactions periodically to keep stubs current.
- Your own downstream services or open-source infrastructure (Postgres, Redis, Kafka): use Testcontainers with real images; the fidelity is worth the cost.
- Both approaches are preferable to mocking HTTP client method calls, because HTTP-level mocking catches serialization errors, headers, status codes, and content negotiation that method-level mocks miss.

---

## Contract Testing Integration

### Spring Cloud Contract + WireMock

Spring Cloud Contract generates WireMock stub JARs from Groovy/YAML contract definitions on the provider side. Consumers declare the stub JAR as a test dependency and use Stub Runner to start WireMock automatically:

```java
@AutoConfigureStubRunner(
    stubsMode = StubRunnerProperties.StubsMode.LOCAL,
    ids = "com.example:payment-service:+:stubs:8080"
)
class OrderServiceTest { ... }
```

The stub runner boots WireMock on port 8080 with the generated mappings. Provider runs contract verification as a normal test suite. Consumer and provider are decoupled; CI enforces compatibility.

### Pact + WireMock (manual integration)

Pact itself does not generate WireMock stubs. If you use Pact for consumer contract generation and need WireMock for component tests, you write adapter code to translate Pact interactions into WireMock stubs, or run both: Pact for contract verification, WireMock for component test isolation. They solve slightly different problems and can coexist.

---

## Checklist

- [ ] Stubs cover all response codes the SUT must handle (200, 400, 404, 500, timeouts).
- [ ] Request matching is precise enough to catch wrong calls (don't over-use `anyUrl()`).
- [ ] Response templating is enabled only where needed; static responses are simpler to debug.
- [ ] Fault simulation tests included: at least one timeout test and one 5xx test per integration.
- [ ] Stub files are checked into source control alongside the tests that use them.
- [ ] Scenarios are reset between tests (`resetAllScenarios()` in `@BeforeEach`).
- [ ] `verify()` assertions confirm expected calls occurred (not just that responses came back).
- [ ] Near-miss logging enabled in CI (`--verbose`) to aid debugging when stubs don't match.
- [ ] For recording: re-record against real service when the API contract changes.
- [ ] For Spring Cloud Contract: provider verification runs in provider CI before publish.

---

## Constraints

- Do not use WireMock to replace contract tests when consumers and providers are in different teams — use Spring Cloud Contract or Pact so providers can verify against consumer-published contracts.
- Do not stub your own services in integration tests when a real Testcontainers image is available and affordable; stubs of your own code have poor fidelity.
- Avoid global catch-all stubs that return 200 for everything — they hide missing stub coverage.
- Do not share a single WireMock instance across parallel test classes without namespace isolation (separate ports or separate container instances per class).
