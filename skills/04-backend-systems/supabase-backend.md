---
name: supabase-backend
description: >
  Production patterns for Supabase as a backend-as-a-service: Row Level Security
  policies, auth provider configuration and JWT handling, storage bucket access
  control, edge functions with Deno, realtime subscriptions, database functions
  and triggers, local development with the Supabase CLI, migration workflow, and
  the most common security mistakes that lead to data exposure.
domain: backend
category: baas
tags: [Supabase, PostgreSQL, RLS, auth, storage, edge-functions, realtime, Deno, migrations, multi-tenant]
triggers:
  - supabase
  - row level security
  - RLS
  - supabase auth
  - supabase storage
  - edge functions supabase
  - supabase realtime
  - supabase migrations
  - supabase CLI
  - postgres policies
---

# Supabase Backend Patterns

## 1. Row Level Security (RLS)

### The non-negotiable baseline

Enable RLS on every table in the `public` schema immediately after creation — no exceptions. An exposed table without RLS is readable and writable by anyone who holds the anon key.

```sql
ALTER TABLE public.documents ENABLE ROW LEVEL SECURITY;
```

Automate this with an event trigger so newly created tables never ship unprotected:

```sql
CREATE OR REPLACE FUNCTION enforce_rls_on_new_tables()
RETURNS event_trigger LANGUAGE plpgsql AS $$
DECLARE
  obj record;
BEGIN
  FOR obj IN SELECT * FROM pg_event_trigger_ddl_commands()
  LOOP
    IF obj.object_type = 'table' AND obj.schema_name = 'public' THEN
      EXECUTE format('ALTER TABLE %I.%I ENABLE ROW LEVEL SECURITY',
                     obj.schema_name, obj.object_identity);
    END IF;
  END LOOP;
END;
$$;

CREATE EVENT TRIGGER ensure_rls
ON ddl_command_end
WHEN TAG IN ('CREATE TABLE', 'CREATE TABLE AS', 'SELECT INTO')
EXECUTE FUNCTION enforce_rls_on_new_tables();
```

### Policy clauses

| Operation | USING clause | WITH CHECK clause |
| ----------- | ------------- | ------------------- |
| SELECT | yes | no |
| INSERT | no | yes |
| UPDATE | yes (existing row filter) | yes (new value validation) |
| DELETE | yes | no |

UPDATE always needs both. A missing SELECT policy means UPDATE silently affects zero rows.

### Ownership pattern (single-user tables)

```sql
-- SELECT
CREATE POLICY "owner can read"
ON public.notes FOR SELECT TO authenticated
USING ((SELECT auth.uid()) = user_id);

-- INSERT
CREATE POLICY "owner can insert"
ON public.notes FOR INSERT TO authenticated
WITH CHECK ((SELECT auth.uid()) = user_id);

-- UPDATE
CREATE POLICY "owner can update"
ON public.notes FOR UPDATE TO authenticated
USING ((SELECT auth.uid()) = user_id)
WITH CHECK ((SELECT auth.uid()) = user_id);

-- DELETE
CREATE POLICY "owner can delete"
ON public.notes FOR DELETE TO authenticated
USING ((SELECT auth.uid()) = user_id);
```

Always specify `TO authenticated` (or `TO anon`) in the role target. Without it, Postgres evaluates the policy for every role including `anon`, wasting plan time.

### Multi-tenant / team access pattern

```sql
CREATE TABLE public.accounts (
  id   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name text NOT NULL
);

CREATE TABLE public.account_members (
  account_id  uuid REFERENCES public.accounts(id) ON DELETE CASCADE,
  user_id     uuid REFERENCES auth.users(id) ON DELETE CASCADE,
  role        text NOT NULL DEFAULT 'member',
  PRIMARY KEY (account_id, user_id)
);

-- Helper: is the current user a member of this account?
CREATE FUNCTION public.is_account_member(p_account_id uuid)
RETURNS boolean LANGUAGE sql SECURITY DEFINER STABLE AS $$
  SELECT EXISTS (
    SELECT 1 FROM public.account_members
    WHERE account_id = p_account_id
      AND user_id = (SELECT auth.uid())
  );
$$;

-- Policy using the helper
CREATE POLICY "members can read account data"
ON public.projects FOR SELECT TO authenticated
USING (public.is_account_member(account_id));
```

Use `SECURITY DEFINER` on the helper function so the policy itself does not need to cross-join against the memberships table on every row — the function result is evaluated once per query block via the `(SELECT ...)` caching trick.

