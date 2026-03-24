---
name: agent-frameworks
description: Agent framework patterns covering LangGraph (StateGraph, nodes, edges, checkpointers, human-in-the-loop) and multi-agent frameworks (CrewAI, AutoGen, Semantic Kernel — architecture, orchestration, conversation, debugging). Use when building stateful agent workflows or multi-agent systems.
domain: ai-engineering
tags: [langgraph, crewai, autogen, semantic-kernel, multi-agent, stateful-agents, human-in-the-loop, agent-frameworks]
triggers: langgraph, crewai, autogen, semantic kernel, multi-agent, stateful agent, agent framework, StateGraph, human in the loop
---


# LangGraph Stateful Agents

## When to Use

Use LangGraph when you need deterministic, inspectable control flow over agent execution — not just a chain of LLM calls.

### Use LangGraph over a simple ReAct loop when

- The task requires branching logic, cycles, or parallel node execution
- You need persistent state across multiple turns (conversation memory, long-running jobs)
- Human approval or review must interrupt and resume execution
- Execution must be fault-tolerant and resumable after failure
- You want full introspection: replay past runs, fork from a prior checkpoint (time-travel debugging)

#### Use LangGraph over CrewAI when

- You want Python-native control flow rather than a DSL/role abstraction
- You need fine-grained control over routing, state mutation, and persistence
- You are deploying to production and need checkpointing, streaming, and LangSmith tracing out of the box

#### Use LangGraph over AutoGen when

- You want a single-framework solution that integrates directly with LangChain tooling
- You need first-class async support and streaming
- Your agent topology is a directed graph, not a free-form conversation between agents

## Core Concepts

### StateGraph

`StateGraph` is the primary class. It is parameterized by a user-defined `State` TypedDict or dataclass. All nodes read from and write updates to this shared state object.

```python
from langgraph.graph import StateGraph, START, END
```

Three fundamental primitives:

- **State** — a shared data structure (snapshot of the application at any point)
- **Nodes** — Python functions that receive state and return a dict of updates
- **Edges** — determine which node executes next (static or conditional)

### State Schema with TypedDict and Annotated

Without a reducer, each update overwrites the existing value. With `Annotated`, you attach a reducer function that merges incoming updates with the existing value.

```python
from typing import Annotated
from typing_extensions import TypedDict
from operator import add

class State(TypedDict):
    foo: int                         # overwrite on update
    bar: Annotated[list[str], add]   # append on update
```

`MessagesState` is a built-in shorthand for a messages list with the `add_messages` reducer (handles deduplication by message id):

```python
from langgraph.graph import MessagesState
# equivalent to:
# class MessagesState(TypedDict):
#     messages: Annotated[list[BaseMessage], add_messages]
```

Pydantic models and dataclasses also work as the state schema.

### Nodes

A node is any callable that accepts a state dict (or typed object) and returns a dict of partial state updates. Optionally it can also accept a `RunnableConfig` (for config access) or a `Runtime` object (for context injection).

```python
from langchain_core.runnables import RunnableConfig

def my_node(state: State, config: RunnableConfig):
    thread_id = config["configurable"]["thread_id"]
    return {"foo": state["foo"] + 1}
```

### Edges

```python
from langgraph.graph import START, END

# Static edge: always go from a to b
graph.add_edge("node_a", "node_b")

# Entry point
graph.add_edge(START, "node_a")

# Terminal node
graph.add_edge("node_a", END)

# Conditional edge: routing function returns a node name
graph.add_conditional_edges("node_a", routing_function)

# Conditional with explicit mapping
graph.add_conditional_edges(
    "node_a",
    routing_function,
    {True: "node_b", False: "node_c"},
)
```

### Command for Routing + State Update Together

`Command` lets a node update state and specify the next destination in a single return value:

```python
from langgraph.types import Command
from typing import Literal

def my_node(state: State) -> Command[Literal["next_node", "other_node"]]:
    return Command(
        update={"foo": "bar"},
        goto="next_node",
    )
```

## Building a Graph

Full concrete example: define state, add nodes, add edges, compile, invoke.

