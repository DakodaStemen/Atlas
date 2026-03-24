---
name: hono-fastify-frameworks
description: Hono (edge-first, ultrafast) and Fastify (fastest Node.js) web frameworks — routing, middleware, validation, serialization, and deployment patterns.
domain: backend
category: web-framework
tags: [Hono, Fastify, Node.js, edge, web-framework, routing, middleware, TypeScript, Bun, Cloudflare Workers]
triggers: Hono framework, Fastify server, Hono middleware, Fastify plugin, Hono route, Fastify schema validation, Hono Cloudflare Worker, Hono Bun
---

# Hono & Fastify Web Frameworks

## When to Use (Hono vs Fastify vs Express)

**Hono** — choose when:

- Deploying to edge runtimes (Cloudflare Workers, Fastly, Deno Deploy, Bun)
- You need a single codebase that runs on multiple runtimes without modification
- End-to-end TypeScript RPC type safety (`hono/client`) matters
- Bundle size is a constraint — Hono has zero Node.js-only dependencies

**Fastify** — choose when:

- Running on Node.js (or Bun) in a traditional server environment
- You need the absolute fastest JSON throughput in Node.js (outperforms Express 3–5×)
- Schema-driven validation and serialization via JSON Schema / TypeBox is the priority
- Plugin encapsulation and a rich ecosystem of `@fastify/*` plugins are needed

**Express** — only for legacy maintenance. It has no built-in schema validation, no serialization optimization, and performance trails far behind both alternatives.

### Edge compatibility matrix

| Framework | CF Workers | Bun | Deno | Node.js | Lambda |
| ----------- | ----------- | ----- | ------ | --------- | -------- |
| Hono | ✓ native | ✓ | ✓ | ✓ adapter | ✓ adapter |
| Fastify | ✗ | ✓ | partial | ✓ native | ✓ |
| Express | ✗ | partial | partial | ✓ | ✓ |

---

## Hono: Routing

Hono uses the `RegExpRouter` (default) or `TrieRouter`, both compiled at startup for fast matching.

```ts
import { Hono } from 'hono'

const app = new Hono()

// Basic HTTP methods
app.get('/users', (c) => c.json({ users: [] }))
app.post('/users', (c) => c.text('Created', 201))
app.put('/users/:id', (c) => c.text(`Updated ${c.req.param('id')}`))
app.delete('/users/:id', (c) => c.text(`Deleted ${c.req.param('id')}`))

// Multiple path parameters
app.get('/posts/:postId/comments/:commentId', (c) => {
  const { postId, commentId } = c.req.param()
  return c.json({ postId, commentId })
})

// Optional param
app.get('/api/animal/:type?', (c) => c.text('ok'))

// Regex constraint on param
app.get('/post/:date{[0-9]+}/:slug{[a-z-]+}', (c) => {
  const { date, slug } = c.req.param()
  return c.json({ date, slug })
})

// Wildcard
app.get('/static/*', (c) => c.text('static file'))

// Match any method
app.all('/ping', (c) => c.text('pong'))

// Multiple methods on same path
app.on(['PUT', 'DELETE'], '/resource', (c) => c.text('ok'))
```

**Route groups** — compose sub-apps onto a base path:

```ts
const api = new Hono()
api.get('/health', (c) => c.json({ ok: true }))
api.get('/users', (c) => c.json([]))

const app = new Hono()
app.route('/api/v1', api)     // /api/v1/health, /api/v1/users
```

**Chained routes** on the same path:

```ts
app
  .get('/item', (c) => c.text('GET'))
  .post((c) => c.text('POST'))
  .delete((c) => c.text('DELETE'))
```

### Base path scoping

```ts
const v2 = new Hono().basePath('/v2')
v2.get('/users', (c) => c.json([]))
app.route('/', v2)   // exposes /v2/users
```

---

## Hono: Middleware

Middleware runs in registration order before (and optionally after, via `await next()`) the route handler.

```ts
import { cors } from 'hono/cors'
import { logger } from 'hono/logger'
import { jwt } from 'hono/jwt'
import { basicAuth } from 'hono/basic-auth'

// Global middleware
app.use(logger())
app.use(cors({ origin: 'https://example.com', credentials: true }))

// Path-scoped middleware
app.use('/api/*', jwt({ secret: process.env.JWT_SECRET! }))
app.use('/admin/*', basicAuth({ username: 'admin', password: 'secret' }))
```

