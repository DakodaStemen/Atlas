---
name: software-design-patterns
description: SOLID principles, Gang of Four patterns, Clean Architecture, and Hexagonal Architecture — practical application with modern language examples.
domain: backend
category: architecture
tags: [SOLID, design-patterns, clean-architecture, hexagonal, GoF, dependency-inversion, open-closed, factory, observer, strategy]
triggers: SOLID principles, design patterns, clean architecture, dependency injection, single responsibility, open closed principle, hexagonal architecture, ports and adapters, repository pattern, factory pattern
---

# Software Design Patterns

## SOLID at a Glance

| Principle | One-liner | Classic violation |
| --- | --- | --- |
| **SRP** — Single Responsibility | One class, one reason to change | `User` class that saves to DB, sends email, and formats reports |
| **OCP** — Open/Closed | Open for extension, closed for modification | `if payment_type == "paypal" / elif payment_type == "stripe"` sprawl |
| **LSP** — Liskov Substitution | Subtypes must be drop-in replacements | `Square` overrides `Rectangle` setters and breaks `area()` contracts |
| **ISP** — Interface Segregation | Don't force clients to depend on methods they don't use | `Robot` implements `Workable` + `Eatable` but has no mouth |
| **DIP** — Dependency Inversion | Depend on abstractions, not concretes | `PasswordReminder.__init__` calls `MySQLConnection()` directly |

---

## SRP in Practice

One class, one reason to change. "Reason to change" maps to one actor or stakeholder.

### Violation — three actors, one class

```python
class Order:
    def calculate_total(self): ...       # finance team owns this
    def save(self, db): ...              # infrastructure team owns this
    def print_invoice(self): ...         # UX team owns this
```

#### Fixed — one class per concern

```python
class Order:
    def __init__(self, items: list[Item]):
        self.items = items

    def total(self) -> Decimal:
        return sum(item.price * item.qty for item in self.items)


class OrderRepository:
    def __init__(self, session: Session):
        self._session = session

    def save(self, order: Order) -> None:
        self._session.add(order)
        self._session.commit()


class InvoicePrinter:
    def print(self, order: Order) -> str:
        lines = [f"{i.name}: {i.price}" for i in order.items]
        return "\n".join(lines)
```

Practical heuristic: if you have to say "and" when describing what a class does, it likely violates SRP.

---

## OCP in Practice

Extension via interfaces and composition, never by editing existing logic.

### Violation — every new payment type modifies `process`

```python
def process_payment(payment_type: str, amount: Decimal):
    if payment_type == "credit_card":
        charge_card(amount)
    elif payment_type == "paypal":
        call_paypal_api(amount)
    # Adding crypto? Edit this function.
```

#### Fixed — Strategy pattern via polymorphism

```python
from abc import ABC, abstractmethod

class PaymentMethod(ABC):
    @abstractmethod
    def charge(self, amount: Decimal) -> None: ...

class CreditCard(PaymentMethod):
    def charge(self, amount: Decimal) -> None:
        # Stripe SDK call
        stripe.PaymentIntent.create(amount=int(amount * 100))

class PayPal(PaymentMethod):
    def charge(self, amount: Decimal) -> None:
        paypal_client.execute_payment(amount)

class CryptoPayment(PaymentMethod):           # added without touching anything above
    def charge(self, amount: Decimal) -> None:
        crypto_gateway.send(amount)

class Checkout:
    def __init__(self, method: PaymentMethod):
        self._method = method

    def complete(self, amount: Decimal) -> None:
        self._method.charge(amount)
```

---

## DIP in Practice

High-level policy must not import low-level detail. Both depend on a shared abstraction (interface / abstract class).

### Violation — use case is coupled to ORM

```python
class CreateUserUseCase:
    def execute(self, dto: CreateUserDTO) -> None:
        user = User(name=dto.name)
        db.session.add(user)      # Django ORM leaking into business logic
        db.session.commit()
```

#### Fixed — constructor injection + port abstraction

```python
# Port (abstraction — lives in the domain layer)
class UserRepository(Protocol):
    def save(self, user: User) -> None: ...

# Use case depends only on the port
class CreateUserUseCase:
    def __init__(self, repo: UserRepository):
        self._repo = repo

    def execute(self, dto: CreateUserDTO) -> None:
        user = User(name=dto.name, email=Email(dto.email))
        self._repo.save(user)

# Adapter (concrete — lives in the infrastructure layer)
class SqlAlchemyUserRepository:
    def __init__(self, session: Session):
        self._session = session

    def save(self, user: User) -> None:
        self._session.add(UserModel.from_domain(user))
        self._session.commit()

# Wired at the composition root (main.py / DI container)
repo = SqlAlchemyUserRepository(session)
use_case = CreateUserUseCase(repo)
```