```python
from typing import Annotated, Literal
from typing_extensions import TypedDict
from operator import add
from langgraph.graph import StateGraph, START, END

class State(TypedDict):
    input: str
    history: Annotated[list[str], add]
    done: bool

def step_one(state: State) -> dict:
    return {"history": ["step_one ran"], "done": False}

def step_two(state: State) -> dict:
    return {"history": ["step_two ran"], "done": True}

def route(state: State) -> Literal["step_two", "__end__"]:
    return "step_two" if not state["done"] else END

builder = StateGraph(State)
builder.add_node("step_one", step_one)
builder.add_node("step_two", step_two)
builder.add_edge(START, "step_one")
builder.add_conditional_edges("step_one", route)
builder.add_edge("step_two", END)

graph = builder.compile()
result = graph.invoke({"input": "hello", "history": [], "done": False})
print(result)
# {'input': 'hello', 'history': ['step_one ran', 'step_two ran'], 'done': True}
```

Method chaining is also supported:

```python
graph = (
    StateGraph(State)
    .add_node(step_one)
    .add_node(step_two)
    .add_edge(START, "step_one")
    .add_conditional_edges("step_one", route)
    .add_edge("step_two", END)
    .compile()
)
```

## Checkpointers

Checkpointers persist graph state after each super-step (one full round of node executions). This enables multi-turn memory, fault recovery, human-in-the-loop, and time-travel debugging.

### Available Implementations

| Checkpointer | Package | Use Case |
| --- | --- | --- |
| `InMemorySaver` | `langgraph` (built-in) | Dev/testing; state lost on process restart |
| `SqliteSaver` / `AsyncSqliteSaver` | `langgraph-checkpoint-sqlite` | Single-process production or local apps |
| `PostgresSaver` / `AsyncPostgresSaver` | `langgraph-checkpoint-postgres` | Multi-process, cloud production |
| `MongoDBSaver` | `langgraph-checkpoint-mongodb` | MongoDB-based deployments |
| `RedisSaver` | `langgraph-checkpoint-redis` | High-throughput, ephemeral persistence |

### The thread_id Pattern

`thread_id` is the primary key for checkpoint sequences. All invocations on the same `thread_id` share a conversation history. Use different `thread_id` values for different users or sessions.

```python
config = {"configurable": {"thread_id": "user-42-session-1"}}
```

### InMemorySaver

```python
from langgraph.checkpoint.memory import InMemorySaver
from langgraph.graph import StateGraph, START, END
from typing import Annotated
from typing_extensions import TypedDict
from operator import add

class State(TypedDict):
    messages: Annotated[list[str], add]

def chat_node(state: State):
    return {"messages": [f"echo: {state['messages'][-1]}"]}

builder = StateGraph(State)
builder.add_node(chat_node)
builder.add_edge(START, "chat_node")
builder.add_edge("chat_node", END)

checkpointer = InMemorySaver()
graph = builder.compile(checkpointer=checkpointer)

config = {"configurable": {"thread_id": "1"}}
graph.invoke({"messages": ["hello"]}, config)
graph.invoke({"messages": ["world"]}, config)
# state["messages"] now contains both turns
```

### SqliteSaver

Use for local or single-process production apps. Requires `pip install langgraph-checkpoint-sqlite`.

```python
import sqlite3
from langgraph.checkpoint.sqlite import SqliteSaver

checkpointer = SqliteSaver(sqlite3.connect("checkpoints.db", check_same_thread=False))
graph = builder.compile(checkpointer=checkpointer)
```

For async graphs, use `AsyncSqliteSaver`:

```python
from langgraph.checkpoint.sqlite.aio import AsyncSqliteSaver

async with AsyncSqliteSaver.from_conn_string("checkpoints.db") as checkpointer:
    graph = builder.compile(checkpointer=checkpointer)
    await graph.ainvoke({"messages": ["hi"]}, config)
```

### PostgresSaver

For production multi-process deployments. Requires `pip install langgraph-checkpoint-postgres`.

```python
from langgraph.checkpoint.postgres import PostgresSaver

DB_URI = "postgresql://user:pass@localhost:5432/mydb?sslmode=disable"
with PostgresSaver.from_conn_string(DB_URI) as checkpointer:
    graph = builder.compile(checkpointer=checkpointer)
    graph.invoke({"messages": ["hi"]}, {"configurable": {"thread_id": "1"}})
```

### Inspecting State

