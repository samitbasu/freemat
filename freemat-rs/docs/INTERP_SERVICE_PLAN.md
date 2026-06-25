# Interpreter-as-Service — Design Plan (REPL + DAP, nested debugger REPL)

Status: **design draft.** This document is the roadmap for turning the FreeMat-rs interpreter
into a single-threaded **service (actor)** with multiple clients — today the terminal REPL and
a Debug Adapter Protocol (DAP) server, tomorrow potentially an LSP / notebook / remote driver.
It supersedes the current model where `fm-dap` spins up its own throwaway interpreter per
session. Background for the DAP wire layer lives next to the code in `crates/fm-dap`; the
interpreter debug seam is `crates/fm-interp/src/debug.rs`.

Roll high-level status into `PROGRESS.md` as phases land.

## Goal

One interpreter, owned by one thread, driven by messages. Two concurrent clients:

1. **The REPL** (the `fm` CLI) — sends lines to evaluate, gets output back.
2. **The DAP server** — sets breakpoints, steps, inspects/mutates variables, and (the headline
   feature) opens a **nested debugger REPL** when stopped at a breakpoint, where typed commands
   execute *in the paused frame's live context* (MATLAB's `K>>` mode), with breakpoints still
   live in code called from that prompt (recursive debugging).

The win is structural: once the interpreter is a service, the *N*-th client is cheap, and the
two hard constraints that block "attach a debugger to my live REPL session" dissolve.

## Why an actor (and not `Arc<Mutex<Interpreter>>`)

- **`Interpreter` is `!Send`** (the `debugger` field is `Box<dyn DebugHook>` with no `Send`
  bound; the engine also leans on `Rc`-friendly internals). In the actor model it **never
  crosses a thread** — only `Command`/`Event` *messages* do, and those carry owned/formatted
  data (`String`, `serde_json::Value`, formatted variable rows), never `Array` handles or
  interpreter internals. So nothing `!Send` leaks across a channel.
- **Single writer ⇒ no locks, no races.** A shared `Mutex<Interpreter>` would have to be held
  across an entire `run()` — blocking every other client for the whole execution. The actor has
  exactly one mutator, so `evaluate` with side effects (`x = 5` in the debug console) is
  race-free for free.
- **Blocking `readline` stops mattering.** The REPL thread parks in `rustyline::readline()`; the
  engine thread parks on its inbox. Neither blocks the other.

## Thread map

```
REPL/CLI thread ──ReplCommand──▶┐
                                 ├─▶ Engine thread (owns Interpreter; single writer)
DAP socket-IO thread ─Control──▶┘        │   └─ at the seam: paused loop selects BOTH inboxes
        ▲                                 │
        └────────── Event ◀───────────────┘    (Stopped / Continued / Output / Terminated)
```

Three threads: REPL, engine actor, DAP socket I/O. **No mutex on interpreter state anywhere.**
The DAP socket-IO thread does the blocking `read_message`/`write_message` framing and is a pure
translator: socket → `Control` messages, `Event` → socket writes.

## Message taxonomy

Two inbound channels into the engine (kept separate to avoid head-of-line blocking — a queued
`Eval` must never sit in front of a `Continue` the engine needs in order to *reach* that
`Eval`), plus one outbound event channel:

```rust
// REPL channel (from the CLI; also serviced by the paused loop — see Nested REPL)
enum ReplCommand {
    Eval { source: String, reply: Sender<EvalOutcome> },   // run a line
    Query(Query),                                           // e.g. completion: function names
    Shutdown,
}

// Control channel (from the DAP socket-IO thread)
enum Control {
    SetBreakpoints(Vec<Line>),          // also mirrored into shared state (see below)
    Continue, StepIn, StepOver, StepOut,
    StackTrace { reply: Sender<Vec<Frame>> },
    Variables  { frame: FrameId, reply: Sender<Vec<VarRow>> },
    Evaluate   { frame: FrameId, expr: String, reply: Sender<EvalOutcome> },
    Pause,                              // async "stop now" (see attention flag)
    Disconnect,
}

// Outbound, engine → DAP socket-IO thread (server-push; NOT request/reply)
enum Event { Stopped { reason, level }, Continued, Output(String), Terminated, Exited(i32) }
```