This is also the ports-and-adapters pattern. The port is the interface; the adapter is the implementation.

---

## Creational Patterns

### Factory Method

Define an interface for creating an object, but let subclasses decide which class to instantiate. Use when you need to decouple object construction from usage and may have multiple variants.

```typescript
interface Notifier {
  send(message: string): Promise<void>;
}

class EmailNotifier implements Notifier {
  async send(message: string) {
    await smtp.send({ body: message });
  }
}

class SlackNotifier implements Notifier {
  async send(message: string) {
    await slackClient.post({ text: message });
  }
}

function createNotifier(channel: "email" | "slack"): Notifier {
  if (channel === "email") return new EmailNotifier();
  return new SlackNotifier();
}

// Caller never imports EmailNotifier or SlackNotifier directly.
const notifier = createNotifier(config.alertChannel);
await notifier.send("Deployment succeeded");
```

### Abstract Factory

Creates families of related objects. Use when you have multiple product variants that must be used together (e.g., light/dark theme components, platform-specific UI widgets).

```typescript
interface Button { render(): string; }
interface Input  { render(): string; }

interface UIFactory {
  createButton(): Button;
  createInput(): Input;
}

class MaterialFactory implements UIFactory {
  createButton() { return new MaterialButton(); }
  createInput()  { return new MaterialInput(); }
}

class FluentFactory implements UIFactory {
  createButton() { return new FluentButton(); }
  createInput()  { return new FluentInput(); }
}

function buildForm(factory: UIFactory) {
  const btn = factory.createButton();
  const inp = factory.createInput();
  return `${inp.render()} ${btn.render()}`;
}
```

### Builder

Separates construction of a complex object from its representation. Essential when an object requires many optional parameters or a specific build order.

```typescript
class QueryBuilder {
  private table = "";
  private conditions: string[] = [];
  private limitVal?: number;

  from(table: string): this {
    this.table = table;
    return this;
  }

  where(condition: string): this {
    this.conditions.push(condition);
    return this;
  }

  limit(n: number): this {
    this.limitVal = n;
    return this;
  }

  build(): string {
    let sql = `SELECT * FROM ${this.table}`;
    if (this.conditions.length) sql += ` WHERE ${this.conditions.join(" AND ")}`;
    if (this.limitVal !== undefined) sql += ` LIMIT ${this.limitVal}`;
    return sql;
  }
}

const query = new QueryBuilder()
  .from("orders")
  .where("status = 'pending'")
  .where("created_at > '2024-01-01'")
  .limit(50)
  .build();
```

### Singleton — anti-pattern warning

Singleton introduces global mutable state and makes testing hard. Prefer dependency injection of a single shared instance instead.

```python
# Bad — hidden global state, untestable
class Config:
    _instance = None

    @classmethod
    def get(cls):
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

# Better — create once, inject everywhere
config = Config.from_env()            # one instance
service = MyService(config=config)    # explicit dependency
```

If you truly need a singleton (e.g., connection pool), scope it at the composition root and inject it; never call `getInstance()` from inside business logic.

---

## Structural Patterns

### Adapter

Wraps an incompatible interface so it matches what the client expects. Common when integrating third-party libraries or legacy systems.

```python
# Target interface your application expects
class PaymentGateway(Protocol):
    def charge(self, amount_cents: int, currency: str) -> str: ...

# Legacy / third-party SDK with a different signature
class LegacyPayProcessor:
    def make_payment(self, dollars: float, curr_code: str) -> dict:
        ...

# Adapter bridges the gap
class LegacyPayAdapter:
    def __init__(self, processor: LegacyPayProcessor):
        self._p = processor

    def charge(self, amount_cents: int, currency: str) -> str:
        result = self._p.make_payment(amount_cents / 100, currency)
        return result["transaction_id"]
```

### Decorator

Adds behaviour at runtime by wrapping an object. Composable and respects OCP — each decorator adds one concern.