```python
config = {"configurable": {"thread_id": "1"}}

# Latest snapshot
snapshot = graph.get_state(config)
print(snapshot.values)      # current state dict
print(snapshot.next)        # nodes that will run next (empty tuple = done)
print(snapshot.created_at)  # ISO 8601 timestamp
print(snapshot.metadata)    # source, writes, step number

# Full history (most recent first)
history = list(graph.get_state_history(config))

# Specific checkpoint by ID
specific = graph.get_state({
    "configurable": {
        "thread_id": "1",
        "checkpoint_id": "1ef663ba-28fe-6528-8002-5a559208592c",
    }
})

# Mutate state externally (creates a new checkpoint)
graph.update_state(config, {"foo": "patched_value"})
```

## Human-in-the-Loop

### interrupt() — Dynamic Pause Inside a Node

`interrupt()` pauses execution at the call site and surfaces a payload to the caller. The graph resumes when `Command(resume=...)` is passed on the next invocation against the same `thread_id`.

```python
from langgraph.types import interrupt, Command
from langgraph.checkpoint.memory import MemorySaver
from langgraph.graph import StateGraph, START, END
from typing import Optional, Literal
from typing_extensions import TypedDict

class State(TypedDict):
    action: str
    status: Optional[Literal["approved", "rejected"]]

def approval_node(state: State) -> Command[Literal["proceed", "cancel"]]:
    decision = interrupt({
        "question": "Approve this action?",
        "details": state["action"],
    })
    return Command(goto="proceed" if decision else "cancel")

def proceed_node(state: State):
    return {"status": "approved"}

def cancel_node(state: State):
    return {"status": "rejected"}

builder = StateGraph(State)
builder.add_node("approval", approval_node)
builder.add_node("proceed", proceed_node)
builder.add_node("cancel", cancel_node)
builder.add_edge(START, "approval")
builder.add_edge("proceed", END)
builder.add_edge("cancel", END)

graph = builder.compile(checkpointer=MemorySaver())

config = {"configurable": {"thread_id": "approval-1"}}

# First invoke — hits interrupt, returns with __interrupt__ populated
result = graph.invoke({"action": "delete /prod/db", "status": None}, config)
print(result["__interrupt__"])
# [Interrupt(value={'question': 'Approve this action?', 'details': 'delete /prod/db'})]

# Human reviews and resumes
final = graph.invoke(Command(resume=True), config)
print(final["status"])  # "approved"
```

### Static Interrupts at Compile Time (for inspection/debugging)

```python
graph = builder.compile(
    checkpointer=checkpointer,
    interrupt_before=["node_a"],           # pause before these nodes run
    interrupt_after=["node_b", "node_c"],  # pause after these nodes run
)

config = {"configurable": {"thread_id": "t1"}}
graph.invoke(inputs, config=config)
# execution stops; inspect state with graph.get_state(config)

graph.invoke(None, config=config)  # resume
```

Runtime override (without recompiling):

```python
graph.invoke(
    inputs,
    interrupt_before=["node_a"],
    config=config,
)
graph.invoke(None, config=config)
```

### Multiple Concurrent Interrupts

When nodes fan out in parallel and each calls `interrupt()`, resume with a dict mapping interrupt IDs to values:

```python
result = graph.invoke({"vals": []}, config)
resume_map = {
    i.id: f"answer for {i.value}"
    for i in result["__interrupt__"]
}
final = graph.invoke(Command(resume=resume_map), config)
```

### v2 API (cleaner interrupt access)

```python
result = graph.invoke(inputs, config=config, version="v2")
# result is GraphOutput
if result.interrupts:
    print(result.interrupts[0].value)
    graph.invoke(Command(resume=True), config=config, version="v2")
```

### Critical Rules for interrupt()

1. Never wrap `interrupt()` in a bare `try/except` — the exception is how LangGraph signals the pause; catching it silently breaks the mechanism.
2. Keep `interrupt()` calls in a consistent order across runs — resumption uses index-based matching.
3. Only pass JSON-serializable values to `interrupt()` — functions and non-serializable objects cannot cross checkpoint boundaries.
4. Any code before `interrupt()` re-executes on resume — make those side effects idempotent.

## Streaming

LangGraph supports seven stream modes via `graph.stream()` and `graph.astream()`.

### Stream Modes

