---
name: orm-database-access
description: ORM and database access patterns covering general ORM patterns (Active Record, Data Mapper, Repository, Unit of Work, query builders) and Prisma/Drizzle ORM (TypeScript ORMs, schema definition, migrations, relations, performance). Use when choosing or implementing database access layers.
domain: data-engineering
tags: [orm, prisma, drizzle, active-record, data-mapper, repository, query-builder, typescript-orm]
triggers: ORM, prisma, drizzle, active record, data mapper, repository pattern, query builder, database access layer, typeorm
---


# ORM Patterns

## ORM Selection Guide

The right ORM depends on three axes: type-safety story, performance envelope, and migration maturity.

| ORM | Language | Type Safety | Bundle Size | Migration Tooling | Best Fit |
| ----- | ---------- | ------------- | ------------- | ------------------- | ---------- |
| **Drizzle** | TypeScript | Inferred (real-time) | ~50 KB | drizzle-kit, robust | Serverless, SQL-savvy teams |
| **Prisma** | TypeScript | Generated (.d.ts) | ~2–5 MB | prisma migrate, mature | Teams wanting high abstraction, broad DB support |
| **TypeORM** | TypeScript | Decorator-based, weak on relations | Medium | Buggy; schema drift common | Java/C# migrations, NestJS legacy |
| **SQLAlchemy** | Python | 2.0: typed stubs + mapped_column | N/A | Alembic (separate) | Python backends, FastAPI, complex queries |
| **GORM** | Go | Struct-tag based | Small | External (golang-migrate, goose) | Go services, simple-to-medium complexity |

**Type-safety tiebreaker:** Prisma pre-computes types at generate time (~300 type instantiations). Drizzle infers on every query (~40k instantiations in 1.x, ~5k in the 1.0 beta). Result: Prisma tsc checks run ~2–3× faster on large schemas. For query construction Drizzle's RQB is now faster per query than Prisma's client.

**Performance tiebreaker:** Drizzle wins cold starts — no engine binary, bundle ~90% smaller. Prisma 7 (late 2025) dropped the Rust engine for pure TypeScript, closing but not eliminating that gap. For serverless (Cloudflare Workers, Lambda) Drizzle is the safer default. For long-running Node servers Prisma's higher abstraction pays for itself.

**Migration maturity tiebreaker:** Prisma Migrate is the most polished in the JS/TS space. drizzle-kit is reliable but younger. TypeORM migrations drift frequently; treat them as untrusted and always inspect generated SQL. GORM's AutoMigrate is development-only — never use it in production. Alembic (SQLAlchemy) is production-grade with autogenerate support.


## Drizzle

### Schema definition

```typescript
// schema.ts — TypeScript is the schema; no separate file, no generation step
import { pgTable, serial, text, boolean, integer, index } from 'drizzle-orm/pg-core'

export const users = pgTable('users', {
  id: serial('id').primaryKey(),
  email: text('email').notNull().unique(),
})

export const posts = pgTable('posts', {
  id: serial('id').primaryKey(),
  title: text('title').notNull(),
  published: boolean('published').default(false),
  authorId: integer('author_id').notNull().references(() => users.id),
}, (t) => ({
  authorIdx: index('posts_author_idx').on(t.authorId),
}))
```

### Query patterns (Drizzle)

```typescript
import { db } from './db'
import { eq, and, gt } from 'drizzle-orm'
import { users, posts } from './schema'

// Explicit join — SQL maps directly to the query
const result = await db
  .select({ email: users.email, title: posts.title })
  .from(users)
  .innerJoin(posts, eq(users.id, posts.authorId))
  .where(and(eq(posts.published, true), gt(users.id, 0)))

// Relational query builder (avoids manual joins for nested reads)
const withPosts = await db.query.users.findMany({
  with: { posts: { where: eq(posts.published, true) } },
})

// Raw SQL with type inference
import { sql } from 'drizzle-orm'
const count = await db.select({ n: sql<number>`count(*)` }).from(posts)
```