**Custom middleware** — the handler receives the context and a `next` function:

```ts
import type { MiddlewareHandler } from 'hono'

const timing: MiddlewareHandler = async (c, next) => {
  const start = Date.now()
  await next()                              // run downstream handlers
  const ms = Date.now() - start
  c.res.headers.set('X-Response-Time', `${ms}ms`)
}

app.use(timing)
```

**Error-handling middleware** — use `app.onError`:

```ts
app.onError((err, c) => {
  console.error(err)
  return c.json({ error: err.message }, 500)
})
```

### Not-found handler

```ts
app.notFound((c) => c.json({ error: 'Not Found' }, 404))
```

Built-in middleware catalogue: `basicAuth`, `bearerAuth`, `bodyLimit`, `cache`, `compress`, `cors`, `csrf`, `etag`, `ipRestriction`, `jsxRenderer`, `jwt`, `logger`, `methodOverride`, `prettyJSON`, `requestId`, `secureHeaders`, `timeout`, `timing`, `trailingSlash`.

---

## Hono: Context API

The `c` parameter is a `Context` object available in every handler and middleware.

```ts
app.get('/demo', async (c) => {
  // Request access
  const id      = c.req.param('id')         // path param
  const page    = c.req.query('page')        // query string
  const auth    = c.req.header('Authorization')
  const body    = await c.req.json()         // parse JSON body
  const form    = await c.req.formData()

  // Response helpers
  return c.json({ ok: true }, 200)           // application/json
  return c.text('Hello')                     // text/plain
  return c.html('<h1>Hello</h1>')            // text/html
  return c.redirect('/other', 302)
  return c.body('raw bytes', 200, { 'Content-Type': 'application/octet-stream' })

  // Set response header / status without returning yet
  c.header('X-Request-Id', 'abc123')
  c.status(201)
})

// Request-scoped key/value store (shared across middleware and handler)
const authMiddleware: MiddlewareHandler = async (c, next) => {
  const user = await resolveUser(c.req.header('Authorization')!)
  c.set('user', user)   // store
  await next()
}
app.get('/me', authMiddleware, (c) => {
  const user = c.get('user')    // retrieve; typed via generics
  return c.json(user)
})

// Cloudflare Workers bindings
app.get('/kv', (c) => {
  const ns = c.env.MY_KV          // KVNamespace binding from wrangler.toml
  c.executionCtx.waitUntil(ns.put('key', 'value'))
  return c.text('queued')
})

// Access raw Response for header mutation
app.use(async (c, next) => {
  await next()
  c.res.headers.append('X-Powered-By', 'Hono')
})
```

**Typed variables** — declare the variable map on the `Hono` instance to get type-safe `c.get/set` and `c.var`:

```ts
type Variables = { user: { id: string; role: string } }

const app = new Hono<{ Variables: Variables }>()

app.use(async (c, next) => {
  c.set('user', { id: '1', role: 'admin' })
  await next()
})

app.get('/profile', (c) => {
  const { id, role } = c.var.user   // fully typed
  return c.json({ id, role })
})
```

---

## Hono: Validation

Hono ships a thin built-in validator; pair it with `@hono/zod-validator` for production use.

```ts
import { z } from 'zod'
import { zValidator } from '@hono/zod-validator'

const CreateUserSchema = z.object({
  name: z.string().min(1),
  email: z.string().email(),
  age: z.number().int().positive().optional(),
})

app.post(
  '/users',
  zValidator('json', CreateUserSchema),       // validates req body
  (c) => {
    const data = c.req.valid('json')          // typed as z.infer<typeof CreateUserSchema>
    return c.json({ created: data }, 201)
  }
)

// Query string validation
const ListSchema = z.object({
  page: z.coerce.number().default(1),
  limit: z.coerce.number().max(100).default(20),
})

app.get(
  '/users',
  zValidator('query', ListSchema),
  (c) => {
    const { page, limit } = c.req.valid('query')
    return c.json({ page, limit })
  }
)

// Param validation
app.get(
  '/users/:id',
  zValidator('param', z.object({ id: z.string().uuid() })),
  (c) => c.json({ id: c.req.valid('param').id })
)

// Custom error formatting — second argument to zValidator
app.post(
  '/items',
  zValidator('json', CreateUserSchema, (result, c) => {
    if (!result.success) {
      return c.json(
        { errors: result.error.flatten().fieldErrors },
        422
      )
    }
  }),
  (c) => c.json({ ok: true })
)
```