### Permission-based role system

```sql
CREATE TYPE public.app_permission AS ENUM (
  'members.manage', 'billing.manage', 'content.publish'
);

CREATE TABLE public.role_permissions (
  role       text NOT NULL,
  permission public.app_permission NOT NULL,
  PRIMARY KEY (role, permission)
);

CREATE FUNCTION public.has_permission(
  p_account_id uuid,
  p_permission public.app_permission
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STABLE AS $$
BEGIN
  RETURN EXISTS (
    SELECT 1
    FROM public.account_members m
    JOIN public.role_permissions rp ON m.role = rp.role
    WHERE m.user_id     = (SELECT auth.uid())
      AND m.account_id  = p_account_id
      AND rp.permission = p_permission
  );
END;
$$;
```

### Performance rules

1. **Wrap auth functions in a subselect.** `auth.uid()` called bare is evaluated per row. `(SELECT auth.uid())` is evaluated once and cached by the planner.

   ```sql
   -- slow
   USING (auth.uid() = user_id)

   -- fast
   USING ((SELECT auth.uid()) = user_id)
   ```

2. **Index every column referenced in a policy.**

   ```sql
   CREATE INDEX ix_projects_account_id ON public.projects(account_id);
   CREATE INDEX ix_account_members_user  ON public.account_members(user_id);
   ```

3. **Push explicit filters from the client.** Even though RLS adds an implicit WHERE, handing the planner an explicit `.eq('account_id', id)` from the SDK helps it choose the right index.

4. **Use security-definer functions for join-heavy checks.** Inline subqueries that touch 3+ tables in a policy are a red flag; extract them into a `SECURITY DEFINER STABLE` function.

### Testing RLS

The SQL editor in the Supabase dashboard runs as the `postgres` superuser and bypasses RLS. Always test via the JS/TS SDK or by impersonating roles in a test suite.

Use `pgTap` with the `basejump-supabase_test_helpers` extension:

```sql
BEGIN;
SELECT plan(4);

SELECT tests.create_supabase_user('alice', 'alice@test.com');
SELECT tests.create_supabase_user('bob',   'bob@test.com');

-- Alice can see her own rows
SELECT tests.authenticate_as('alice');
SELECT isnt_empty(
  $$ SELECT * FROM public.notes WHERE user_id = tests.get_supabase_uid('alice') $$,
  'alice sees her notes'
);

-- Bob cannot see Alice's rows
SELECT tests.authenticate_as('bob');
SELECT is_empty(
  $$ SELECT * FROM public.notes WHERE user_id = tests.get_supabase_uid('alice') $$,
  'bob cannot see alice notes'
);

SELECT * FROM finish();
ROLLBACK;
```

RLS SELECT/UPDATE/DELETE failures return empty results, not errors. Test for `is_empty()` on negative cases, not for thrown errors.

---

## 2. Authentication

### Provider configuration

Supabase Auth supports: email+password, magic links, OTP, OAuth (Google, GitHub, Apple, etc.), SAML SSO, and phone OTP. Providers are configured in `supabase/config.toml` for local dev, and in the Auth section of the dashboard for production.

```toml
# supabase/config.toml
[auth.external.google]
enabled = true
client_id = "env(GOOGLE_CLIENT_ID)"
secret = "env(GOOGLE_CLIENT_SECRET)"
redirect_uri = "https://yourproject.supabase.co/auth/v1/callback"
```

### JWT claims and session management

Supabase now uses asymmetric JWT signing keys (replaces the legacy symmetric secret). The JWT payload exposes two claim namespaces:

- `raw_app_meta_data` — set server-side only (service role or database trigger). Safe to use in RLS policies.
- `raw_user_meta_data` — user-editable from the client. **Never trust this for authorization.**

```sql
-- SAFE: app_metadata is server-controlled
USING ((auth.jwt() -> 'app_metadata' ->> 'role') = 'admin')

-- DANGEROUS: user_metadata is client-editable
USING ((auth.jwt() -> 'user_metadata' ->> 'role') = 'admin')
```

JWT contents are stale until the token is refreshed. If you remove a user from a team and update `app_metadata`, that change is not visible in RLS policies until the user refreshes their session. For real-time access revocation, check against the database table, not the JWT.

### Linking auth users to application data