| Mode | What it emits |
| --- | --- |
| `values` | Full state snapshot after each step |
| `updates` | Only the dict of changes each node returned |
| `messages` | `(token_chunk, metadata)` tuples from LLM calls |
| `custom` | Arbitrary data emitted via `get_stream_writer()` |
| `checkpoints` | Checkpoint events (requires checkpointer) |
| `tasks` | Task start/finish events (requires checkpointer) |
| `debug` | Combined checkpoints + tasks with full metadata |

### values vs updates

```python
# updates — only what changed per node
for chunk in graph.stream({"topic": "ice cream"}, stream_mode="updates", version="v2"):
    if chunk["type"] == "updates":
        for node_name, state in chunk["data"].items():
            print(f"{node_name}: {state}")

# values — full state after each step
for chunk in graph.stream({"topic": "ice cream"}, stream_mode="values", version="v2"):
    if chunk["type"] == "values":
        print(chunk["data"])
```

### messages — LLM token streaming

```python
from langchain.chat_models import init_chat_model
from langgraph.graph import StateGraph, START
from typing_extensions import TypedDict

model = init_chat_model("gpt-4.1-mini")

class State(TypedDict):
    topic: str
    joke: str

def call_model(state: State):
    response = model.invoke([{"role": "user", "content": f"Joke about {state['topic']}"}])
    return {"joke": response.content}

graph = StateGraph(State).add_node(call_model).add_edge(START, "call_model").compile()

for chunk in graph.stream({"topic": "cats"}, stream_mode="messages", version="v2"):
    if chunk["type"] == "messages":
        token, metadata = chunk["data"]
        if token.content:
            print(token.content, end="", flush=True)
```

Filter tokens by node: `metadata["langgraph_node"] == "call_model"`
Filter tokens by model tag: `metadata["tags"] == ["my_tag"]`

### custom — emit arbitrary data from inside a node

```python
from langgraph.config import get_stream_writer

def my_node(state: State):
    writer = get_stream_writer()
    writer({"status": "processing record 1/100"})
    # ... work ...
    writer({"status": "processing record 100/100"})
    return {"result": "done"}

for chunk in graph.stream(inputs, stream_mode="custom", version="v2"):
    if chunk["type"] == "custom":
        print(chunk["data"]["status"])
```

### Multiple modes simultaneously

```python
for chunk in graph.stream(inputs, stream_mode=["updates", "custom"], version="v2"):
    if chunk["type"] == "updates":
        ...
    elif chunk["type"] == "custom":
        ...
```

### Subgraph streaming

Pass `subgraphs=True`. The `ns` field identifies which subgraph emitted the event — `()` for root, `("subgraph_node:<task_id>",)` for nested:

```python
for chunk in graph.stream(
    inputs,
    subgraphs=True,
    stream_mode="updates",
    version="v2",
):
    if chunk["type"] == "updates":
        print(chunk["ns"], chunk["data"])
```

### Async streaming

```python
async for chunk in graph.astream(inputs, stream_mode="messages", version="v2"):
    if chunk["type"] == "messages":
        token, _ = chunk["data"]
        print(token.content, end="", flush=True)
```

## Multi-Agent Patterns

### Subgraphs

A compiled graph can be passed directly as a node. Two communication patterns:

**Shared state keys** (use when parent and subgraph schemas overlap):

```python
from langgraph.graph import StateGraph, START, END
from typing_extensions import TypedDict

class SharedState(TypedDict):
    foo: str

def sub_node(state: SharedState):
    return {"foo": "sub: " + state["foo"]}

subgraph = (
    StateGraph(SharedState)
    .add_node(sub_node)
    .add_edge(START, "sub_node")
    .add_edge("sub_node", END)
    .compile()
)

parent = (
    StateGraph(SharedState)
    .add_node("sub", subgraph)   # compiled graph as a node
    .add_edge(START, "sub")
    .add_edge("sub", END)
    .compile()
)
```

**Different state schemas** (use a wrapper node to translate):

```python
class ParentState(TypedDict):
    query: str

class SubState(TypedDict):
    input_text: str
    result: str

def sub_node(state: SubState):
    return {"result": "processed: " + state["input_text"]}

subgraph = (
    StateGraph(SubState)
    .add_node(sub_node)
    .add_edge(START, "sub_node")
    .add_edge("sub_node", END)
    .compile()
)

def call_sub(state: ParentState):
    out = subgraph.invoke({"input_text": state["query"], "result": ""})
    return {"query": out["result"]}

parent = (
    StateGraph(ParentState)
    .add_node("sub_wrapper", call_sub)
    .add_edge(START, "sub_wrapper")
    .add_edge("sub_wrapper", END)
    .compile()
)
```

