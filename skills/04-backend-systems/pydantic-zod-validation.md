---
name: pydantic-zod-validation
description: Pydantic v2 (Python) and Zod (TypeScript) — schema definition, parsing, serialization, custom validators, and integration with FastAPI/tRPC.
domain: backend
category: validation
tags: [Pydantic, Zod, validation, schema, TypeScript, Python, FastAPI, parsing, serialization]
triggers: Pydantic model, Zod schema, data validation Python, input validation TypeScript, Pydantic v2, Zod parse, model_validator, Pydantic BaseModel
---

# Pydantic v2 & Zod — Data Validation Patterns

## When to Use

**Parse, don't validate** is the core principle for both libraries. The goal is not to check data and return a boolean — it is to transform untrusted input into a typed, guaranteed-correct value. After parsing, the rest of the application operates on types it can trust unconditionally.

**Trust boundary placement**: validate at every external boundary — HTTP request bodies, CLI arguments, environment variables, database rows read from legacy tables, webhook payloads, file imports. Never re-validate data that has already crossed the boundary and stayed in your typed domain.

**Pydantic vs attrs vs dataclasses** (Python):

- `dataclasses` — no validation at all; pure structural convenience. Use when you own all data construction.
- `attrs` — opt-in validators; good for internal data structures where performance trumps ergonomics.
- `Pydantic` — automatic coercion + validation + serialization + JSON schema generation. Use at every I/O boundary. Pairs naturally with FastAPI, SQLModel, and any schema-driven workflow.

---

## Pydantic v2: BaseModel

```python
from pydantic import BaseModel, ConfigDict, Field
from typing import Annotated
from datetime import datetime

class User(BaseModel):
    model_config = ConfigDict(
        str_strip_whitespace=True,   # strip leading/trailing whitespace on all str fields
        extra="forbid",              # reject unknown keys — safe default for API inputs
        frozen=False,                # set True to make instances immutable (hashable)
        populate_by_name=True,       # allow both alias and field name during construction
    )

    id: int
    username: Annotated[str, Field(min_length=3, max_length=50, pattern=r"^[a-z0-9_]+$")]
    email: str = Field(validation_alias="emailAddress")   # accept "emailAddress" on input
    score: float = Field(default=0.0, ge=0.0, le=100.0)
    created_at: datetime = Field(default_factory=datetime.utcnow)
    tags: list[str] = Field(default_factory=list, exclude=True)  # omitted from dumps
```

### Field constraint reference

| Param | Types | Meaning |
| --- | --- | --- |
| `gt` / `ge` | numeric | greater than / greater-or-equal |
| `lt` / `le` | numeric | less than / less-or-equal |
| `min_length` / `max_length` | str, list | length bounds |
| `pattern` | str | regex (applied to raw string, not coerced) |
| `max_digits` / `decimal_places` | Decimal | numeric precision |
| `alias` | any | field name for both input and output |
| `validation_alias` | any | field name for input only |
| `serialization_alias` | any | field name for output only |
| `default_factory` | any | callable; receives validated fields dict in v2.8+ |
| `exclude` | any | omit field from `model_dump` / `model_dump_json` |
| `description` / `examples` | any | included in JSON schema output |
| `discriminator` | union | literal key for discriminated union fast dispatch |

#### Construction and validation methods

```python
# from a dict (most common at API boundaries)
user = User.model_validate({"id": 1, "username": "alice", "emailAddress": "a@example.com"})

# from a JSON string — faster than json.loads() + model_validate()
user = User.model_validate_json('{"id":1,"username":"alice","emailAddress":"a@example.com"}')

# model_construct skips all validation — only use when you built the data yourself
user = User.model_construct(id=1, username="alice", email="a@example.com")
```

---

## Pydantic v2: Validators

### field_validator