```sql
-- profiles table auto-populated on signup
CREATE TABLE public.profiles (
  id         uuid PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
  username   text UNIQUE,
  avatar_url text
);

CREATE FUNCTION public.handle_new_user()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
  INSERT INTO public.profiles (id)
  VALUES (NEW.id);
  RETURN NEW;
END;
$$;

CREATE TRIGGER on_auth_user_created
  AFTER INSERT ON auth.users
  FOR EACH ROW EXECUTE FUNCTION public.handle_new_user();
```

### Key types

| Key | Use | Bypasses RLS? |
| ----- | ----- | --------------- |
| `anon` / publishable key | Client-side, public requests | No |
| `service_role` / server key | Server-side admin operations | Yes |

Never embed the `service_role` key in frontend code, environment variables shipped to the browser, or any public repository. It grants unrestricted database access.

---

## 3. Storage

### Bucket model

Buckets are top-level containers. Each bucket is either **public** (objects served without auth) or **private** (requires an RLS-authorized signed URL or bearer token). Public buckets still respect RLS for uploads and deletes — only the download URL is unauthenticated.

```sql
-- RLS on storage.objects controls all operations
-- bucket_id, name (full path), and owner_id are the key columns

-- Allow authenticated users to upload to their own folder
CREATE POLICY "user owns folder"
ON storage.objects FOR INSERT TO authenticated
WITH CHECK (
  bucket_id = 'user-uploads'
  AND (storage.foldername(name))[1] = (SELECT auth.uid())::text
);

-- Allow users to read their own files
CREATE POLICY "user reads own files"
ON storage.objects FOR SELECT TO authenticated
USING (
  bucket_id = 'user-uploads'
  AND owner_id = (SELECT auth.uid())
);

-- Allow users to delete their own files
CREATE POLICY "user deletes own files"
ON storage.objects FOR DELETE TO authenticated
USING (
  bucket_id = 'user-uploads'
  AND owner_id = (SELECT auth.uid())
);
```

### Storage helper functions

| Function | Returns |
| ---------- | --------- |
| `storage.filename(name)` | Filename with extension |
| `storage.extension(name)` | Extension only |
| `storage.foldername(name)` | Array of path segments |

### Signed URLs vs public URLs

- Use `createSignedUrl()` for private objects — URLs expire and require no persistent auth.
- Use `getPublicUrl()` only for genuinely public assets. Once a bucket is public, the URL is always accessible regardless of who calls it.

### Upsert requires three permissions

Overwriting an existing file requires `INSERT` + `SELECT` + `UPDATE` policies. Missing any one causes a silent failure or permission error.

---

## 4. Edge Functions (Deno)

### Structure

Edge functions run on Deno under the Supabase Edge Runtime, distributed close to users. Each function is a file at `supabase/functions/<function-name>/index.ts`.

```ts
import { createClient } from 'jsr:@supabase/supabase-js@2'

Deno.serve(async (req: Request) => {
  // Always use env vars — never hardcode URLs
  const supabaseUrl  = Deno.env.get('SUPABASE_URL')!
  const serviceKey   = Deno.env.get('SUPABASE_SERVICE_ROLE_KEY')!
  const anonKey      = Deno.env.get('SUPABASE_ANON_KEY')!

  // Use user's JWT to forward their auth context (respects RLS)
  const authHeader = req.headers.get('Authorization')
  const userClient = createClient(supabaseUrl, anonKey, {
    global: { headers: { Authorization: authHeader ?? '' } }
  })

  // Use service client only for operations that must bypass RLS
  const adminClient = createClient(supabaseUrl, serviceKey)

  // ... business logic
  return new Response(JSON.stringify({ ok: true }), {
    headers: { 'Content-Type': 'application/json' }
  })
})
```

### Auth verification in edge functions

Supabase now requires explicit auth verification — the runtime no longer auto-validates JWTs:

```ts
import { createClient } from 'jsr:@supabase/supabase-js@2'

Deno.serve(async (req) => {
  const supabase = createClient(
    Deno.env.get('SUPABASE_URL')!,
    Deno.env.get('SUPABASE_ANON_KEY')!,
    { global: { headers: { Authorization: req.headers.get('Authorization') ?? '' } } }
  )

  const { data, error } = await supabase.auth.getClaims(
    req.headers.get('Authorization')?.replace('Bearer ', '') ?? ''
  )

  if (error || !data?.claims?.sub) {
    return new Response(JSON.stringify({ error: 'Unauthorized' }), { status: 401 })
  }

  const userId = data.claims.sub
  // proceed with userId
})
```

For custom JWT providers or asymmetric verification, use the `jose` library:

```ts
import * as jose from 'https://deno.land/x/jose@v5.2.0/index.ts'

const JWKS = jose.createRemoteJWKSet(
  new URL(`${Deno.env.get('SUPABASE_URL')!}/auth/v1/.well-known/jwks.json`)
)

const { payload } = await jose.jwtVerify(token, JWKS)
```

### Accessing the database directly

Edge functions can bypass PostgREST and use raw SQL via `deno-postgres` for transactions:

```ts
import { Pool } from 'https://deno.land/x/postgres@v0.17.0/mod.ts'

const pool = new Pool(Deno.env.get('SUPABASE_DB_URL'), 3, true)

const client = await pool.connect()
try {
  await client.queryObject('BEGIN')
  await client.queryObject(
    `INSERT INTO orders (user_id, total) VALUES ($1, $2)`,
    [userId, total]
  )
  await client.queryObject('COMMIT')
} catch (e) {
  await client.queryObject('ROLLBACK')
  throw e
} finally {
  client.release()
}
```

### Deployment

```bash
supabase functions deploy my-function
supabase secrets set MY_SECRET=value   # inject env vars
```

### Rate limiting and fan-out

Outbound `fetch()` calls from edge functions to other edge functions within the same project count against rate limits — this includes direct recursion, chaining, and fan-out patterns. Design accordingly.

---

## 5. Realtime

Supabase Realtime has three distinct channels:

| Channel type | Use case | Auth-aware? |
| --- | --- | --- |
| **Postgres Changes** | Listen to INSERT/UPDATE/DELETE on tables | Yes — respects RLS |
| **Broadcast** | Low-latency client-to-client messages | No server persistence |
| **Presence** | Synchronized online state across clients | No server persistence |

### Postgres Changes subscription

```ts
const channel = supabase
  .channel('public:messages')
  .on(
    'postgres_changes',
    { event: '*', schema: 'public', table: 'messages', filter: `room_id=eq.${roomId}` },
    (payload) => console.log('Change:', payload)
  )
  .subscribe()

// Cleanup
supabase.removeChannel(channel)
```

RLS applies to Postgres Changes. If the subscribing user does not have SELECT access to a row, that row's change event will not be delivered to them.

### Broadcast (client-to-client)

```ts
const channel = supabase.channel('cursor-room')

channel
  .on('broadcast', { event: 'cursor' }, ({ payload }) => {
    updateCursor(payload.x, payload.y)
  })
  .subscribe()

// Send
channel.send({ type: 'broadcast', event: 'cursor', payload: { x: 100, y: 200 } })
```

### Presence

```ts
const channel = supabase.channel('online-users', {
  config: { presence: { key: userId } }
})

channel
  .on('presence', { event: 'sync' }, () => {
    const state = channel.presenceState()
    console.log('Online users:', Object.keys(state))
  })
  .subscribe(async (status) => {
    if (status === 'SUBSCRIBED') {
      await channel.track({ user: userId, online_at: new Date().toISOString() })
    }
  })
```

---

## 6. Database Functions and Triggers

### Security definer vs security invoker

- **SECURITY DEFINER** — function runs with the privileges of the function owner (typically `postgres`). Use for operations that need elevated access, but be precise — callers inherit all definer privileges for the duration of the call.
- **SECURITY INVOKER** (default) — function runs with caller's privileges, respects RLS.

Always set `search_path` explicitly on security-definer functions to prevent search path injection:

```sql
CREATE FUNCTION public.get_account_role(p_account_id uuid)
RETURNS text
LANGUAGE sql SECURITY DEFINER STABLE
SET search_path = public, pg_temp AS $$
  SELECT role FROM public.account_members
  WHERE account_id = p_account_id
    AND user_id = (SELECT auth.uid());
$$;
```

### Audit trail trigger pattern

```sql
CREATE TABLE public.audit_log (
  id          bigserial PRIMARY KEY,
  table_name  text      NOT NULL,
  record_id   uuid      NOT NULL,
  operation   text      NOT NULL,
  old_data    jsonb,
  new_data    jsonb,
  changed_by  uuid      REFERENCES auth.users(id),
  changed_at  timestamptz NOT NULL DEFAULT now()
);

CREATE FUNCTION public.record_audit()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
  INSERT INTO public.audit_log (table_name, record_id, operation, old_data, new_data, changed_by)
  VALUES (
    TG_TABLE_NAME,
    COALESCE(NEW.id, OLD.id),
    TG_OP,
    CASE WHEN TG_OP = 'DELETE' THEN to_jsonb(OLD) ELSE NULL END,
    CASE WHEN TG_OP != 'DELETE' THEN to_jsonb(NEW) ELSE NULL END,
    (SELECT auth.uid())
  );
  RETURN COALESCE(NEW, OLD);
END;
$$;
```