```python
class DataSource(Protocol):
    def read(self) -> bytes: ...
    def write(self, data: bytes) -> None: ...

class FileDataSource:
    def read(self) -> bytes: return Path("data.bin").read_bytes()
    def write(self, data: bytes) -> None: Path("data.bin").write_bytes(data)

class EncryptionDecorator:
    def __init__(self, source: DataSource):
        self._source = source

    def write(self, data: bytes) -> None:
        self._source.write(encrypt(data))

    def read(self) -> bytes:
        return decrypt(self._source.read())

class CompressionDecorator:
    def __init__(self, source: DataSource):
        self._source = source

    def write(self, data: bytes) -> None:
        self._source.write(compress(data))

    def read(self) -> bytes:
        return decompress(self._source.read())

# Composable — compression then encryption
source = CompressionDecorator(EncryptionDecorator(FileDataSource()))
source.write(b"sensitive data")
```

### Facade

Provides a single simplified entry point into a complex subsystem.

```typescript
// Complex internals
class VideoEncoder { encode(file: string): Buffer { ... } }
class ThumbnailGenerator { generate(buffer: Buffer): string { ... } }
class CDNUploader { upload(buffer: Buffer, thumb: string): string { ... } }

// Facade — callers interact with one surface
class VideoUploadService {
  private encoder = new VideoEncoder();
  private thumbGen = new ThumbnailGenerator();
  private cdn = new CDNUploader();

  async upload(filePath: string): Promise<string> {
    const encoded = this.encoder.encode(filePath);
    const thumb = this.thumbGen.generate(encoded);
    return this.cdn.upload(encoded, thumb);
  }
}
```

### Proxy

Controls access to another object. Use for lazy initialisation, access control, logging, or caching.

```typescript
interface ImageLoader {
  display(): void;
}

class RealImage implements ImageLoader {
  constructor(private path: string) {
    this.load(); // expensive I/O
  }
  private load() { console.log(`Loading ${this.path} from disk`); }
  display() { console.log(`Displaying ${this.path}`); }
}

// Virtual proxy — defers loading until first use
class ImageProxy implements ImageLoader {
  private real?: RealImage;
  constructor(private path: string) {}

  display() {
    if (!this.real) this.real = new RealImage(this.path);
    this.real.display();
  }
}
```

---

## Behavioral Patterns

### Strategy

Encapsulates a family of interchangeable algorithms behind a shared interface. Eliminates if/else or switch branching on algorithm variants.

```python
from typing import Protocol

class SortStrategy(Protocol):
    def sort(self, data: list[int]) -> list[int]: ...

class QuickSort:
    def sort(self, data: list[int]) -> list[int]:
        if len(data) <= 1:
            return data
        pivot = data[len(data) // 2]
        left  = [x for x in data if x < pivot]
        mid   = [x for x in data if x == pivot]
        right = [x for x in data if x > pivot]
        return self.sort(left) + mid + self.sort(right)

class TimSort:
    def sort(self, data: list[int]) -> list[int]:
        return sorted(data)   # Python's built-in Timsort

class DataProcessor:
    def __init__(self, strategy: SortStrategy):
        self._strategy = strategy

    def process(self, data: list[int]) -> list[int]:
        return self._strategy.sort(data)

# Swap at runtime based on data size
strategy = QuickSort() if len(data) < 10_000 else TimSort()
processor = DataProcessor(strategy)
```

### Observer

One-to-many dependency: when the subject changes, all observers are notified automatically. Foundation of event-driven systems.

```typescript
interface Observer<T> {
  update(event: T): void;
}

class EventBus<T> {
  private observers: Observer<T>[] = [];

  subscribe(observer: Observer<T>) {
    this.observers.push(observer);
  }

  unsubscribe(observer: Observer<T>) {
    this.observers = this.observers.filter(o => o !== observer);
  }

  publish(event: T) {
    this.observers.forEach(o => o.update(event));
  }
}

// Usage
type OrderEvent = { orderId: string; status: string };
const bus = new EventBus<OrderEvent>();

class EmailNotificationObserver implements Observer<OrderEvent> {
  update(event: OrderEvent) {
    sendEmail(`Order ${event.orderId} is now ${event.status}`);
  }
}

class AuditLogObserver implements Observer<OrderEvent> {
  update(event: OrderEvent) {
    auditLog.write(event);
  }
}

bus.subscribe(new EmailNotificationObserver());
bus.subscribe(new AuditLogObserver());
bus.publish({ orderId: "123", status: "shipped" });
```

### Command

Encapsulates a request as an object, enabling undo/redo, queuing, and logging of operations.

