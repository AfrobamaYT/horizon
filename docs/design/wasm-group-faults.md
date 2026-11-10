# What happens to a wasm dependency group after a trap or an interruption

Design note for #2766 (interruption counted by work, not by wall clock) and #2761 (one failing mod shouldn't
disable its whole group). Both come down to one question: when a call into a group fails, what is still safe to run?
This note answers that question. It changes no code. The facts below are from `crates/horizon-mod-wasm` at main
fbf4f06490.

## Decisions

1. **Classify each failure by whether guest code was cut off midway.** A *refusal* happens when guest code returned
   normally or never ran: the guest's error reply, a reply the host can't decode, the depth limit, a busy group.
   A *fault* happens when guest code stopped midway: a trap (including a guest panic, which is a trap under
   `panic_abort`), an import error that traps the guest, or an interruption.
2. **A refusal no longer stops the group.** The call answers as unavailable, the same as today, and logs once per
   listener with the reason. The group keeps running. Today every refusal except a busy group stops the group.
3. **A fault stops the whole group, and the group is never resumed.** Group memory, the allocator in the std member,
   library statics and the guest's per-depth call stacks may be half-updated. A per-member stop flag over that memory
   would not make continuing safe, so none is added.
4. **Recovery means rebuilding the group, not resuming it.** At a safe point a new group is instantiated through the
   existing load path, with the faulting member treated as refused at load. Its dependents go with it, by the rule
   load already applies to a required dependency that is missing. Each library is still one copy in one memory per
   group, and no second mechanism is added. Until a rebuild exists, the stop lasts until the next start, as today.
5. **The interruption budget counts guest time, not wall time.** Epoch interruption stays (it is cheap). A deadline
   callback grants back the epochs spent inside host imports, and checks the thread's CPU time before it interrupts,
   so a host import or a descheduled thread can't use up a guest's budget. Fuel is the alternative to measure, not
   the default.
6. **A fault names three things:** the member whose hook was entered, the member whose code was running at the trap
   (the innermost wasm frame), and the call path through the host. It goes to the log, to the Mods page and to
   operators, so it is never silent.
7. **Stopped members are inert and say so.** Their listeners, Parts and providers are skipped, as `is_stopped`
   already does. A block entity whose Part belongs to a stopped group keeps its saved data unchanged, so it resumes
   when the group runs again. The Mods page shows the group as stopped with its fault.

## Where things stand

- **One store per group.** `WasmMod::instantiate` (`runtime.rs:439-531`) builds one `Store`, links every member into
  one memory through `Group::link_with`, and gives each member's `WasmMod` the same `Arc<Running>`. `Running` holds
  the store and a single `poisoned` flag. A group is created once, at startup (`entry.rs:98` → `load_installed`); a
  world reload does not rebuild it.
- **Every failure stops the group.** `call` returns `None` on an already-stopped group (`:551`). It stops the group
  when the depth passes 64 (`:605`) and on any `Err` from the call body (`:664`). That `Err` covers traps,
  interruptions, import errors, the guest's `u32::MAX` error reply (`invoke`), and replies the host can't decode or
  that have trailing bytes. Only a busy group (`Refusal::Busy`) returns `None` without stopping it. Nothing clears
  the flag.
- **The deadline counts wall clock.** `engine()` (`:74-90`) runs a thread that adds one epoch every millisecond. An
  outermost call sets a deadline of 100 epochs, or 30,000 for `Load` (`:587-593`). Nested calls keep the outer
  deadline. Epochs keep advancing while native code runs inside an import and while the guest thread is
  descheduled. The test `nested_load_calls_do_not_extend_the_outer_deadline` advances 1,000 epochs inside an import
  and shows that a finite nested call afterwards stops the group.
- **A stopped Part goes quiet.** The generated `WasmHandler` falls back to `NoAnswer` (`wire/host.rs:244`). The
  block-entity tick (`block_entity_ticks.rs:19-94` → `tick_mod_block_entity`, `native_hooks/shared.rs:3371-3420`)
  keeps iterating and saving the entity, so its mod logic silently never runs again. That silence is #2766's bug.
- **What a trap skips in the guest.** `wire_guest::run` pushes a `Keep` and takes a per-depth reply buffer, then
  pops and returns them when the call ends. A trap skips the pop. It also skips every `Drop` and every `RefCell`
  borrow release in the frames it unwinds, and an allocation that was mid-update in the std member's allocator
  stays mid-update. The group state (`horizon_group_state`: the handler table, the cache budget, the eviction
  callbacks) belongs to the std member, so every member shares it.
- **What a fault can't leave half-done on the host.** Under the #1909 boundary, the host decodes a reply completely
  before writing any field back, and point records do the same. So a call that faults applies none of its reply.
  What it already did through imports before the fault stays done. The native resource scope and `FrameScope`
  unwind correctly, and the group is marked stopped before held native objects drop (`PoisonOnPanic`).