---

## 7. Supabase CLI and Local Development

### Initial setup

```bash
npm install -g supabase
supabase login
supabase init          # creates supabase/ directory in project root
supabase start         # starts local Postgres, Auth, Storage, Edge Runtime via Docker
```

### Migration workflow

```bash
# Create a new migration file
supabase migration new add_projects_table

# Edit supabase/migrations/<timestamp>_add_projects_table.sql
# then apply it locally
supabase db reset      # resets local DB, replays all migrations, runs seed.sql

# Generate a migration from dashboard changes (schema diff)
supabase db diff --schema public -f my_changes
```

### Linking to remote and deploying

```bash
supabase link --project-ref <project-ref-from-dashboard>
supabase db pull                          # sync remote schema to local
supabase db push                          # apply local migrations to remote
supabase functions deploy my-function     # deploy edge function
```

### Seed data

Place INSERT statements in `supabase/seed.sql`. `supabase db reset` replays migrations then seeds. Keep seed data minimal and deterministic — use fixed UUIDs so foreign key relationships stay consistent across resets.

### Config file structure

```text
supabase/
  config.toml          # local project settings (auth providers, storage, etc.)
  seed.sql             # local seed data
  migrations/
    20240101_init.sql
    20240215_add_rls.sql
  functions/
    my-function/
      index.ts
```

---

## 8. Common Security Pitfalls

### 1. Exposing the service role key on the client

The `SUPABASE_SERVICE_ROLE_KEY` bypasses all RLS. It belongs only in server-side environments (edge functions, backend APIs, CI). If it is ever in a `.env` file committed to a public repo, or shipped in a frontend bundle, rotate it immediately from the Supabase dashboard.

### 2. Trusting user_metadata in policies

`raw_user_meta_data` is writable by the user via `supabase.auth.updateUser()`. Any RLS policy or application logic that reads from it for authorization can be bypassed. Use `raw_app_meta_data` (service role or trigger only) or a lookup against a database table.

### 3. Views bypassing RLS

By default, views created under the `postgres` role ignore RLS even when accessed by a less-privileged role. In PostgreSQL 15+, use `security_invoker`:

```sql
CREATE VIEW public.my_view
WITH (security_invoker = true)
AS SELECT * FROM public.sensitive_table;
```

### 4. Missing UPDATE + SELECT policy pair

An UPDATE policy without a corresponding SELECT policy will silently succeed (no error) but update zero rows, causing hard-to-debug phantom failures.

### 5. Stale JWT claims for access revocation

Removing a user from a team and updating their `app_metadata` takes effect only after their JWT expires and refreshes (default 1-hour expiry). For immediate revocation, combine JWT claims with a live database membership check inside your policy or edge function.

### 6. anon role has too many permissions

Check that your RLS policies explicitly target `TO authenticated` where appropriate. A policy without a role target applies to `anon` as well. Unauthenticated users should generally be limited to pre-approved SELECT on public-facing content only.

### 7. No RLS coverage audit before launch

Run this query to catch unprotected tables before shipping:

```sql
SELECT schemaname, tablename, rowsecurity
FROM pg_tables
WHERE schemaname = 'public'
  AND rowsecurity = false;
```

Any row returned is a table exposed to full access from the anon key.

### 8. Functions without explicit search_path

A `SECURITY DEFINER` function without `SET search_path = public, pg_temp` can be exploited by a user who creates an object in a schema that appears earlier in the default search path, hijacking the function's resolution.

---

## 9. Pre-deployment Checklist

- [ ] RLS enabled on all `public` schema tables
- [ ] SELECT, INSERT, UPDATE, DELETE policies defined for every table
- [ ] All policies specify `TO authenticated` or `TO anon` explicitly
- [ ] `auth.uid()` always wrapped in `(SELECT auth.uid())`
- [ ] Columns used in policies are indexed
- [ ] No `raw_user_meta_data` used for authorization
- [ ] No service role key in frontend code or committed `.env` files
- [ ] Views use `security_invoker = true`
- [ ] Security-definer functions have explicit `SET search_path`
- [ ] Negative-case RLS tests written with pgTap
- [ ] Migration files cover all schema and policy changes
- [ ] `supabase db push` dry-run reviewed before production deployment