```python
from pydantic import BaseModel, field_validator

class Product(BaseModel):
    name: str
    price: float
    sku: str

    # mode="before": receives raw input before Pydantic type coercion
    @field_validator("sku", mode="before")
    @classmethod
    def normalize_sku(cls, v: object) -> str:
        if isinstance(v, str):
            return v.strip().upper()
        return v

    # mode="after" (default): receives the already-coerced typed value
    @field_validator("price", mode="after")
    @classmethod
    def must_be_positive(cls, v: float) -> float:
        if v <= 0:
            raise ValueError("price must be positive")
        return v

    # apply one validator to multiple fields
    @field_validator("name", "sku", mode="after")
    @classmethod
    def no_empty(cls, v: str) -> str:
        if not v:
            raise ValueError("must not be empty")
        return v
```

### model_validator

```python
from typing_extensions import Self
from pydantic import BaseModel, model_validator

class PasswordForm(BaseModel):
    password: str
    password_confirm: str

    @model_validator(mode="after")
    def passwords_match(self) -> Self:
        if self.password != self.password_confirm:
            raise ValueError("passwords do not match")
        return self

class RawImport(BaseModel):
    start: int
    end: int

    @model_validator(mode="before")
    @classmethod
    def coerce_range(cls, data: object) -> object:
        # reshape before any field-level parsing
        if isinstance(data, str) and "-" in data:
            lo, hi = data.split("-", 1)
            return {"start": int(lo), "end": int(hi)}
        return data
```

### Annotated (functional) validators — preferred for reusability

```python
from typing import Annotated
from pydantic import AfterValidator, BeforeValidator, BaseModel
from pydantic_core import PydanticCustomError

def must_be_even(v: int) -> int:
    if v % 2 != 0:
        raise PydanticCustomError("not_even", "{value} is not even", {"value": v})
    return v

def coerce_to_list(v: object) -> list:
    return [v] if not isinstance(v, list) else v

EvenInt = Annotated[int, AfterValidator(must_be_even)]
CoercedList = Annotated[list[int], BeforeValidator(coerce_to_list)]

class Report(BaseModel):
    row_count: EvenInt         # reusable across many models
    flags: CoercedList          # 5 or [5] both accepted
```

**Validator execution order** within a single field annotation:

1. `BeforeValidator` — right to left
2. Pydantic internal type coercion
3. `AfterValidator` — left to right

`WrapValidator` receives a `handler` callable and can run before/after or short-circuit entirely:

```python
from pydantic import WrapValidator, ValidationError, ValidatorFunctionWrapHandler

def truncate_long_string(v: object, handler: ValidatorFunctionWrapHandler) -> str:
    try:
        return handler(v)
    except ValidationError as exc:
        if exc.errors()[0]["type"] == "string_too_long":
            return handler(str(v)[:100])
        raise

TruncatedStr = Annotated[str, Field(max_length=100), WrapValidator(truncate_long_string)]
```

### @computed_field

```python
from pydantic import BaseModel, computed_field

class Rectangle(BaseModel):
    width: float
    height: float

    @computed_field
    @property
    def area(self) -> float:
        return self.width * self.height

# area is included in model_dump() and model_dump_json() automatically
```

---

## Pydantic v2: Serialization

```python
user = User(id=1, username="alice", email="a@example.com", score=42.5)

# basic dict
user.model_dump()
# => {"id": 1, "username": "alice", "email": "a@example.com", "score": 42.5, "created_at": datetime(...)}

# exclude specific fields
user.model_dump(exclude={"created_at", "score"})

# include only specific fields
user.model_dump(include={"id", "username"})

# use serialization_alias as keys
user.model_dump(by_alias=True)

# serialize nested models as dicts vs model instances
user.model_dump(mode="python")  # nested → model instances
user.model_dump(mode="json")    # nested → JSON-compatible dicts

# JSON string — faster than model_dump() + json.dumps()
user.model_dump_json()
user.model_dump_json(by_alias=True, exclude={"tags"})
```

### Custom serializer

```python
from pydantic import field_serializer

class Event(BaseModel):
    name: str
    ts: datetime

    @field_serializer("ts")
    def serialize_ts(self, v: datetime) -> str:
        return v.isoformat()
```

---