```typescript
interface Command {
  execute(): void;
  undo(): void;
}

class TextEditor {
  private content = "";

  insert(text: string, pos: number) {
    this.content = this.content.slice(0, pos) + text + this.content.slice(pos);
  }

  delete(pos: number, len: number) {
    this.content = this.content.slice(0, pos) + this.content.slice(pos + len);
  }

  get text() { return this.content; }
}

class InsertCommand implements Command {
  constructor(
    private editor: TextEditor,
    private text: string,
    private pos: number,
  ) {}

  execute() { this.editor.insert(this.text, this.pos); }
  undo()    { this.editor.delete(this.pos, this.text.length); }
}

class CommandHistory {
  private stack: Command[] = [];

  run(cmd: Command) {
    cmd.execute();
    this.stack.push(cmd);
  }

  undo() {
    this.stack.pop()?.undo();
  }
}
```

### Chain of Responsibility

Passes a request along a chain of handlers; each handler decides to process it or forward it. Used in middleware pipelines, approval workflows, logging level filtering.

```python
from __future__ import annotations
from abc import ABC, abstractmethod

class Handler(ABC):
    def __init__(self, successor: Handler | None = None):
        self._next = successor

    @abstractmethod
    def handle(self, request: dict) -> dict | None: ...

    def pass_on(self, request: dict) -> dict | None:
        return self._next.handle(request) if self._next else None

class AuthMiddleware(Handler):
    def handle(self, request: dict) -> dict | None:
        if not request.get("token"):
            return {"error": "Unauthorized", "status": 401}
        return self.pass_on(request)

class RateLimitMiddleware(Handler):
    def handle(self, request: dict) -> dict | None:
        if request.get("rate_exceeded"):
            return {"error": "Too Many Requests", "status": 429}
        return self.pass_on(request)

class RouteHandler(Handler):
    def handle(self, request: dict) -> dict | None:
        return {"body": f"Processed {request['path']}", "status": 200}

# Build the chain
pipeline = AuthMiddleware(RateLimitMiddleware(RouteHandler()))
response = pipeline.handle({"token": "abc123", "path": "/api/orders"})
```

---

## Clean Architecture

Layers are concentric rings. Dependencies only point inward — outer layers know about inner layers, never the reverse.

```text
┌──────────────────────────────────────┐
│  Frameworks & Drivers                │  (HTTP, DB, CLI, UI)
│  ┌────────────────────────────────┐  │
│  │  Interface Adapters            │  │  (Controllers, Presenters, Gateways)
│  │  ┌──────────────────────────┐  │  │
│  │  │  Application (Use Cases) │  │  │  (Orchestrate domain rules)
│  │  │  ┌────────────────────┐  │  │  │
│  │  │  │   Domain (Entities)│  │  │  │  (Pure business logic, no imports)
│  │  │  └────────────────────┘  │  │  │
│  │  └──────────────────────────┘  │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

**The dependency rule:** source code dependencies must only point inward. Nothing in an inner circle can know anything about an outer circle. This means the domain layer imports nothing from frameworks, ORMs, or HTTP libraries.

### In practice

```text
src/
  domain/
    entities/        # User, Order — plain classes, no framework imports
    repositories/    # Abstract interfaces (UserRepository, OrderRepository)
    services/        # Domain services (pure business logic)
  application/
    use_cases/       # CreateOrder, CancelOrder — orchestrate domain
    dtos/            # Input/output data structures
  infrastructure/
    persistence/     # SQLAlchemy / Prisma implementations of repositories
    http/            # FastAPI / Express route handlers
    messaging/       # Kafka / RabbitMQ adapters
  main.py            # Composition root — wires everything together
```

**Key constraint:** `application/` and `domain/` have zero `import` statements pointing to `infrastructure/`. The infrastructure layer imports from application and domain, not vice versa.

---

## Hexagonal Architecture

Also called Ports and Adapters. The application core defines ports (interfaces); adapters implement them. There are two kinds:

- **Primary (driving) adapters** — initiate actions against the application (HTTP controllers, CLI commands, test harnesses).
- **Secondary (driven) adapters** — called by the application to reach external systems (databases, message queues, payment APIs).

```bash
          [HTTP Controller]  [CLI]  [Test Harness]
                  │              │          │
          Primary Adapters (call the port)
                  │
         ┌────────▼─────────────────────┐
         │     Application Core         │
         │                              │
         │  Port: UserRepository        │
         │  Port: EmailSender           │
         │  Port: PaymentGateway        │
         └───────────┬──────────────────┘
                     │
          Secondary Adapters (implement the port)
                     │
         [SQLAlchemy]  [SendGrid]  [Stripe]