### Migrations with drizzle-kit

```bash
# Generate SQL from schema diff
npx drizzle-kit generate

# Apply to dev DB
npx drizzle-kit migrate

# Push schema directly (dev only, no migration file)
npx drizzle-kit push
```

Never use `push` in production. Commit generated SQL migration files to version control just like Prisma.

### Connection pooling (serverless)

```typescript
import { drizzle } from 'drizzle-orm/neon-serverless'
import { Pool } from '@neondatabase/serverless'

const pool = new Pool({ connectionString: process.env.DATABASE_URL })
export const db = drizzle(pool, { schema })
```

For long-running servers use `postgres` (node-postgres) with a standard pool.


## SQLAlchemy 2.0

### Declarative models with mapped_column (2.0 style)

```python
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column, relationship
from sqlalchemy import String, Boolean, ForeignKey

class Base(DeclarativeBase):
    pass

class User(Base):
    __tablename__ = "users"
    id: Mapped[int] = mapped_column(primary_key=True)
    email: Mapped[str] = mapped_column(String(255), unique=True)
    posts: Mapped[list["Post"]] = relationship(back_populates="author")

class Post(Base):
    __tablename__ = "posts"
    id: Mapped[int] = mapped_column(primary_key=True)
    title: Mapped[str] = mapped_column(String(500))
    published: Mapped[bool] = mapped_column(Boolean, default=False)
    author_id: Mapped[int] = mapped_column(ForeignKey("users.id"), index=True)
    author: Mapped["User"] = relationship(back_populates="posts")
```

### Async session pattern (FastAPI)

```python
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker, AsyncSession

engine = create_async_engine(
    "postgresql+asyncpg://...",
    pool_pre_ping=True,
    pool_size=10,
    max_overflow=20,
)
# One sessionmaker per application; expire_on_commit=False avoids lazy-load
# errors after commit in async contexts
AsyncSessionLocal = async_sessionmaker(engine, expire_on_commit=False)

async def get_db():
    async with AsyncSessionLocal() as session:
        yield session
```

One engine per application. One session per request/task. Do not share sessions across tasks or event loops.

### Eager loading to prevent N+1

```python
from sqlalchemy.orm import selectinload, joinedload
from sqlalchemy import select

# selectinload: separate IN query per relationship — good for collections
stmt = select(User).options(selectinload(User.posts))

# joinedload: LEFT JOIN — good for many-to-one / single related record
stmt = select(Post).options(joinedload(Post.author))

result = await session.execute(stmt)
users = result.scalars().unique().all()
```

Prefer `selectinload` for one-to-many (avoids Cartesian explosion). Use `joinedload` for many-to-one.

### Migrations with Alembic

```bash
alembic revision --autogenerate -m "add published to posts"
alembic upgrade head
alembic downgrade -1   # roll back one step
```

Always review autogenerated migration scripts before applying — autogenerate misses some cases (CHECK constraints, custom types, conditional indexes).


## N+1 Problem: Detection and Solutions

The N+1 problem: fetch N parent rows, then issue one query per row to load a relation. Result is N+1 round trips instead of 1–2.

### Detection

- Enable query logging and look for repeated identical queries with different ID parameters.
- Use `EXPLAIN ANALYZE` on slow endpoints.
- Tools: Django Debug Toolbar, pgBadger, Drizzle's `logger: true`, Prisma's `log: ['query']`.

#### Solutions by ORM

| ORM | Solution | Mechanism |
| ----- | ---------- | ----------- |
| Prisma | `include` / `select` with nested | Generates a single batched query or JOIN |
| Drizzle | `.leftJoin()` or relational `with:` | Explicit SQL JOIN or batch select |
| TypeORM | `QueryBuilder.leftJoinAndSelect` | Explicit JOIN; avoid `eager: true` on entities |
| SQLAlchemy | `selectinload` / `joinedload` | IN-query batch or LEFT JOIN |
| GORM | `Preload()` | IN-query batch |

#### DataLoader pattern (GraphQL/API layers)