## Pydantic v2: Performance

**TypeAdapter** — validate/serialize types that aren't BaseModel subclasses (plain lists, dicts, primitives, TypedDicts):

```python
from pydantic import TypeAdapter

# instantiate once at module level, not inside a function
list_of_ints = TypeAdapter(list[int])

def process(raw: str) -> list[int]:
    return list_of_ints.validate_json(raw)  # reuses compiled validator
```

**model_rebuild()** — required when a model has a forward reference that wasn't resolvable at class definition time:

```python
class Node(BaseModel):
    value: int
    children: list["Node"] = []

Node.model_rebuild()  # resolves the forward reference
```

**Performance hierarchy** (fastest → slowest for construction):

1. `model_construct()` — no validation, no coercion
2. `model_validate_json()` — Rust JSON parser + validator in one pass
3. `model_validate()` — Python dict → validator
4. `User(**kwargs)` — constructor + validation

### Other tips

- Replace `Sequence[T]` / `Mapping[K, V]` with `list[T]` / `dict[K, V]` when you know the concrete type — eliminates repeated `isinstance` checks.
- Prefer `TypedDict` over nested `BaseModel` for read-only intermediate structures — ~2.5x faster to validate.
- Use `FailFast` annotation (v2.8+) on large sequences when you only need the first error: `Annotated[list[int], FailFast()]`.
- Discriminated unions (`Field(discriminator="type")`) are significantly faster than bare `Union[A, B, C]` when the union is large.

---

## Pydantic + FastAPI

FastAPI reads the type annotations on path operation functions and automatically validates/coerces using Pydantic.

```python
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field, ValidationError

app = FastAPI()

class CreateUserRequest(BaseModel):
    username: str = Field(min_length=3, max_length=50)
    email: str
    age: int = Field(ge=0, le=150)

class UserResponse(BaseModel):
    id: int
    username: str
    email: str

@app.post("/users", response_model=UserResponse, status_code=201)
async def create_user(body: CreateUserRequest) -> UserResponse:
    # body is already validated; no try/except needed here
    saved = db.insert(body.model_dump())
    return UserResponse(**saved)
```

### What FastAPI does automatically

- Validates the request body against `CreateUserRequest`; returns HTTP 422 with structured error details on failure
- Filters the return value through `UserResponse` before sending — fields not in the response model are stripped
- Generates `/docs` and `/openapi.json` from the Pydantic JSON schemas

#### Dependency injection + validation

```python
from fastapi import Depends, Query

def pagination(page: int = Query(ge=1, default=1), size: int = Query(ge=1, le=100, default=20)):
    return {"page": page, "size": size}

@app.get("/items")
async def list_items(params: dict = Depends(pagination)):
    ...
```

**Accessing raw ValidationError** (custom error handling):

```python
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse

@app.exception_handler(RequestValidationError)
async def validation_exception_handler(request, exc: RequestValidationError):
    return JSONResponse(
        status_code=422,
        content={"errors": exc.errors(), "body": exc.body},
    )
```

---

## Zod: Schema Types