Chain multiple validators on one route:

```ts
app.post(
  '/posts/:id/comments',
  zValidator('param', z.object({ id: z.string().uuid() })),
  zValidator('json', z.object({ body: z.string().min(1) })),
  (c) => {
    const { id } = c.req.valid('param')
    const { body } = c.req.valid('json')
    return c.json({ id, body }, 201)
  }
)
```

---

## Hono: RPC Client

Export the app type from the server, import it on the client — zero code-gen, full type inference.

### Server (shared type export)

```ts
// server/routes/users.ts
import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'

const users = new Hono()
  .get('/', (c) => c.json([{ id: '1', name: 'Alice' }]))
  .post(
    '/',
    zValidator('json', z.object({ name: z.string() })),
    (c) => c.json({ id: '2', ...c.req.valid('json') }, 201)
  )
  .get('/:id', (c) => c.json({ id: c.req.param('id'), name: 'Alice' }))

export type UsersType = typeof users
export default users

// server/index.ts
const app = new Hono().route('/users', users)
export type AppType = typeof app
```

#### Client

```ts
import { hc } from 'hono/client'
import type { AppType } from '../server'

const client = hc<AppType>('http://localhost:8787')

// Fully typed request and response
const res = await client.users.$get()
const users = await res.json()   // typed as { id: string; name: string }[]

// POST with body
const created = await client.users.$post({ json: { name: 'Bob' } })

// Path params
const user = await client.users[':id'].$get({ param: { id: '1' } })

// Infer request / response types statically
import type { InferRequestType, InferResponseType } from 'hono/client'
type CreateBody = InferRequestType<typeof client.users.$post>['json']
type CreateResponse = InferResponseType<typeof client.users.$post, 201>
```

**Monorepo setup** — both tsconfig files must have `"strict": true` for the type inference to work correctly.

---

## Hono: Deployments

The same application code runs everywhere; only the entry point differs.

**Cloudflare Workers** (`wrangler.toml` + adapter):

```ts
// src/index.ts
import { Hono } from 'hono'

type Bindings = { MY_KV: KVNamespace; DB: D1Database }

const app = new Hono<{ Bindings: Bindings }>()
app.get('/', (c) => c.text('Hello from Workers'))

export default app   // Workers expects export default { fetch }
```

### Bun

```ts
import { Hono } from 'hono'

const app = new Hono()
app.get('/', (c) => c.text('Hello Bun'))

export default {
  port: 3000,
  fetch: app.fetch,
}
```

**Node.js** (requires `@hono/node-server`):

```ts
import { serve } from '@hono/node-server'
import { Hono } from 'hono'

const app = new Hono()
app.get('/', (c) => c.text('Hello Node'))

serve({ fetch: app.fetch, port: 3000 })
```

#### Deno

```ts
import { Hono } from 'hono'

const app = new Hono()
app.get('/', (c) => c.text('Hello Deno'))

Deno.serve(app.fetch)
```

**AWS Lambda** (requires `hono/aws-lambda`):

```ts
import { Hono } from 'hono'
import { handle } from 'hono/aws-lambda'

const app = new Hono()
app.get('/', (c) => c.text('Hello Lambda'))

export const handler = handle(app)
```

---

## Fastify: Plugin System

Plugins are the unit of encapsulation in Fastify. Every `register` call creates a new scope: routes and decorators added inside are not visible outside unless you opt out of encapsulation using `fastify-plugin`.

```ts
import Fastify, { FastifyPluginAsync } from 'fastify'
import fp from 'fastify-plugin'

// Encapsulated plugin — routes/decorators stay scoped
const usersPlugin: FastifyPluginAsync = async (fastify) => {
  fastify.get('/users', async () => [])
  fastify.post('/users', async (req) => req.body)
}

// Non-encapsulated plugin — decorations leak upward (use for shared services)
const dbPlugin = fp(async (fastify) => {
  const db = await connectDB()
  fastify.decorate('db', db)   // available on all instances after this runs
}, { name: 'db', fastify: '5.x' })

const app = Fastify({ logger: true })

await app.register(dbPlugin)
await app.register(usersPlugin, { prefix: '/api/v1' })
await app.register(import('./routes/auth'), { prefix: '/auth' })

await app.listen({ port: 3000 })
```