Shared state (read on the hot path, writable any time):

- `breakpoints: Arc<Mutex<HashSet<Line>>>` (or arc-swap) — **breakpoint detection is local state,
  not a message.** The seam reads this every statement; the DAP thread writes it, so live
  breakpoint edits mid-run "just work" without draining a channel.
- `attention: Arc<AtomicBool>` — a cheap interrupt signal. The seam does a relaxed atomic load
  per statement (≈ free); only when set does it take the slow path and drain `Control` to learn
  *what* is wanted (pause vs terminate). This keeps the per-statement hot path from paying a
  `try_recv`.

## Engine state machine

`Idle → Running → Paused`, where **Paused is a stack** (nested breakpoints / nested `K>>`).

- **Idle** (outer dispatch loop): block-`recv` on both inboxes. `Eval` → run at top level. Trivial
  inspection (`threads`, base-frame `stackTrace`) answerable here; real inspection happens paused.
- **Running** (inside `interp.run()`): the per-statement seam fires. While running we **poll**
  (via the attention flag), never block, and **do not read the REPL channel** — we're mid-`Eval`;
  a new top-level `Eval` can't run now, so it waits in the queue.
- **Paused** (seam entered a stop): a **blocking** loop that `select!`s over **both** inboxes:
  - `Control::Continue|Step*` → return from the seam (resume); pops one pause level.
  - `Control::StackTrace|Variables|Evaluate` → answer directly against `&mut self.interp`.
  - `ReplCommand::Eval` → **run in place against the current top scope** (the nested REPL).

## The seam changes (`fm-interp`)

The existing seam (`exec_statement` → `debug_check` → `DebugHook::on_statement`) is the right
boundary and stays. Three changes are needed to support a *nested* REPL with *recursive*
breakpoints:

1. **Drop `take()`-based exclusivity; make consultation re-entrant.** Today `debug_check` does
   `self.debugger.take()`, which removes the hook for the duration of the call — so breakpoints
   do **not** fire in code run from the paused state. That blunt suppression is only correct for
   watch/hover `evaluate`. Replace it so the hook stays installed and the seam can be entered
   recursively (channel `recv` needs only a shared borrow; any hook bookkeeping goes behind
   `Cell`/`RefCell` with non-overlapping borrows, or moves into the engine).
2. **Add an explicit `suppress_breakpoints` guard** on the interpreter, set *only* around
   watch/hover evaluation. The seam checks it first and runs clean (no recursion, no UI hang).
3. **Track a pause-level depth** so `Stopped` events and the protocol know the nesting level and
   `Continue`/`Step` act on the **innermost** level (`dbcont` semantics: popping a level resumes
   the command that opened it, eventually the outer program).

No other `fm-interp` API changes: a nested REPL line is just `interp.run(line)`, which runs a
script-style line **in the current top scope without pushing a frame** — and the paused frame is
the current top scope, so `y`, `y = 5`, `disp(z)`, `dbstack` read/mutate the live suspended state
for real. The recursion through the seam is the normal call path.

## Nested debugger REPL — the headline behavior

When stopped, the paused loop services the REPL channel too:

```text
// paused, on the engine thread, holding &mut self.interp:
loop {
    select {
        Control(Continue|Step) => return,                     // resume outer program
        Control(Inspect)       => answer(&mut self.interp),   // stackTrace/variables/evaluate
        ReplCommand(Eval(line))=> reply(self.interp.run(&line)), // ← nested K>>, in current frame
    }
}
```

Policy decisions to pin down (defaults shown):

- **Breakpoints fire in code called from the nested prompt** (MATLAB behavior; enabled by the
  re-entrant seam). A deeper hit pushes another pause level. *Off* would just set the suppress
  flag around the nested run.
- **Output routing:** a nested command's echo returns as the `Eval` reply **and** is mirrored as
  a DAP `Output` event when a client is attached — one `take_output()` buffer, two sinks.