### Subgraph Persistence Modes

| Mode | Config | Behavior |
| --- | --- | --- |
| Per-invocation | `checkpointer=None` (default) | Fresh state each call; can still interrupt if parent has checkpointer |
| Per-thread | `checkpointer=True` | State accumulates across calls on same thread |
| Stateless | `checkpointer=False` | No checkpointing; plain function execution |

### Supervisor Pattern

A supervisor node routes work to specialist subgraphs. Each specialist returns to the supervisor, which decides whether to call another specialist or end.

```python
from langgraph.graph import StateGraph, MessagesState, START, END
from langgraph.types import Command
from typing import Literal

def supervisor(state: MessagesState) -> Command[Literal["researcher", "coder", "__end__"]]:
    # call an LLM to decide which specialist to invoke next
    decision = llm_router(state["messages"])
    if decision == "done":
        return Command(goto=END)
    return Command(goto=decision, update={"messages": state["messages"]})

builder = StateGraph(MessagesState)
builder.add_node("supervisor", supervisor)
builder.add_node("researcher", researcher_subgraph)
builder.add_node("coder", coder_subgraph)
builder.add_edge(START, "supervisor")
builder.add_edge("researcher", "supervisor")  # always return to supervisor
builder.add_edge("coder", "supervisor")

graph = builder.compile(checkpointer=MemorySaver())
```

### Handoff Tools

Agents can hand off to each other via tools that return `Command`:

```python
from langchain_core.tools import tool
from langgraph.types import Command

def make_handoff_tool(target_agent: str):
    @tool
    def handoff(message: str) -> Command:
        """Transfer control to another agent."""
        return Command(
            goto=target_agent,
            update={"messages": [{"role": "tool", "content": message}]},
        )
    handoff.__name__ = f"transfer_to_{target_agent}"
    return handoff
```

## Tool Calling

### ToolNode (pre-built)

`ToolNode` handles the boilerplate of iterating over tool calls in the last message, invoking each tool, and returning `ToolMessage` results. Expects `state["messages"]` to follow the LangChain message schema.

```python
from langgraph.prebuilt import ToolNode, tools_condition
from langchain_core.tools import tool

@tool
def search(query: str) -> str:
    """Search the web."""
    return f"Results for: {query}"

@tool
def calculator(expression: str) -> str:
    """Evaluate a math expression."""
    return str(eval(expression))

tools = [search, calculator]
tool_node = ToolNode(tools)
```

### tools_condition + full agent loop

`tools_condition` is a pre-built routing function: returns `"tools"` if the last message has tool calls, otherwise `END`.

```python
from langgraph.prebuilt import ToolNode, tools_condition
from langgraph.graph import StateGraph, MessagesState, START, END
from langchain.chat_models import init_chat_model

model = init_chat_model("gpt-4.1-mini")
model_with_tools = model.bind_tools(tools)

def call_model(state: MessagesState):
    response = model_with_tools.invoke(state["messages"])
    return {"messages": [response]}

builder = StateGraph(MessagesState)
builder.add_node("agent", call_model)
builder.add_node("tools", tool_node)
builder.add_edge(START, "agent")
builder.add_conditional_edges("agent", tools_condition)
builder.add_edge("tools", "agent")  # loop back after tool execution

graph = builder.compile()
```

### Manual tool routing (without ToolNode)

```python
from langchain_core.messages import ToolMessage
from typing import Literal

def tool_executor(state: MessagesState):
    tools_by_name = {t.name: t for t in tools}
    results = []
    for call in state["messages"][-1].tool_calls:
        result = tools_by_name[call["name"]].invoke(call["args"])
        results.append(ToolMessage(content=str(result), tool_call_id=call["id"]))
    return {"messages": results}

def should_continue(state: MessagesState) -> Literal["tools", "__end__"]:
    last = state["messages"][-1]
    return "tools" if last.tool_calls else END
```

## Error Handling and Retry

### RetryPolicy on nodes

Pass a `RetryPolicy` to `add_node` to automatically retry failed nodes with exponential backoff:

```python
from langgraph.pregel import RetryPolicy

builder.add_node(
    "flaky_api_node",
    flaky_api_call,
    retry=RetryPolicy(
        max_attempts=3,
        backoff_factor=2.0,    # interval doubles each attempt
        initial_interval=0.5,  # seconds before first retry
        retry_on=Exception,    # or a tuple of specific exception types
    ),
)
```

### Catching errors inside a node

For partial recovery — return a degraded result rather than crashing the graph:

```python
def safe_api_node(state: State):
    try:
        result = call_external_api(state["query"])
        return {"result": result, "error": None}
    except TimeoutError as e:
        return {"result": None, "error": str(e)}
```

### Conditional routing on error

```python
def route_on_error(state: State) -> Literal["retry_node", "error_node", "__end__"]:
    if state.get("error"):
        return "error_node"
    return END

builder.add_conditional_edges("api_node", route_on_error)
```

### GraphRecursionError

LangGraph enforces a recursion limit (default 25 steps) to prevent infinite loops. Increase via config:

```python
graph.invoke(inputs, config={"recursion_limit": 100})
```

## Critical Rules / Gotchas

**State mutation** — Nodes must return a dict of updates, not mutate `state` in place. Mutations to the state dict inside a node are not persisted.

```python
# WRONG
def bad_node(state: State):
    state["foo"] = "bar"   # silently discarded

# RIGHT
def good_node(state: State):
    return {"foo": "bar"}
```

**Reducer functions and parallel fan-out** — When multiple nodes write to the same key in a parallel fan-out, LangGraph calls the reducer with each update in sequence. Without a reducer, only the last write wins. Always use `Annotated[list[T], add]` (or a custom reducer) for keys written by multiple parallel nodes.

```python
class State(TypedDict):
    # Written by two parallel nodes — needs a reducer
    results: Annotated[list[str], add]
```

**interrupt() exception** — `interrupt()` raises an internal exception to pause the graph. Never swallow it with a bare `except Exception`. Use specific exception types when catching errors around interrupt calls:

```python
# WRONG — catches the interrupt signal
def node(state):
    try:
        val = interrupt("question")
    except Exception:
        pass

# RIGHT
def node(state):
    val = interrupt("question")
    return {"answer": val}
```

**Checkpointer required for HITL** — Both `interrupt()` and static `interrupt_before/after` require a checkpointer. Without one there is nowhere to persist state between the pause and the resume.

**thread_id uniqueness** — Each independent conversation or task run needs a unique `thread_id`. Reusing a `thread_id` continues the same session (correct for multi-turn chat; a bug if you meant to start fresh).

**Reducer called on update_state** — `graph.update_state()` runs through reducers. For an `add` reducer, `update_state(config, {"bar": ["x"]})` appends `"x"` to the existing list — it does not replace it.

**Async checkpointers for async graphs** — Use `AsyncSqliteSaver` / `AsyncPostgresSaver` with `ainvoke` / `astream`. Mixing sync checkpointers with async execution causes blocking.

**Pre-interrupt code re-executes on resume** — All code before `interrupt()` in a node runs again when the graph resumes (the node re-runs from the top). Make those side effects idempotent.

**version="v2" is opt-in** — The v2 invocation API (`GraphOutput`, unified chunk format, `result.interrupts`) must be explicitly requested with `version="v2"`. Default is v1 for backward compatibility.

## Key APIs

```python
# Build
from langgraph.graph import StateGraph, START, END, MessagesState

StateGraph(State)
builder.add_node(name, fn, retry=RetryPolicy(...))
builder.add_edge(src, dst)
builder.add_conditional_edges(src, routing_fn, {key: dst})
graph = builder.compile(
    checkpointer=checkpointer,
    interrupt_before=["node_name"],
    interrupt_after=["node_name"],
)

# Invoke / Stream
graph.invoke(inputs, config={"configurable": {"thread_id": "..."}})
graph.stream(inputs, stream_mode="updates", version="v2")
graph.astream(inputs, stream_mode="messages", version="v2")

# State inspection
graph.get_state(config)
graph.get_state_history(config)
graph.update_state(config, partial_state)

# Pre-built helpers
from langgraph.prebuilt import ToolNode, tools_condition
from langgraph.types import interrupt, Command
from langgraph.checkpoint.memory import InMemorySaver, MemorySaver
from langgraph.checkpoint.sqlite import SqliteSaver
from langgraph.checkpoint.sqlite.aio import AsyncSqliteSaver
from langgraph.checkpoint.postgres import PostgresSaver
from langgraph.config import get_stream_writer
from langgraph.pregel import RetryPolicy
```

