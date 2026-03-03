# System Architecture & Elfiee's Role

> **Strictly based on the provided architecture diagram**  
> Key clarifications applied:
> - **Baths (Synapse / “Synopath”) = AgentChannel** (they are the same thing).
> - **Ezagent = Chatroom** (the chat/IM product surface).

---

## 1. High-level mental model

This system is built around a single execution spine:

**Product Surfaces (Elfiee / Synnovator / Ezagent) → AgentChannel (Baths/Synapse) → Agent → AgentContext → Machine + FileSystem + Logs**

- **Products are interaction surfaces**: they trigger or reference Agent actions.
- **Agent is the executor**: the only component that actually performs work.
- **AgentContext is the execution environment**: where configs, credentials, sessions, files, and logs live.
- **Logs/Usage are first-class outputs** of execution.

---

## 2. Components (as shown in the diagram)

### 2.1 Product surfaces (left side)
These are user-facing applications that initiate actions via SDK/API and consume results.

- **Elfiee**: local-first workbench for task execution + skill emergence.
- **Synnovator**: publishing/showcase surface for outcomes (“proof of work”).
- **Ezagent (Chatroom)**: IM-style interaction surface for discussion + prompts around Agent behavior/results.

> In the diagram, Elfiee and Synnovator are shown under Ezagent as sibling product surfaces.

---

### 2.2 Baths (Synapse / Synopath) == AgentChannel (top spine)
**This is the unified orchestration & messaging plane for Agents.**

AgentChannel provides:
- **Agent Register (MCP + Bot)**: registering an Agent into the channel.
- **Agent Router (MCP + Bot)**: routing requests to the appropriate Agent.
- **SDK entry**: product surfaces talk into the channel via SDK (dotted path).
- **IM / Router / 3PID integration**: shown as a green box on the right (“3PID”, “IM Server”, “Router”).
- **Slash/endpoint + MessageType (Data Model)**: a standardized message interface and schema, shown as another green box on the right.

**Interpretation:**  
Baths/Synapse is not a “side pipeline”; it is the **main AgentChannel** through which all requests/messages flow.

---

### 2.3 Agent (center)
**Agent is the only executor.** It receives messages from AgentChannel and performs actions by operating on AgentContext.

Key links shown:
- From AgentChannel → Agent:
  - Agent Register (MCP+Bot)
  - Agent Router (MCP+Bot)
- From AgentContext → Agent:
  - Session (bash) flows upward (Agent uses session to execute).
- From Agent → AgentContext:
  - config + Auth flows downward (Agent obtains configs/auth to operate).

---

### 2.4 AgentContext (execution environment)
AgentContext is the **working runtime context** that Agent depends on.

It includes:
- **config + Auth** (linked to OneSystem / OneAuth on the left)
- **Session (bash)** for execution
- Connectivity and movement of context:
  - **ssh**
  - **sync**

AgentContext connects downward into:
- **Machine**: PC / Mac / ECS (a compute target)
- **FileSystem**:
  - **Credentials**: (Claude.ai / OpenAI)
  - **configs / code / files**
- **Logs**:
  - **usage**
  - **logfiles**

---

## 3. End-to-end flow (canonical execution path)

1) **User initiates an action** in a product surface (Elfiee / Synnovator / Ezagent).  
2) The product sends a request **via SDK** into **AgentChannel (Baths/Synapse)**.  
3) **AgentChannel** normalizes the request using **MessageType (Data Model)** and routes it using **Agent Router**.  
4) The target **Agent** receives the routed request.  
5) Agent reads required **config/auth** and operates within **AgentContext**.  
6) Agent performs work via **bash Session**, potentially using **ssh/sync** across **Machine** and **FileSystem**.  
7) Execution produces artifacts in **FileSystem** (configs/code/files) and produces **Logs** (usage/logfiles).  
8) Results can be surfaced back to:
   - **Elfiee** for iterative work and inspection,
   - **Synnovator** for publishing,
   - **Ezagent (Chatroom)** for discussion and coordination.

---

## 4. Elfiee’s role in this architecture (precise)

### 4.1 What Elfiee IS
Elfiee is a **local-first workbench** that helps users:
- define/organize work,
- trigger Agent execution through AgentChannel,
- manage and inspect AgentContext-relevant artifacts,
- iteratively refine tasks and extract reusable patterns (“skills”).

### 4.2 What Elfiee is NOT (in this diagram)
Elfiee is **not** the execution engine.  
Execution happens in **Agent** through **AgentContext** (bash/session, filesystem, logs).

### 4.3 Elfiee’s primary responsibilities (as a product surface)
In practical UX terms, Elfiee should provide:
- **Context setup UI**: selecting/validating machine, credentials, repo/files, configs.
- **Action triggering**: invoking Agent actions via AgentChannel (SDK → MessageType → route).
- **Progress + recovery**: showing high-level status and failure states.
- **Trace visibility**: links to logs/usage and artifacts produced in filesystem.
- **Skill emergence workflow**: capturing and organizing reusable patterns from repeated executions (grounded in files/logs).

---

## 5. Implications for writing Elfiee User Journey

When writing Elfiee’s user journey, assume:
- The user journey is fundamentally: **“Human drives Agent via AgentChannel, inside an AgentContext.”**
- Key UX moments happen at boundaries:
  1) **Before execution**: context readiness (auth/config/credentials/machine/files)
  2) **During execution**: progress, controllability, and “what is running where”
  3) **After execution**: outputs (files) + evidence (logs/usage)
  4) **Iteration**: tweak context → re-run → converge → skill crystallization
- “Offline/local-first” primarily means:
  - the user can continue organizing work and preparing context locally,
  - but execution and sync behaviors depend on how AgentContext + credentials are available.

---

## 6. Glossary (diagram-aligned)

- **Baths (Synapse / Synopath)**: the **AgentChannel** (same component).
- **AgentChannel**: orchestration plane: register + route + message model + SDK entry.
- **Agent**: executor that performs actions.
- **AgentContext**: execution environment (auth/config/session/ssh/sync + machine + filesystem + logs).
- **Ezagent**: Chatroom / IM surface.
- **Synnovator**: publishing/showcase surface.
- **OneSystem / OneAuth**: shared identity/auth providers feeding AgentContext.
- **3PID / IM Server / Router**: external integration components connected to AgentChannel.

---

## 7. What Claude Code should take as constraints

- Treat **AgentChannel (Baths/Synapse)** as the *single gateway* for cross-product requests.
- Treat **AgentContext** as the *single place where execution state and evidence live*.
- Model Elfiee as an **interaction + orchestration UI**, not as an executor.
- Always tie “skills” and “results” back to **FileSystem outputs + Logs evidence**.