```

```python
# Port — defined in the application layer
class EmailSender(Protocol):
    def send(self, to: str, subject: str, body: str) -> None: ...

# Primary adapter — drives the app via HTTP
class OrderController:
    def __init__(self, create_order: CreateOrderUseCase):
        self._use_case = create_order

    def post(self, payload: dict) -> Response:
        order = self._use_case.execute(CreateOrderDTO(**payload))
        return Response(status=201, body=order.id)

# Secondary adapter — implements the port
class SendGridEmailSender:
    def send(self, to: str, subject: str, body: str) -> None:
        sendgrid_client.send(
            to=to, subject=subject, html_content=body
        )

# Test adapter — swap in tests without mocking frameworks
class FakeEmailSender:
    def __init__(self): self.sent: list[dict] = []
    def send(self, to: str, subject: str, body: str) -> None:
        self.sent.append({"to": to, "subject": subject})
```

---

## Repository Pattern

Abstracts data access behind a collection-like interface. Domain code treats the repository as an in-memory collection; it has no idea whether the backing store is PostgreSQL, Redis, or a flat file.

```typescript
// Port — domain layer
interface UserRepository {
  findById(id: string): Promise<User | null>;
  findByEmail(email: string): Promise<User | null>;
  save(user: User): Promise<void>;
  delete(id: string): Promise<void>;
}

// Adapter — infrastructure layer (Prisma)
class PrismaUserRepository implements UserRepository {
  constructor(private db: PrismaClient) {}

  async findById(id: string): Promise<User | null> {
    const row = await this.db.user.findUnique({ where: { id } });
    return row ? UserMapper.toDomain(row) : null;
  }

  async save(user: User): Promise<void> {
    await this.db.user.upsert({
      where: { id: user.id },
      update: UserMapper.toPersistence(user),
      create: UserMapper.toPersistence(user),
    });
  }

  async findByEmail(email: string): Promise<User | null> {
    const row = await this.db.user.findUnique({ where: { email } });
    return row ? UserMapper.toDomain(row) : null;
  }

  async delete(id: string): Promise<void> {
    await this.db.user.delete({ where: { id } });
  }
}

// In-memory adapter — use in unit tests, no DB required
class InMemoryUserRepository implements UserRepository {
  private store = new Map<string, User>();

  async findById(id: string) { return this.store.get(id) ?? null; }
  async findByEmail(email: string) {
    return [...this.store.values()].find(u => u.email === email) ?? null;
  }
  async save(user: User) { this.store.set(user.id, user); }
  async delete(id: string) { this.store.delete(id); }
}
```

---

## When Patterns Hurt

**Premature abstraction.** Wrapping a 5-line function in a Strategy + Factory + Repository because it "might change" creates more code to maintain than the original problem.

**YAGNI (You Aren't Gonna Need It).** Don't add an Abstract Factory for a system with one UI theme. Don't add a Command pattern for actions that will never need undo. Patterns solve specific recurring problems — apply them when the problem exists, not when it might.

**Over-layering.** Clean Architecture with six layers on a CRUD service that talks to one table. The dependency rule is the valuable part; the exact number of rings is not sacred.

**God Facade.** A facade that exposes every method of every subsystem is just a wrapper with extra steps. Facades should hide complexity, not just proxy it.

**Singleton global state.** The worst Singleton smell is calling `getInstance()` inside a constructor or business method. This makes the class impossible to test in isolation.

**Anemic domain model.** Pushing all logic into services and leaving domain entities as pure data bags (just getters/setters) defeats the purpose of DDD-style layering. Business rules belong on the entity.

```python
# Anemic — logic lives outside the entity
class Order:
    status: str

class OrderService:
    def cancel(self, order: Order):
        if order.status == "shipped":
            raise ValueError("Cannot cancel shipped order")
        order.status = "cancelled"

# Rich domain model — entity owns its own invariants
class Order:
    def cancel(self) -> None:
        if self.status == OrderStatus.SHIPPED:
            raise OrderAlreadyShippedError(self.id)
        self.status = OrderStatus.CANCELLED
        self._events.append(OrderCancelledEvent(self.id))
```

---

## Language Examples

### TypeScript — Strategy + Dependency Injection

```typescript
// Compression strategies
interface Compressor {
  compress(data: Buffer): Buffer;
}

class GzipCompressor implements Compressor {
  compress(data: Buffer): Buffer {
    return zlib.gzipSync(data);
  }
}