## References

- Graph API: <https://docs.langchain.com/oss/python/langgraph/graph-api.md>
- Persistence / checkpointers: <https://docs.langchain.com/oss/python/langgraph/persistence.md>
- Interrupts / HITL: <https://docs.langchain.com/oss/python/langgraph/interrupts.md>
- Streaming: <https://docs.langchain.com/oss/python/langgraph/streaming.md>
- Subgraphs: <https://docs.langchain.com/oss/python/langgraph/use-subgraphs.md>
- Memory (short + long term): <https://docs.langchain.com/oss/python/langgraph/add-memory.md>
- Quickstart: <https://docs.langchain.com/oss/python/langgraph/quickstart.md>
- Docs index: <https://docs.langchain.com/llms.txt>

---


# Multi-Agent Frameworks: CrewAI, AutoGen, and Semantic Kernel

## Framework Identity at a Glance

| | CrewAI | AutoGen | Semantic Kernel |
| --- | --- | --- | --- |
| Core metaphor | Role-based team (crew) | Conversational agents | Middleware kernel + plugins |
| Orchestration model | Sequential or hierarchical tasks | Actor-model, event-driven messaging | Function-calling via LLM planner |
| Primary language | Python | Python (+ .NET via SK merge) | C#, Python, Java |
| Maturity signal | Fast-growing OSS, production-ready | v0.4 full redesign (2025), research-origin | v1.0 stable, Microsoft-supported |
| Best fit | Structured, role-divided workflows | Flexible, conversation-driven multi-agent | Enterprise .NET integration, stable production |
| vs LangGraph | Less control, faster setup | More dynamic, less graph control | Not graph-based; complement to LangGraph |


## AutoGen

### Mental model (v0.4)

AutoGen v0.4 is a full architectural redesign built on an **actor model**. Each agent is an actor that processes messages asynchronously. The framework has three layers:

1. **AutoGen Core** — actor model runtime, message routing, cross-language support
2. **AutoGen AgentChat** — higher-level API with pre-built agents, the layer most users work with
3. **Extensions** — third-party integrations, specialized agents

The classic `AssistantAgent`/`UserProxyAgent` pattern survives in AgentChat but now runs on the async event-driven core.

### AssistantAgent and UserProxyAgent

```python
import asyncio
from autogen_agentchat.agents import AssistantAgent, UserProxyAgent
from autogen_agentchat.teams import RoundRobinGroupChat
from autogen_ext.models.openai import OpenAIChatCompletionClient

model_client = OpenAIChatCompletionClient(model="gpt-4o")

assistant = AssistantAgent(
    name="assistant",
    model_client=model_client,
    system_message="You are a helpful AI assistant. Reply TERMINATE when the task is done.",
)

# UserProxyAgent: represents the human or a code execution environment
user_proxy = UserProxyAgent(
    name="user_proxy",
    # human_input_mode options: "ALWAYS", "NEVER", "TERMINATE"
    # "NEVER" = fully autonomous; "TERMINATE" = human reviews at stop condition
)
```

#### human_input_mode semantics

- `"ALWAYS"` — human must approve every agent message (interactive sessions)
- `"NEVER"` — fully autonomous, no human in the loop
- `"TERMINATE"` — human is only prompted when the termination condition triggers

### Conversation termination

AutoGen v0.4 uses composable `TerminationCondition` objects. They are stateful and reset automatically after each run.

```python
from autogen_agentchat.conditions import (
    MaxMessageTermination,
    TextMentionTermination,
    StopMessageTermination,
)

# Stop after 20 messages — safety net against infinite loops
max_turns = MaxMessageTermination(max_messages=20)

# Stop when an agent says "TERMINATE" — task-complete signal
text_stop = TextMentionTermination("TERMINATE")

# OR: stop at whichever comes first
termination = max_turns | text_stop

# AND: stop only when both conditions are simultaneously true (rare)
strict_termination = max_turns & text_stop
```

Always combine `MaxMessageTermination` with a semantic condition. A conversation with only a `TextMentionTermination` will run forever if the agent never says the magic word (e.g., it gets stuck in a reasoning loop).

### Tool registration