```typescript
import { z } from "zod";

// Primitives
z.string();
z.number();
z.boolean();
z.bigint();
z.null();
z.undefined();
z.date();

// String refinements
z.string().min(1).max(255).email();
z.string().url().uuid().regex(/^\d{4}$/);
z.string().startsWith("SK_").toLowerCase();

// Number refinements
z.number().int().positive().min(0).max(100);
z.number().finite().safe(); // excludes Infinity and numbers outside Number.MAX_SAFE_INTEGER

// Coerce — useful for form data / query params (everything arrives as string)
z.coerce.number();   // Number(input)
z.coerce.boolean();  // Boolean(input)
z.coerce.date();     // new Date(input)

// Literals and enums
z.literal("admin");
z.enum(["admin", "user", "guest"]);         // z.infer gives "admin" | "user" | "guest"
z.nativeEnum(MyTSEnum);                     // works with TypeScript enums

// Objects — all properties required by default
const UserSchema = z.object({
  id: z.number().int().positive(),
  username: z.string().min(3).max(50),
  role: z.enum(["admin", "user"]).default("user"),
  address: z.object({
    city: z.string(),
    zip: z.string().regex(/^\d{5}$/),
  }).optional(),
});

// Strict object (rejects unknown keys) and loose object (passes them through)
z.strictObject({ name: z.string() });
z.looseObject({ name: z.string() });

// Object manipulation
UserSchema.pick({ id: true, username: true });
UserSchema.omit({ address: true });
UserSchema.partial();                       // all fields optional
UserSchema.required();                      // all fields required
UserSchema.extend({ bio: z.string() });     // add fields
UserSchema.merge(OtherSchema);              // combine two object schemas

// Arrays
z.array(z.string()).min(1).max(10);
z.string().array();                         // same as above, chained style

// Tuples
z.tuple([z.string(), z.number(), z.boolean()]);
z.tuple([z.string()]).rest(z.number());     // [string, ...number[]]

// Records
z.record(z.string(), z.number());           // Record<string, number>

// Union and discriminated union
z.union([z.string(), z.number()]);

const ResultSchema = z.discriminatedUnion("status", [
  z.object({ status: z.literal("success"), data: z.string() }),
  z.object({ status: z.literal("error"), code: z.number() }),
]);

// Nullable, optional, nullish
z.string().nullable();   // string | null
z.string().optional();   // string | undefined
z.string().nullish();    // string | null | undefined

// Defaults and catch
z.string().default("anonymous");            // returns "anonymous" when input is undefined
z.number().catch(0);                        // returns 0 on any validation failure

// Readonly
z.object({ name: z.string() }).readonly();  // inferred type has readonly properties
```

---

## Zod: Transforms and Refinements

```typescript
// .transform() — convert the parsed value to a new type
const NumberFromString = z.string().transform((val) => parseInt(val, 10));
type Out = z.output<typeof NumberFromString>; // number
type In  = z.input<typeof NumberFromString>;  // string

// .refine() — custom boolean validator
const PositiveStr = z.string().refine(
  (val) => val.trim().length > 0,
  { message: "must not be blank" }
);

// Async refinement — requires parseAsync / safeParseAsync
const UniqueUsername = z.string().refine(
  async (val) => !(await db.usernameExists(val)),
  { message: "username already taken" }
);

// .superRefine() — add multiple issues with full control
const PasswordSchema = z
  .object({ password: z.string(), confirm: z.string() })
  .superRefine((data, ctx) => {
    if (data.password !== data.confirm) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["confirm"],
        message: "passwords do not match",
      });
    }
    if (data.password.length < 8) {
      ctx.addIssue({
        code: z.ZodIssueCode.too_small,
        minimum: 8,
        type: "string",
        inclusive: true,
        path: ["password"],
        message: "password must be at least 8 characters",
      });
    }
  });

// Chaining transforms and refinements — executed in declaration order
const TrimmedEmail = z
  .string()
  .transform((s) => s.trim().toLowerCase())
  .refine((s) => s.includes("@"), { message: "invalid email" });

// .pipe() — compose schemas
const CoercedId = z.string().pipe(z.coerce.number().int().positive());
```

---

## Zod: Schema Inference

```typescript
const UserSchema = z.object({
  id: z.number(),
  name: z.string(),
  createdAt: z.coerce.date(),
});

// Output type (after transforms/coercion)
type User = z.infer<typeof UserSchema>;
// => { id: number; name: string; createdAt: Date }

// When input and output differ (transforms)
const FormSchema = z.object({
  age: z.string().transform(Number),
});
type FormInput  = z.input<typeof FormSchema>;   // { age: string }
type FormOutput = z.output<typeof FormSchema>;  // { age: number }
// z.infer<> is equivalent to z.output<>

// Extracting partial schemas for reuse
type UserCreate = z.infer<typeof UserSchema.omit({ id: true })>;

// Branded types — simulate nominal typing
const UserId = z.number().int().positive().brand<"UserId">();
type UserId = z.infer<typeof UserId>;
function getUser(id: UserId) { /* TypeScript enforces the brand */ }
```