- **REPL lines typed during a *free* (non-paused) run queue** until the next stop (the outer loop
  isn't `recv`ing while busy in `run()`). Matches a "busy command line." Decide consciously.

## Interrupt / Ctrl-C

Ctrl-C in the REPL becomes a `Control::Pause` (or `Interrupt`) that sets the attention flag, not
a signal the REPL handles locally. Granularity is **per statement**: the seam only fires between
statements, so a single long-running builtin (`A*B`, `pause(60)`) won't yield until it returns.
Finer interruption would need poll points inside long builtins — out of scope here.

## Phasing

Each phase is independently landable and testable. Phases 1 and 2 are independent of each other
(both need Phase 0).

### Phase 0 — Engine skeleton (the refactor)
Introduce `crates/fm-cli/src/engine.rs` (or a small `fm-engine` crate): the actor thread, the
`ReplCommand`/`Event` enums, and the channels. Rewrite the **interactive REPL loop only**
(`main.rs` ~111–119) as a client: send `Eval`, await reply, print. Leave `capture.rs` and
`--list-builtins` owning a direct interpreter (they're non-interactive). No DAP, no debugging.
**DoD:** REPL behaves exactly as before; tab-completion/queries (if any) routed via `Query`;
graphics sink still installed; all existing tests green.

### Phase 1 — Re-entrant seam (`fm-interp`)
Replace `take()` with re-entrant consultation; add `suppress_breakpoints`; add pause-level depth.
Update the standalone `fm-dap` hook to the new shape. **DoD:** the existing 8 `fm-dap` tests pass;
add a test that a watch `evaluate` does **not** recurse, and (with a stub hook) that a nested run
*can* re-enter the seam.

### Phase 2 — Embedded DAP-over-TCP as an engine client (attach)
Add `--dap-port N` to `fm-cli`: a DAP socket-IO thread + the `Control`/`Event` channels +
`Arc<Mutex<breakpoints>>` + the attention flag. This is the **attach** counterpart to the launch
binary: set breakpoints from the IDE, trigger a run from the REPL, stop at the breakpoint.
**DoD:** point the existing `DapClient` harness at the TCP port; a breakpoint set by the client
fires on a run triggered via a simulated REPL `Eval`; variables/stack correct.

### Phase 3 — Nested debugger REPL + recursive breakpoints
Paused loop services the REPL channel; nested lines run in the current scope; output mirrored;
recursive breakpoints push/pop pause levels; `Continue`/`Step` act on the innermost level.
**DoD:** tests for (a) a nested `Eval` mutating the paused frame and the change being visible on
resume, (b) a nested call hitting a second breakpoint (depth 2 `Stopped`), (c) `continue` popping
levels back to the outer program.

### Phase 4 — Polish
Ctrl-C → `Pause`; stream program `Output` as DAP `output` events; engine-thread panic isolation
(reply channels return `RecvError` → clean client teardown, REPL recovers); wire
`dbstack`/`dbup`/`dbdown` to the pause-level stack; document the `attach` flow for VS Code/Zed.

## Testing strategy

- Reuse `crates/fm-dap/tests/common/DapClient`; add a variant that connects over the embedded
  `--dap-port` instead of spawning `serve` directly.
- **Two-client tests** are the novel coverage: a thread driving REPL `Eval`s and a `DapClient`
  driving control, asserting on interleavings (set BP → REPL-run → stop → nested `Eval` mutates →
  continue → REPL sees the mutation).
- Keep per-statement seam overhead honest: a micro-bench that runs a tight loop with a DAP client
  attached but no breakpoints set, asserting the attention-flag fast path doesn't regress.

## Risks & open questions

- **Re-entrancy borrow shape.** Making the seam re-entrant without `take()` is the fiddliest bit;
  the hook's mutable bookkeeping must avoid a borrow held across a nested `recv`. Prefer moving
  that state into the engine and keeping the hook's channel ends shareable.
- **Audit all direct `interp` access in `fm-cli`.** Anything interactive that currently touches
  the interpreter synchronously (notably tab-completion querying the symbol table) must become a
  `Query` round-trip. Phase 0 must enumerate these (the grep is short today).
- **Queued-REPL-during-free-run UX.** Confirm "command line is busy until the run stops" is the
  desired behavior vs. some explicit rejection.
- **Frame-scoped `evaluate`.** DAP passes a `frameId`; today evaluation is top-frame only. Honor
  non-top frames via `set_active` or defer (document the limitation).
```