```python
from autogen_agentchat.agents import AssistantAgent
from autogen_core.tools import FunctionTool

def search_web(query: str) -> str:
    """Search the web and return a summary of results."""
    return do_search(query)

def run_python(code: str) -> str:
    """Execute Python code in a sandboxed environment and return stdout."""
    return sandbox_exec(code)

search_tool = FunctionTool(search_web, description="Search the web for current information.")
code_tool = FunctionTool(run_python, description="Run Python code.")

agent = AssistantAgent(
    name="coder",
    model_client=model_client,
    tools=[search_tool, code_tool],
    reflect_on_tool_use=True,   # agent reflects on tool output before responding
)
```

For code execution, prefer `CodeExecutorAgent` with a Docker or subprocess executor rather than a plain `FunctionTool` — it handles sandboxing, working directories, and multi-file output properly.

### Group chat patterns

```python
from autogen_agentchat.teams import RoundRobinGroupChat, SelectorGroupChat

# Round robin: each agent speaks in turn
team = RoundRobinGroupChat(
    participants=[assistant, critic, summarizer],
    termination_condition=termination,
)

# Selector: an LLM selects the next speaker based on context
team = SelectorGroupChat(
    participants=[researcher, coder, reviewer],
    model_client=model_client,   # selector LLM
    termination_condition=termination,
)

async def main():
    result = await team.run(task="Build a sentiment analysis pipeline for Twitter data.")
    print(result.messages[-1].content)

asyncio.run(main())
```

`SelectorGroupChat` is more flexible but costs more (extra LLM call per turn to select next speaker). Use `RoundRobinGroupChat` when the turn order is predictable.

### AutoGen Studio

For rapid prototyping and debugging: AutoGen Studio provides a drag-and-drop UI with live execution visualization. Use it to test agent configurations before encoding them in code. Export the resulting config as Python for production.


## Framework Selection Guide

### Choose CrewAI when

- The workflow maps naturally to a team of specialists with distinct roles
- You need fastest time-to-prototype (YAML config, minimal boilerplate)
- The task sequence is mostly predictable (sequential process)
- Your team is new to agentic AI — the role/goal/backstory model is intuitive
- You want to mix deterministic logic with LLM intelligence via Flows

### Choose AutoGen when

- You need flexible, dynamic conversation between agents where turn order isn't predetermined
- Human-in-the-loop is a first-class requirement (approve intermediate results)
- You're building research/experimentation systems that will evolve rapidly
- You need agents that write and execute code as part of their workflow
- Cross-language agent collaboration matters (Core layer supports non-Python agents)

### Choose Semantic Kernel when

- You're building in .NET/C# or need Java support
- The primary goal is integrating LLM reasoning into an existing enterprise application (not building a new agent system from scratch)
- You need Microsoft support SLAs and stability guarantees
- You're already in the Microsoft ecosystem (Azure, Copilot, Microsoft 365)
- The workflow is better described as "application with AI features" than "multi-agent system"

### Choose LangGraph when

- You need precise, explicit control over agent state and execution flow
- The workflow has complex branching, cycles, and conditional routing
- You need production-grade reliability with full state persistence and checkpointing
- You're comfortable with graph-based thinking and want the control that comes with it
- CrewAI or AutoGen produced unpredictable behavior that you need to constrain

### Avoid using any of these when

- A single well-prompted LLM call solves the problem — orchestration overhead isn't free
- You need sub-100ms latency — agent loops add at minimum 2-3 LLM round-trips
- The workflow is fully deterministic — use regular code


## Production Checklist

- [ ] Termination conditions have a hard message-count ceiling (no termination = infinite spend)
- [ ] LLM calls are logged with cost tracking
- [ ] Tool functions are idempotent or clearly guarded against double-execution
- [ ] Secrets are injected via DI/env vars, not passed through agent prompts
- [ ] `verbose=True` / debug logging is disabled or gated by log level
- [ ] Agent `max_iter` / `max_consecutive_tool_calls` set explicitly
- [ ] Rate limits (`max_rpm` in CrewAI, request throttling in AutoGen) are configured
- [ ] System tested with `temperature=0` before enabling higher temperatures
- [ ] Memory/state store backed by a durable store in production (not in-memory default)
- [ ] Human-in-the-loop checkpoints defined for any irreversible actions (send email, execute trade, delete data)