---

## Zod + tRPC / React Hook Form

### tRPC

```typescript
// server/router.ts
import { initTRPC } from "@trpc/server";
import { z } from "zod";

const t = initTRPC.create();

// Shared schemas — define once, use on both client and server
const CreateUserInput = z.object({
  username: z.string().min(3).max(50),
  email: z.string().email(),
});

const UserOutput = z.object({
  id: z.number(),
  username: z.string(),
  email: z.string(),
});

export const appRouter = t.router({
  user: t.router({
    create: t.procedure
      .input(CreateUserInput)
      .output(UserOutput)
      .mutation(async ({ input }) => {
        // input is typed as z.infer<typeof CreateUserInput>
        const user = await db.users.create(input);
        return user; // validated against UserOutput before sending
      }),

    list: t.procedure
      .input(z.object({ page: z.number().int().min(1).default(1) }))
      .query(async ({ input }) => {
        return db.users.findMany({ skip: (input.page - 1) * 20, take: 20 });
      }),
  }),
});

export type AppRouter = typeof appRouter;
```

**Input merging** — stacking `.input()` calls in middleware:

```typescript
const authedProcedure = t.procedure
  .input(z.object({ sessionToken: z.string() }))
  .use(async ({ input, next }) => {
    const user = await verifyToken(input.sessionToken);
    return next({ ctx: { user } });
  });

const updateUser = authedProcedure
  .input(z.object({ username: z.string() }))
  // input is now { sessionToken: string; username: string }
  .mutation(({ input, ctx }) => { /* ... */ });
```

### React Hook Form + Zod

```typescript
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";

const SignupSchema = z.object({
  email: z.string().email("invalid email address"),
  password: z.string().min(8, "at least 8 characters"),
  confirmPassword: z.string(),
}).refine((d) => d.password === d.confirmPassword, {
  message: "passwords do not match",
  path: ["confirmPassword"],
});

type SignupForm = z.infer<typeof SignupSchema>;

function SignupForm() {
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<SignupForm>({
    resolver: zodResolver(SignupSchema),
  });

  const onSubmit = async (data: SignupForm) => {
    await api.signup(data); // data is fully validated and typed
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <input {...register("email")} />
      {errors.email && <p>{errors.email.message}</p>}

      <input type="password" {...register("password")} />
      {errors.password && <p>{errors.password.message}</p>}

      <input type="password" {...register("confirmPassword")} />
      {errors.confirmPassword && <p>{errors.confirmPassword.message}</p>}

      <button type="submit" disabled={isSubmitting}>Sign up</button>
    </form>
  );
}
```

When transforms make input and output types differ, use the three-generic form:

```typescript
useForm<z.input<typeof Schema>, unknown, z.output<typeof Schema>>({
  resolver: zodResolver(Schema),
});
```

---

## Error Handling

### Pydantic ValidationError

```python
from pydantic import BaseModel, ValidationError, Field

class Item(BaseModel):
    name: str
    price: float = Field(gt=0)

try:
    Item(name="", price=-5)
except ValidationError as exc:
    # exc.error_count() — number of errors
    # exc.errors() — list of error dicts
    for err in exc.errors():
        print(err["loc"])     # tuple of field path, e.g. ("price",)
        print(err["type"])    # error type string, e.g. "greater_than"
        print(err["msg"])     # human-readable message
        print(err["input"])   # the value that failed
        print(err["ctx"])     # optional context dict (e.g. {"gt": 0})

    # compact JSON representation
    print(exc.json(indent=2))
```

**Custom error types** via `PydanticCustomError`:

```python
from pydantic_core import PydanticCustomError

raise PydanticCustomError(
    "invalid_sku",               # error type (used in exc.errors()[n]["type"])
    "SKU '{sku}' is not valid",  # message template
    {"sku": value},              # context — substituted into template
)
```

### Zod ZodError