- **Native differs.** A native listener's panic is fatal to the game, and the crash report names the mod through
  `content/crash.rs`. Wasm stopping only the faulting group is already more lenient. This note keeps that difference,
  because it comes from the sandbox, not from the API.

## 1. Interruption by guest work (#2766)

The runaway check should fire when the guest itself runs too long. Waiting on the host or on the OS scheduler should
not count.

**Mechanism.** Wasmtime 50 has `Store::epoch_deadline_callback`, which runs only when the deadline is reached and
returns `UpdateDeadline::Continue(n)` or `Interrupt`. This design uses it in place of the bare trap:

- **Count host time in epochs.** The ticker thread also bumps an `AtomicU64` epoch count that the runtime owns
  (`Engine` doesn't expose its counter). `host()` reads that count on entry and on exit, and adds the difference to
  the store's `host_epochs`. That is two relaxed atomic loads per import, with no clock call. #1780 counts about
  9,000 imports per 1,000-machine tick, so this adds roughly 18,000 relaxed loads per tick. That cost is to be
  measured, not assumed to be negligible.
- **Grant those epochs back at the deadline.** The callback runs at the deadline. It computes `guest = elapsed -
  host_epochs` since the outermost call began. If `guest` is under the call's budget, it continues by
  `budget - guest`. Nested calls keep sharing the outer budget, so recursion through the host still counts.
- **Confirm by CPU time before interrupting.** When `guest` reaches the budget, the callback reads the thread's CPU
  time once (`CLOCK_THREAD_CPUTIME_ID` on Unix, `GetThreadTimes` on Windows) and continues for one more budget.
  At the next deadline it reads the CPU time again. If the thread used at least half of the wall time between the
  two reads, the guest is spinning, and the call is interrupted. If it used less, the thread was starved, and the
  deadline is extended. Either way the clock is read only at a deadline, never on an ordinary call.
- **Keep a hard ceiling against true hangs.** A ceiling of 100× the budget (10 s for an ordinary call) interrupts
  whatever the CPU reading says. A server starved for that long is hung anyway, and the diagnostic says
  "starved", not "runaway".

**Why not fuel by default?** Fuel is deterministic: the same guest work uses the same fuel on any machine, which is
exactly what #2766 asks for. But it instruments every basic block. #1780's profile puts guest code, meaning dispatch
glue, codecs and allocation, at a large share of the remaining wasm tick. Fuel is therefore a measured candidate. If
its overhead on the 1,000-machine scene stays within the noise of #1780's runs, choose fuel instead and drop the CPU
confirmation. The note leaves that choice to the measurement.

**Budgets stay as they are:** 100 for ordinary calls and 30,000 for `Load`, now counted in guest epochs. A
100 ms guest hook already exceeds two ticks, so a lower budget would only add false positives.

## 2. After an interruption or a trap (#2766, #2761)

| Failure | Guest code | Group | Call answers | Diagnostic |
|---|---|---|---|---|
| Guest error reply (`u32::MAX`) | returned normally | keeps running | unavailable | once per listener: the mod, hook and message |
| Host can't decode the reply, or it has trailing bytes | returned normally | keeps running | unavailable | once per listener: mod, hook, decode error |
| Depth over 64 | not entered at that depth | keeps running | unavailable | once per call path: the path, the limit |
| Busy | not entered | keeps running | unavailable | as today (`warn`) |
| Trap, guest panic, import error | cut off | **stops** | unavailable | fault report (decision 6) |
| Interruption (after §1) | cut off | **stops** | unavailable | fault report, marked runaway or starved |

**Why a refusal is safe:** for every refusal, guest code either never ran at that depth or returned through
`wire_guest::run`, which pops its own state. The host decoded nothing, so it wrote nothing back. Rate-limiting the
log keeps a listener that refuses every tick from flooding it. A listener that refuses N times in a row could be
stopped by policy, but that is a choice to make, not a requirement for safety (open question 3).

**Why a fault stops the group:** see §4. A stopped group is skipped before any work, which costs one relaxed load,
as today.

## 3. Attributing a fault to a member (#2761)

Today the log names only `self.id`, the member whose hook was entered. That is wrong whenever the fault happens in a
library the addon called, or in a nested callback into another member.

- **The faulting member.** Read the innermost frame of the trap's `WasmBacktrace` (`FrameInfo::module`) and map its
  module back to its member with `Module::same` against the modules the linker compiled (`link.rs:509`). Members are
  linked with `--strip-all`, so function names are missing, but which module a frame belongs to is not. For an
  interruption, this is the member that was running when the deadline fired. Stated honestly: it is not
  necessarily the member that used the budget.
- **The entered member and the call path.** The host already pushes a frame for each nested call (`cx.frames`,
  `cx.module`). The report lists every member on that stack, outermost first, with the hook or point at each level.
- **What stays unattributable.** Direct calls from one member into a library don't cross the host, so the time
  inside a library function can't be charged to its caller without fuel or a profiler. The report names members at
  the granularity of host crossings plus the innermost frame, and says that this is the limit.
- **A guest panic's message.** The panic hook already sends the text through `HOST_LOG` before the trap. The fault
  report attaches that message to the innermost member, not to `cx.module`.

## 4. What is safe to keep running (#2761)

**The constraint:** all members of a group share one linear memory, one std member (with its allocator and handler
table) and one copy of each library. A trap can stop:

- inside the allocator, leaving free lists inconsistent for every member;
- with a `RefCell` borrowed, so a later borrow of it panics in another member;
- partway through a library's static update, which every dependent then reads;
- with `wire_guest`'s per-depth stacks not popped, so the next call at that depth starts out of step.

The trap gives no proof that it avoided all of these. So, after a fault:

- **Resuming any member of the faulted instance is unsafe.** That includes members that "had nothing to do with
  it": the allocator and the handler table are theirs too.
- **Splitting the group to contain faults is rejected.** Giving each addon its own copy of the library would break
  the one-copy model (#1909) and make wasm's dependency model differ from native's.
- **Rolling back to a snapshot before each call is rejected for hot paths.** It means copying or tracking writes to
  the group's memory on every outermost call. Wasmtime offers no copy-on-write checkpoint of a live memory, so the
  cost would be per call. It stays an open question for cold calls (`Load`).
- **Rebuilding is safe.** A new store, a new memory and a fresh `Load` for every member except the faulting one and
  its dependents (decision 4). The rebuild runs between ticks, never inside a call. It needs the following, before
  it can be built:
  - **Content registered at load must replay identically.** Registries are frozen once the world loads. The rebuilt
    members' `Load` must make the same registrations, which the host checks. On any difference the group stays
    stopped, with that reason.
  - **Host-held guest handler ids become stale.** Every Part, provider and handler id the host holds for that group
    is released and resolved again from the new instance, by the same ordering of registrations.
  - **State held only in guest memory is lost.** It is lost the same way as on a restart. Data kept in block entity
    NBT or attachments survives.
  - **One rebuild per fault, not a loop.** A member that faults again after its group is rebuilt stays refused until
    the next start.

## 5. Constraints kept

- **One copy of each library per group, in one memory.** A rebuild replaces the instance; it never duplicates a
  library.
- **The same dependency model for native and wasm.** Dropping a faulting member's dependents follows the load-time
  rule for a missing required dependency. No wasm-only packaging unit or grouping is added.
- **The #1909 boundary is unchanged.** Wire bytes in, a reply decoded whole before write-back, and refusals at load
  for what can't cross.
- **No silent skip.** Every stop, refusal and rebuild is reported, and the Mods page shows the group's state.

## What to measure

All measurements run on the same frozen main, with the same scene as #1780's runs: `tools/shm-scene`, 1,000
`machine_sample:machine`, a flat dedicated world, 15 s warmup and 20 s report, three repetitions, with the CPU named.

| Measurement | Compared | Acceptance |
|---|---|---|
| Host-epoch accounting (two atomic loads per import) | before and after, wasm mspt mean and p99, plus `block_entity_ticks` bench | within the run-to-run spread |
| Deadline callback | a hook that ends just under the budget, against today | no change while the guest is under budget |
| Fuel instead of epochs | same scene and bench, plus a 1M-operation guest loop | adopt fuel only if its cost is within the spread |
| False interruptions | finite work with 1,000 epochs inside an import; a nested call after a long import; a thread starved by `SCHED_IDLE` or a pinned busy sibling | no interruption |
| True runaway | an infinite loop, an infinite loop inside a nested call, a loop inside a library | interrupted within budget × 2, report names the innermost member |
| Refusal survival | guest error reply, bad reply, depth 65 | group keeps running, later calls answer |
| Fault isolation (after the rebuild lands) | addon trap in a group with a library and a second addon | the second addon and the library answer after the rebuild; the faulting addon stays refused |
| Rebuild cost | time to rebuild a group of 3 and of 10 members between ticks | reported, not gated |

## Open questions

1. **Fuel or epochs plus a CPU check?** Decided by the overhead measurement above. If fuel wins, §1's CPU
   confirmation and ceiling go away.
2. **When is the rebuild allowed?** Between any two ticks, or only at a save point, or on an operator command? A
   rebuild mid-session drops guest-memory state, which some mods may not expect.
3. **Should repeated refusals stop a listener?** For example, after 100 consecutive refusals of the same listener,
   stop that listener only (never the group). It is safe either way; the question is noise against hiding a broken
   mod.
4. **Snapshots for `Load`.** A checkpoint before each `Load` call could let a fault there drop only the faulting
   member without a full rebuild. Is that worth the extra path, or is the rebuild enough?
5. **Writes made through imports before a fault.** Hooks with a transaction (decide, fire, apply) can abort it, but
   imports outside a transaction stay applied. Should a fault always abort the current transaction, as a native
   panic unwinding through it would? That depends on #796 and #781's transaction contract.
6. **Who sees the report in multiplayer?** Server log and operators, certainly. Should players get a chat line when
   a group whose content they're interacting with stops?