When eager loading is impractical (e.g., dynamic GraphQL resolvers), collect all relation IDs within a request cycle and issue one batched `IN` query. Libraries: `dataloader` (JS), `strawberry-django` (Python), `graph-gophers/dataloader` (Go).


## Connection Pooling

| ORM | Default pool | Production config |
| ----- | ------------- | ------------------- |
| Prisma | Built-in connection pool via query engine | Set `connection_limit` in DATABASE_URL; use PgBouncer for high concurrency |
| Drizzle | Delegates to underlying driver (pg, mysql2, etc.) | Configure pool in the driver; Drizzle adds no extra pool |
| TypeORM | Built-in via `extra: { max, min }` | Set `extra.max` in DataSource options |
| SQLAlchemy | QueuePool (sync), AsyncAdaptedQueuePool (async) | `pool_size`, `max_overflow`, `pool_pre_ping=True` |
| GORM | Wraps `database/sql` pool | `SetMaxOpenConns`, `SetMaxIdleConns`, `SetConnMaxLifetime` |

For serverless (AWS Lambda, Cloudflare Workers): use a connection proxy (PgBouncer, RDS Proxy, Neon's serverless driver, Supabase's connection pooler) because each invocation would otherwise open a new connection and exhaust the database limit.


## Testing Patterns

### Unit tests (mock the ORM)

- **Prisma**: `jest-mock-extended` → `mockDeep<PrismaClient>()`, inject via DI.
- **Drizzle**: wrap db calls in a thin repository interface; mock the interface.
- **TypeORM**: use `@InjectRepository` with NestJS testing module; mock with `jest.fn()`.
- **SQLAlchemy**: use `unittest.mock.AsyncMock` or pytest fixtures that provide a session backed by an in-memory SQLite DB (for sync) or `pytest-asyncio` + real test DB (for async).
- **GORM**: use `go-sqlmock` to intercept queries without a real database.

### Integration tests (real database)

Use a throwaway database per test run. Options:

- Docker Compose service spun up in CI.
- `testcontainers` libraries (available for Node, Python, Go).
- For Prisma: `prisma migrate reset --force` in `beforeAll`.
- For Alembic: run `alembic upgrade head` against a fresh `TEST_DATABASE_URL`.
- For GORM: `golang-migrate` apply + teardown per suite.

Integration tests should test the ORM queries, not mock them. Unit tests mock to isolate business logic from I/O.


---


# Prisma and Drizzle ORM — TypeScript Patterns

## When to Use (Decision Matrix)

| Concern | Prisma | Drizzle |
| --- | --- | --- |
| Type safety model | Generated client types from schema DSL | Inferred directly from TypeScript schema definitions |
| Query style | ORM API (`findMany`, `create`, etc.) | SQL-like builder + relational query API |
| Bundle size | Heavier (generated client, query engine binary) | Lightweight, zero dependencies |
| Raw SQL | `$queryRaw`, `$executeRaw` (less ergonomic) | `sql` template tag — first-class, fully typed |
| Migrations | Declarative diff-based (`prisma migrate dev`) | `drizzle-kit generate` + `drizzle-kit migrate` or `push` |
| Serverless fit | Acceptable with Prisma Accelerate / connection pooler | Excellent — always one SQL query per operation |
| Ecosystem maturity | Mature, large community, rich docs | Newer, rapidly growing, thinner middleware ecosystem |
| Schema as source of truth | `.prisma` DSL file | TypeScript files — schema IS the types |

Choose Prisma when you want a fully managed migration workflow, a rich middleware/plugin ecosystem, and prefer ORM-style APIs. Choose Drizzle when you want SQL transparency, smaller bundle size, serverless performance, or you want your schema to live entirely in TypeScript.


## Prisma: Migrations