```typescript
import { z, ZodError } from "zod";

const schema = z.object({ age: z.number().int().positive() });

// .parse() — throws on failure
try {
  schema.parse({ age: -1 });
} catch (err) {
  if (err instanceof ZodError) {
    for (const issue of err.issues) {
      console.log(issue.path);     // string[] — field path
      console.log(issue.code);     // ZodIssueCode enum value
      console.log(issue.message);  // human-readable
    }
    console.log(err.format());     // nested { _errors: string[] } tree
    console.log(err.flatten());    // { formErrors: string[], fieldErrors: Record<string, string[]> }
  }
}

// .safeParse() — never throws, always returns a discriminated union
const result = schema.safeParse({ age: "not a number" });
if (!result.success) {
  const fieldErrors = result.error.flatten().fieldErrors;
  // fieldErrors.age => ["Expected number, received string"]
} else {
  const age = result.data.age; // typed correctly
}

// Flat error map utility
function toFieldErrors(err: ZodError): Record<string, string> {
  return err.issues.reduce((acc, issue) => {
    const key = issue.path.join(".");
    acc[key] = issue.message;
    return acc;
  }, {} as Record<string, string>);
}
```

---

## Critical Rules / Gotchas

### Pydantic v2 migration from v1

- `@validator` → `@field_validator` (decorator signature changed: must be `@classmethod`, receives value as first arg, not `cls` and `values`)
- `@root_validator` → `@model_validator(mode="before"|"after")`
- `class Config:` → `model_config = ConfigDict(...)`
- `.dict()` → `.model_dump()`; `.json()` → `.model_dump_json()`; `.parse_obj()` → `.model_validate()`
- `orm_mode = True` → `from_attributes = True` in `ConfigDict`
- Validators that previously mutated `values` dict must now operate via `model_validator(mode="before")`
- `schema_extra` → `json_schema_extra` in `Field()` or `ConfigDict`

### Zod

- **`parse` vs `safeParse`**: Use `safeParse` at external boundaries (API responses, form input, env vars). Use `parse` only when failure should be a hard crash (startup config, fixtures in tests).
- **Transforms change the type**: After `.transform()`, `z.infer<>` gives the output type. If you need the input type (e.g., for a form), use `z.input<typeof schema>` explicitly.
- **`z.object` by default strips unknown keys** — not an error, they just disappear. Use `z.strictObject` to reject them or `z.looseObject` to keep them.
- **Async refinements require async parse**: if any `.refine()` callback is `async`, calling `.parse()` synchronously throws a `ZodError` with a special async error code. Always use `.parseAsync()` / `.safeParseAsync()` when async refinements are present.
- **Schema reuse**: define schemas once in a shared module and import them on both client and server (tRPC's whole value proposition). Don't duplicate.
- **Performance**: Zod schema construction is not free. For hot paths (e.g., per-request middleware), build schemas at module load time, not inside functions.

### Both libraries

- Never use `model_construct()` (Pydantic) or skip `.parse()` (Zod) on untrusted data. Skipping validation defeats the entire purpose.
- Document which fields use `validation_alias` (Pydantic) or `z.input<>` vs `z.output<>` (Zod) — the asymmetry bites people during serialization and client codegen.

---

## References

- [Pydantic v2 — Models](https://docs.pydantic.dev/latest/concepts/models/)
- [Pydantic v2 — Validators](https://docs.pydantic.dev/latest/concepts/validators/)
- [Pydantic v2 — Fields](https://docs.pydantic.dev/latest/concepts/fields/)
- [Pydantic v2 — Performance](https://docs.pydantic.dev/latest/concepts/performance/)
- [Pydantic v2 — Migration Guide](https://docs.pydantic.dev/latest/migration/)
- [Zod — API reference](https://zod.dev/api)
- [Zod — Basics (parse / safeParse)](https://zod.dev/basics)
- [tRPC — Input & Output Validators](https://trpc.io/docs/server/validators)
- [@hookform/resolvers — zodResolver](https://github.com/react-hook-form/resolvers)