**Route-level prefix** — pass `{ prefix: '/api' }` to `register`; all routes inside inherit it.

**`fastify.decorate`** adds properties to the Fastify instance; **`fastify.decorateRequest`** and **`fastify.decorateReply`** extend request/reply objects — always initialize decorators with a null/empty value at decoration time to avoid prototype pollution warnings.

**Plugin load order** is controlled by `fastify.after()` or simply `await app.register(...)`. Plugins registered at the same level load in registration order.

---

## Fastify: Schema Validation

Fastify compiles JSON Schema into optimized validation functions via Ajv v8 at startup — validation overhead is near-zero at runtime.

```ts
import { FastifyInstance } from 'fastify'

async function routes(fastify: FastifyInstance) {
  const createUserSchema = {
    body: {
      type: 'object',
      required: ['name', 'email'],
      properties: {
        name:  { type: 'string', minLength: 1 },
        email: { type: 'string', format: 'email' },
        age:   { type: 'integer', minimum: 0 },
      },
      additionalProperties: false,
    },
    querystring: {
      type: 'object',
      properties: {
        dryRun: { type: 'boolean' },
      },
    },
    response: {
      201: {
        type: 'object',
        properties: {
          id:   { type: 'string' },
          name: { type: 'string' },
        },
      },
    },
  } as const

  fastify.post('/users', { schema: createUserSchema }, async (req, reply) => {
    reply.status(201).send({ id: 'uuid', name: (req.body as any).name })
  })
}
```

**TypeBox** — derive JSON Schema and TypeScript types from the same definition:

```ts
import { Type, Static } from '@sinclair/typebox'
import { TypeBoxTypeProvider } from '@fastify/type-provider-typebox'

const UserBody = Type.Object({
  name:  Type.String({ minLength: 1 }),
  email: Type.String({ format: 'email' }),
})
type UserBodyType = Static<typeof UserBody>

const app = Fastify().withTypeProvider<TypeBoxTypeProvider>()

app.post<{ Body: UserBodyType }>(
  '/users',
  { schema: { body: UserBody } },
  async (req) => {
    // req.body is typed as UserBodyType — no cast needed
    return { name: req.body.name }
  }
)
```

**Ajv configuration** — customize at instance creation:

```ts
const app = Fastify({
  ajv: {
    customOptions: {
      removeAdditional: 'all',   // strip unknown fields
      coerceTypes: 'array',      // coerce query strings to declared types
      allErrors: false,          // stop on first error (faster)
      useDefaults: true,         // fill in schema defaults
    },
  },
})
```

**Shared schemas** via `$ref`:

```ts
app.addSchema({
  $id: 'Address',
  type: 'object',
  properties: {
    street: { type: 'string' },
    city:   { type: 'string' },
  },
})

app.post('/delivery', {
  schema: {
    body: {
      type: 'object',
      properties: { address: { $ref: 'Address#' } },
    },
  },
}, async (req) => req.body)
```

---

## Fastify: Serialization

Response schemas do double duty: they validate the outgoing shape AND enable `fast-json-stringify` to serialize 2–5× faster than `JSON.stringify` by skipping type inference.

```ts
// Response schema tells fast-json-stringify the exact shape
app.get('/users/:id', {
  schema: {
    params: { type: 'object', properties: { id: { type: 'string' } } },
    response: {
      200: {
        type: 'object',
        properties: {
          id:        { type: 'string' },
          name:      { type: 'string' },
          createdAt: { type: 'string', format: 'date-time' },
        },
      },
      404: {
        type: 'object',
        properties: { error: { type: 'string' } },
      },
    },
  },
}, async (req, reply) => {
  const user = await db.findUser(req.params.id)
  if (!user) return reply.status(404).send({ error: 'Not Found' })
  return user   // serialized via fast-json-stringify; extra fields are stripped
})
```

**Custom serializer** for a specific route:

```ts
app.get('/raw', {
  schema: { response: { 200: { type: 'string' } } },
  serializerCompiler: () => (data) => JSON.stringify(data),
}, async () => 'hello')
```

**Serializer cache** — Fastify caches compiled serializers per schema reference. Use `addSchema` + `$ref` rather than defining inline schemas per route to maximize cache hits and reduce startup time.

Fields not listed in the response schema are **silently omitted** from the response. This is intentional for security (prevents leaking internal fields) but can bite you during development if you forget to add a new field to the schema.

---

## Fastify: Hooks

Hooks intercept the request lifecycle. They run in this order:

```bash
onRequest → preParsing → preValidation → preHandler → handler
    → preSerialization → onSend → onResponse
```

`onError` runs when an error is thrown in any phase before `onSend`.

```ts
import {
  FastifyRequest,
  FastifyReply,
  HookHandlerDoneFunction,
} from 'fastify'

// Global authentication hook
app.addHook('onRequest', async (request: FastifyRequest, reply: FastifyReply) => {
  const token = request.headers.authorization?.replace('Bearer ', '')
  if (!token) {
    return reply.status(401).send({ error: 'Unauthorized' })
    // returning early from onRequest aborts the chain
  }
})

// Attach user to request before handler runs
app.addHook('preHandler', async (request) => {
  request.user = await verifyToken(request.headers.authorization!)
})

// Modify response payload before serialization
app.addHook('onSend', async (request, reply, payload) => {
  reply.header('X-Request-Id', request.id)
  return payload   // must return (possibly modified) payload
})

// Error hook — for logging only, do NOT call reply.send here
app.addHook('onError', async (request, reply, error) => {
  metrics.increment('errors', { route: request.routeOptions.url })
})
```

**Route-level hooks** run after global hooks of the same type:

```ts
app.route({
  method: 'DELETE',
  url: '/admin/users/:id',
  preHandler: [
    globalAuthHook,
    async (req) => { if (req.user.role !== 'admin') throw new Error('Forbidden') },
  ],
  handler: async (req, reply) => {
    await db.deleteUser(req.params.id)
    reply.status(204).send()
  },
})
```

---

## Fastify: TypeScript

```ts
import Fastify, {
  FastifyInstance,
  FastifyRequest,
  FastifyReply,
  RouteGenericInterface,
} from 'fastify'
import fp, { FastifyPluginAsync } from 'fastify-plugin'

// Typed route via RouteGenericInterface
interface GetUserRoute extends RouteGenericInterface {
  Params: { id: string }
  Reply: { 200: { id: string; name: string }; 404: { error: string } }
}

app.get<GetUserRoute>('/users/:id', async (req, reply) => {
  const user = await db.find(req.params.id)   // req.params.id: string
  if (!user) return reply.status(404).send({ error: 'Not Found' })
  return reply.send({ id: user.id, name: user.name })
})

// Plugin with typed decorator — declaration merging
declare module 'fastify' {
  interface FastifyRequest {
    user?: { id: string; role: string }
  }
  interface FastifyInstance {
    db: ReturnType<typeof createDbClient>
  }
}

const authPlugin: FastifyPluginAsync = fp(async (fastify) => {
  fastify.addHook('preHandler', async (req) => {
    req.user = await resolveUser(req.headers.authorization)
  })
})

// fastify-plugin wrapping — always include name and fastify version
export default fp(authPlugin, { name: 'auth', fastify: '5.x' })
```

**Type providers** (`@fastify/type-provider-typebox` or `@fastify/type-provider-json-schema-to-ts`) eliminate the manual `RouteGenericInterface` ceremony by inferring types directly from the schema object.

---

## Fastify: Performance

Fastify consistently ranks at or near the top of Node.js HTTP framework benchmarks (wrk, autocannon). Key reasons:

- **Schema compilation at startup**: Ajv and fast-json-stringify compile validators/serializers once; per-request cost is negligible.
- **Pino logger**: default logger; async, low-overhead. Disable logging entirely (`logger: false`) in latency-critical paths.
- **Reply pipeline**: avoid `res.json()` anti-patterns; use `reply.send(data)` and let the serializer do its job.

Practical rules:

```ts
// Always define a response schema for JSON routes — enables fast-json-stringify
// Without it, Fastify falls back to JSON.stringify (3–5× slower)
fastify.get('/hot-path', {
  schema: { response: { 200: HotResponseSchema } },
}, async () => heavyComputation())

// Avoid async where sync suffices — saves a microtask queue cycle
fastify.get('/simple', {
  schema: { response: { 200: { type: 'string' } } },
}, (req, reply) => {
  reply.send('pong')   // sync handler, no async overhead
})

// Reuse Fastify instance across Lambda invocations — don't create it per request
let app: FastifyInstance
export async function handler(event: any) {
  if (!app) {
    app = buildApp()
    await app.ready()
  }
  return app.inject({ method: event.httpMethod, url: event.path })
}

// Use addSchema + $ref for shared shapes — serializer cache hit
app.addSchema({ $id: 'User', ...userSchema })
app.get('/me',  { schema: { response: { 200: { $ref: 'User#' } } } }, ...)
app.get('/you', { schema: { response: { 200: { $ref: 'User#' } } } }, ...)
```

---

## Critical Rules / Gotchas

### Hono

**Runtime detection** — `c.env` is only populated on Cloudflare Workers. On Node.js, use environment variables via `process.env` directly or inject them via the `Hono` constructor's `Bindings` generic. Never assume `c.env` exists in a runtime-agnostic handler.

```ts
// Wrong — breaks on Node.js
app.get('/key', (c) => c.json({ key: c.env.SECRET }))

// Correct — conditional or per-runtime type
type Bindings = { SECRET: string }
const app = new Hono<{ Bindings: Bindings }>()
```

**Middleware and handler registration order is significant.** A middleware registered after a route will not run for that route. Always register global middleware before route definitions.

**`await next()` must be called in middleware** for the chain to continue. Forgetting it or not awaiting it produces silent truncation of the response.

**Validation content-type gotcha** — `zValidator('json', ...)` requires `Content-Type: application/json`. Sending a body without this header returns an empty parse result, which fails validation with a confusing error.

**RPC client requires strict TypeScript** — set `"strict": true` in every tsconfig (client and server) in a monorepo, or type inference on `hc<AppType>` silently degrades to `any`.

### Fastify

**Async plugin registration** — never call `fastify.listen` before all plugins have loaded. Either `await app.register(...)` sequentially, or call `await app.ready()` after all synchronous `register` calls before starting the server. Unresolved async plugins cause subtle "decorator not found" errors at runtime.

```ts
// Wrong — listen may start before dbPlugin finishes async init
app.register(dbPlugin)
app.listen({ port: 3000 })

// Correct
await app.register(dbPlugin)
await app.listen({ port: 3000 })
```

**Schema compilation timing** — all `addSchema` calls must happen before the first `app.listen` / `app.ready`. Schemas added after the server starts are not compiled and will throw.

**Do not mix async and callback style in hooks.** Calling `done()` in an async function, or not calling it in a callback function, runs the hook chain twice or hangs the request.

**`fastify-plugin` vs plain function** — wrapping with `fp` is required for a plugin's decorators to be visible outside its scope. A plain async function always creates an encapsulated child scope; decorators set inside it disappear at the parent level.

**Response schema strips unknown fields** — if a handler returns an object with a field not listed in the response schema, that field is silently omitted. Add all fields you intend to send to the schema.

---

## References

- Hono docs: <https://hono.dev/docs/>
- Hono routing API: <https://hono.dev/docs/api/routing>
- Hono context API: <https://hono.dev/docs/api/context>
- Hono validation guide: <https://hono.dev/docs/guides/validation>
- Hono RPC guide: <https://hono.dev/docs/guides/rpc>
- Hono best practices: <https://hono.dev/docs/guides/best-practices>
- `@hono/zod-validator`: <https://github.com/honojs/middleware/tree/main/packages/zod-validator>
- Fastify plugins reference: <https://fastify.dev/docs/latest/Reference/Plugins/>
- Fastify validation & serialization: <https://fastify.dev/docs/latest/Reference/Validation-and-Serialization/>
- Fastify hooks reference: <https://fastify.dev/docs/latest/Reference/Hooks/>
- Fastify TypeScript guide: <https://fastify.dev/docs/latest/Reference/TypeScript/>
- `@sinclair/typebox`: <https://github.com/sinclairzx81/typebox>
- `fastify-plugin` package: <https://github.com/fastify/fastify-plugin>