class BrotliCompressor implements Compressor {
  compress(data: Buffer): Buffer {
    return zlib.brotliCompressSync(data);
  }
}

// Service depends on abstraction, injected at construction
class FileStorage {
  constructor(
    private readonly bucket: StorageBucket,
    private readonly compressor: Compressor,
  ) {}

  async store(key: string, data: Buffer): Promise<void> {
    const compressed = this.compressor.compress(data);
    await this.bucket.put(key, compressed);
  }
}

// Composition root
const storage = new FileStorage(
  new S3Bucket(config.bucket),
  new BrotliCompressor(),
);
```

### Python — Observer with typed events

```python
from dataclasses import dataclass
from typing import Callable, Generic, TypeVar

T = TypeVar("T")

@dataclass
class UserRegistered:
    user_id: str
    email: str

class EventDispatcher(Generic[T]):
    def __init__(self):
        self._handlers: list[Callable[[T], None]] = []

    def register(self, handler: Callable[[T], None]) -> None:
        self._handlers.append(handler)

    def dispatch(self, event: T) -> None:
        for handler in self._handlers:
            handler(event)

# Wire up handlers
dispatcher: EventDispatcher[UserRegistered] = EventDispatcher()
dispatcher.register(lambda e: send_welcome_email(e.email))
dispatcher.register(lambda e: provision_free_tier(e.user_id))
dispatcher.register(lambda e: audit_log.record("user_registered", e.user_id))

# Dispatch from use case
dispatcher.dispatch(UserRegistered(user_id="u-123", email="alice@example.com"))
```

### Python — Repository + Clean Architecture wiring

```python
# main.py — composition root
from infrastructure.persistence import SqlAlchemyUserRepository
from infrastructure.email import SendGridEmailSender
from application.use_cases import RegisterUserUseCase
from infrastructure.http import create_app

def bootstrap() -> App:
    session = create_session(DATABASE_URL)
    repo = SqlAlchemyUserRepository(session)
    email_sender = SendGridEmailSender(api_key=SENDGRID_KEY)
    use_case = RegisterUserUseCase(repo, email_sender)
    return create_app(use_case)

app = bootstrap()
```

---

## Critical Rules / Gotchas

**DI containers vs manual wiring.** DI containers (FastAPI `Depends`, NestJS providers, Spring) are convenient but obscure the dependency graph. In smaller codebases or domain-heavy services, manual wiring at the composition root is clearer and easier to trace. Use containers when the graph is large and wiring becomes repetitive.

**Circular dependencies.** If module A imports from module B and B imports from A, you have a layering violation. Symptom: `ImportError: cannot import name X`. Fix: introduce a shared abstraction (interface) that both depend on, or move the shared type to a lower layer.

**Don't make interfaces for everything.** An interface with a single implementation that will never change adds noise. Extract an interface when you have multiple implementations (or need a test double). The "I" in ISP is about keeping interfaces focused, not about creating one for every class.

**Ports should be defined by the consumer.** The application layer defines the `UserRepository` port according to what it needs — not according to what SQLAlchemy can do. This is the key insight of DIP: the high-level policy dictates the shape of the abstraction.

**Avoid fat constructors.** If a constructor takes more than 3–4 dependencies, the class is doing too much (SRP violation). Extract a collaborator.

**Test at the use-case boundary.** With ports and adapters, unit-test use cases by injecting in-memory adapters. Integration-test adapters separately against real infrastructure. This gives fast tests for business logic and slower tests only where I/O is involved.

**Event sourcing vs Observer.** Observer is synchronous and in-process. For cross-service or durable event handling, use a message broker (Kafka, RabbitMQ) with explicit serialisation and at-least-once delivery semantics.

---

## References

- Robert C. Martin, *Clean Architecture* (2017)
- Erich Gamma, Richard Helm, Ralph Johnson, John Vlissides, *Design Patterns: Elements of Reusable Object-Oriented Software* (1994)
- Alistair Cockburn, "Hexagonal Architecture" — <https://alistair.cockburn.us/hexagonal-architecture/>
- Martin Fowler, *Patterns of Enterprise Application Architecture* (2002)
- DigitalOcean — SOLID Design Principles: <https://www.digitalocean.com/community/conceptual-articles/s-o-l-i-d-the-first-five-principles-of-object-oriented-design>
- DEV.to — SOLID and GoF crash course: <https://dev.to/burakboduroglu/solid-design-principles-and-design-patterns-crash-course-2d1c>