```bash
# Development: generate SQL migration + apply to dev DB + regenerate client
npx prisma migrate dev --name add-user-role

# Production: apply all pending migrations (no shadow DB, no client regen)
npx prisma migrate deploy

# Baseline an existing DB (mark existing state as migration 0)
npx prisma migrate diff \
  --from-empty \
  --to-schema-datamodel prisma/schema.prisma \
  --script > prisma/migrations/0_init/migration.sql
npx prisma migrate resolve --applied 0_init

# Regenerate client without migrating (after pulling schema changes)
npx prisma generate

# Inspect current DB state
npx prisma db pull
```

Shadow database: `prisma migrate dev` creates a temporary shadow DB to safely validate the generated SQL diff before applying it to your real dev database. Required for PostgreSQL; SQLite uses a temp file. Set `shadowDatabaseUrl` in the datasource block when your DB user lacks CREATE DATABASE privileges.

Migration files are committed to version control. Never edit applied migration SQL — create a new migration instead.


## Prisma: Relations

```typescript
// One-to-many: create parent + children in one round trip
const author = await prisma.user.create({
  data: {
    email: "author@example.com",
    posts: {
      create: [
        { title: "First post" },
        { title: "Second post" },
      ],
    },
  },
  include: { posts: true },
});

// Connect existing records
await prisma.post.update({
  where: { id: 5 },
  data: {
    categories: {
      connect: [{ id: 1 }, { id: 2 }],
    },
  },
});

// connectOrCreate — create if absent, connect if present
await prisma.post.create({
  data: {
    title: "Tagged post",
    categories: {
      connectOrCreate: {
        where: { name: "TypeScript" },
        create: { name: "TypeScript" },
      },
    },
  },
});

// Disconnect relation
await prisma.post.update({
  where: { id: 5 },
  data: {
    categories: { disconnect: [{ id: 1 }] },
  },
});
```

Explicit many-to-many join model (when you need extra columns on the join):

```prisma
model PostCategory {
  postId     Int
  categoryId Int
  assignedAt DateTime @default(now())

  post     Post     @relation(fields: [postId], references: [id])
  category Category @relation(fields: [categoryId], references: [id])

  @@id([postId, categoryId])
}
```


## Prisma: Performance

### Avoid N+1 — always use `include` or batched `findMany` with `in`

```typescript
// Bad — N+1
const users = await prisma.user.findMany();
for (const u of users) {
  const posts = await prisma.post.findMany({ where: { authorId: u.id } });
}

// Good — one query
const usersWithPosts = await prisma.user.findMany({
  include: { posts: true },
});
```

#### Select only what you need

```typescript
// Returns only id + email, narrower type, less data transferred
const users = await prisma.user.findMany({
  select: { id: true, email: true },
});
```

**Connection pooling:** Prisma opens a connection pool by default. In serverless environments each function instance creates its own pool — use [Prisma Accelerate](https://www.prisma.io/data-platform/accelerate) or PgBouncer to cap total connections.

```typescript
// Instantiate once per process, not per request
const prisma = new PrismaClient({
  log: process.env.NODE_ENV === "development" ? ["query", "warn", "error"] : ["error"],
});
```

**Batch operations** (`createMany`, `updateMany`, `deleteMany`) issue a single SQL statement. Prefer them over loops of individual writes.

#### Raw SQL when needed

```typescript
const result = await prisma.$queryRaw<{ count: bigint }[]>`
  SELECT COUNT(*) as count FROM "User" WHERE role = ${Role.ADMIN}
`;
```


## Drizzle: Migrations

```bash
# Generate SQL migration files from schema diff
npx drizzle-kit generate

# Apply pending migrations to the database
npx drizzle-kit migrate

# Push schema directly to DB without migration files (dev only)
npx drizzle-kit push

# Inspect DB and pull schema back
npx drizzle-kit introspect
```

`drizzle.config.ts`:

```typescript
import { defineConfig } from "drizzle-kit";

export default defineConfig({
  schema: "./src/db/schema.ts",
  out: "./drizzle",
  dialect: "postgresql",
  dbCredentials: {
    url: process.env.DATABASE_URL!,
  },
});
```

Use `generate` + `migrate` in CI/production. Use `push` only in local dev when you want fast iteration without migration files. Never use `push` against a production database.


## Drizzle: Relations

Define relations separately from the schema using `defineRelations` (v2):

```typescript
import { defineRelations } from "drizzle-orm";

export const relations = defineRelations({ users, posts, categories, postCategories }, (r) => ({
  users: {
    posts: r.many.posts(),
  },
  posts: {
    author: r.one.users({
      from: r.posts.authorId,
      to: r.users.id,
    }),
    postCategories: r.many.postCategories(),
  },
  postCategories: {
    post: r.one.posts({
      from: r.postCategories.postId,
      to: r.posts.id,
    }),
    category: r.one.categories({
      from: r.postCategories.categoryId,
      to: r.categories.id,
    }),
  },
}));

// Pass relations to drizzle()
const db = drizzle(pool, { relations });

// Eager loading with `with`
const usersWithPosts = await db.query.users.findMany({
  with: {
    posts: {
      with: {
        postCategories: {
          with: { category: true },
        },
      },
    },
  },
});

// Partial columns + filter on related
const posts = await db.query.posts.findMany({
  columns: { id: true, title: true },
  where: { published: { eq: true } },
  with: {
    author: {
      columns: { id: true, name: true },
    },
  },
  limit: 10,
  orderBy: { id: "desc" },
});
```

Drizzle's relational query API always emits exactly one SQL query regardless of nesting depth.


## Drizzle: Raw SQL and Escaping

The `sql` template tag produces typed, parameterized queries. Never concatenate user input into query strings.

```typescript
import { sql } from "drizzle-orm";

// Typed raw select
const result = await db.execute<{ count: number }>(
  sql`SELECT COUNT(*) as count FROM ${users} WHERE ${users.role} = 'admin'`
);

// sql tag in update — expression-level raw SQL
await db.update(posts).set({
  views: sql`${posts.views} + 1`,
});

// Extras (computed columns) in relational queries
const enriched = await db.query.users.findMany({
  extras: {
    fullName: (u, { sql }) => sql<string>`concat(${u.name}, ' (', ${u.email}, ')')`,
  },
});

// Prepared statements with placeholders
import { placeholder } from "drizzle-orm";

const prepared = db
  .select()
  .from(users)
  .where(eq(users.id, placeholder("id")))
  .prepare("get_user_by_id");

const user = await prepared.execute({ id: 1 });
```


## Critical Rules / Gotchas

### Prisma

- Always run `npx prisma generate` after any schema change, including in CI before building. The generated client is not committed to the repo; it must be regenerated at build time.
- `findUnique` returns `null` when not found — not an exception. Use `findUniqueOrThrow` if absence is a bug.
- `select` and `include` are mutually exclusive at the top level of a query.
- Interactive transactions hold a live DB connection for their entire duration. Do not await external HTTP calls inside them.
- Never manually edit SQL files inside `prisma/migrations/` for already-applied migrations. Create a new migration instead.
- `createMany` does not support nested relation writes (e.g., you cannot create posts with nested categories in a single `createMany`).
- In serverless environments, create `PrismaClient` once per module (module-level singleton), not inside request handlers, to avoid exhausting the connection pool.

#### Drizzle

- Drizzle does not validate data at runtime. If you need input validation, use Zod (or `drizzle-zod`) separately — Drizzle only enforces types at compile time.
- `drizzle-kit push` is for local dev only. It can cause data loss in production by applying destructive schema diffs without a migration history.
- Relations defined via `defineRelations` are only used by the relational query API (`db.query.*`). They do not affect the SQL builder API (`db.select().from(...)`) — you still need explicit `leftJoin` / `innerJoin` there.
- Column types differ across dialects. `serial` is PostgreSQL-only; use `int().autoincrement()` for MySQL and `integer({ mode: "number" })` for SQLite.
- `returning()` is not supported by MySQL — use `insertId` from the result metadata instead.
- Always pass the `relations` object to `drizzle(driver, { relations })` when using the relational query API, otherwise `db.query` will be empty.

