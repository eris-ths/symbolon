# Misses

A record of predictions we wrote down **before** measuring, and what the measurement said.

◆ **Why this file exists.** Results are cheap to publish; error rates are not. Every entry
below was written into a source file or a commit message *before the run*, so git holds the
timestamp. That is the only thing that makes a prediction a prediction.

⚠️ We are wrong often enough that this file is the honest summary of the work.

---

## The seventeen at a glance

◆ Each row links a prediction to what the measurement said, and to the type it left behind.
The types are the part that transfers; the numbers are only how we found them.
▲ Every number in this table is restated from the entry below it — the entry is the source.

| # | what we predicted | what it measured | ◆ the type it left behind |
|---|---|---|---|
| 1 | a systems language buys 50–100× | 8.6× | the cost was the *height of the tower*, not the host language |
| 2 | refcounting is what is left to remove | 1.1× | ownership buys **boundedness, not speed** |
| 3 | cons-heavy programs fall to 50–200× | 426×, higher than the integer loop | "it stopped" is not "it could not go further" |
| 4 | collapsing the remaining tower buys another 2–10× | ≈1.0 | our output has no tower left to collapse — which says nothing about the *other* axis |
| 5 | prepending to a packed string degrades to cells | silently wrong | **graceful degradation is a mechanism you build**, not a property a system has |
| 6 | the stages agree, so the compiler is right | one stage was wrong about a shape the others never disagreed on | agreement between implementations that share an assumption does not test that assumption |
| 7 | four gates on that stage is decent coverage | a program that reads a pair without ever building one produced a broken module | **derive a declaration from its use.** Written *next to* the use, the two drift |
| 8 | refusing was timid; supporting it is the real fix | the support silently truncated — the refusal it replaced had been correct | replacing a refusal removes a gate; check what the refusal was holding |
| 9 | register allocation is worth several times | 1.37× | the mechanism was read right and the **cost centre** was read wrong |
| 10 | the gates cover the claims | they covered the *numbers*; the entrance page said the wrong thing for two days | **a ratchet only holds the shape it was cut for** |
| 11 | the gates cover the repository | the commit message is outside the repository, and no gate reads it | the same shape, one level out — the boundary of a gate is not the boundary of the work |
| 12 | the closure probe covers closures | it covered closures whose captures were never shadowed | a probe covers the case it draws, not the name it carries |
| 13 | tighten the branch — the hottest thing in a loop | the loop had one branch and no comparisons at all | **a target chosen from intuition is a target chosen from someone else's program** |
| 14 | the fuzzer draws six shapes, so six shapes are covered | one of the six had never been drawn — 0 out of 200,000 | **emitting a shape is not shooting it**, and absence is unreadable |
| 15 | packing small integers is invisible to everything else | it was visible in the type system, and refused ordinary programs | **a representation is not a type** |
| 16 | floor D is a switch-dispatch VM at its practical limit | 38.7% of its dispatches were a single four-instruction sequence | one number had been standing for two quantities |
| 17 | the cell-level reclaim is covered — there is a probe, and it passes | every probe hit the same side of the branch | **an optimisation with a condition in it is not covered until both answers are drawn** |

⚠️ This table grows by one row per entry, and the fourth column is the reason to add one: a
miss that leaves no transferable type has probably not been understood yet.

---

## 1. "Rewriting the interpreter in a systems language will buy 50–100×"

**Measured: 8.6×.**

Refuted before the current work began, and kept here as the cautionary type. The cost we were
trying to remove was not in the host language. It was in the *height of the tower* — the
interpreter re-deciding, on every step, what the program already knew.

## 2. "Reference counting is what is left to remove"

**Measured: 1.1×.**

Dropping refcounts and moving to an arena bought almost nothing in time. ⇒ The claim had to be
rewritten, not softened: **ownership buys boundedness, not speed.** Its payoff shows up at a
region boundary, as memory that stops growing — measured as peak cells `1,000 → 50`, at a cost
of `+80` instructions (+0.149%).

🔴 We had written "ownership is the remaining cost" in three places. Changing one and leaving
the others is how a document rots. See the type in §5 of the chronicle.

## 3. "Programs dominated by `cons` will fall to 50–200×, because the allocation is real work"

**Measured: 426× — higher than the integer loop.**

The reason we missed: **99.2% of the allocation was the interpreter's own.** Folding the tower
removed it along with everything else. What remained (300 cells) was the list itself — the
data, which cannot be removed because it is the answer.

▲ On the way to this number the run stopped with an unimplemented opcode. We nearly reported
that as a limit of the approach. It was a hole in our own specializer. **"It stopped" is not
"it could not go further."**

## 4. "Collapsing the remaining tower will buy another 2–10×"

**Measured: ≈1.0.**

Run against a sister runtime that performs all three Futamura projections literally, our output
collapsed by essentially nothing. The finding is not a failure of that runtime — it is that
**our output has no tower left to collapse.** The distance to the floor, along the axis of
interpretation, is zero.

▲ This says nothing about the *other* axis. Reducing the number of emitted operations —
register allocation, peephole — is untouched.

## 5. "Prepending to a packed string will fall back to cons cells — correct but slow"

**Measured: silently wrong.**

A literal string is packed into a single machine word (an address and a length) and read in
place, so no cell is ever allocated for it. Writing that up, we named the one case we had not
built: putting a character *in front* of such a string needs a copy, so the fast exit would
decline it and the program would fall back to cells — slower, but right.

It did not decline. It accepted the value and stored the packed word where a cell address
belongs. Reading the second character of the result gave `0` where the reference floor gives
`115`. Nothing crashed. A longer walk did eventually trap, but only by luck — it had wandered
off following a length as though it were an address.

🔴 The prediction was not merely wrong; it was wrong in the direction that sounds safe.
"It will degrade gracefully" was a hope, and no line of code implemented it. **Graceful
degradation is a mechanism you build, not a property a system has by default.**

▲ Chasing the cause found a second door onto the same hole — storing a string into a slot the
compiler had already typed as a list — failing just as quietly. One report is one report; it
is never the whole set.

▲ The rule the compiler was following ("a string may stand in for a list") existed as a
comment above a helper that **was never called even once**. The behaviour lived somewhere
else. A comment describing a design is not the design.

The fast exit refused both, so such a program fell back to the interpreter stage and was
merely slow — the behaviour the prediction had claimed. Storing a string into a slot already
typed as a list is a type error and is still refused: `probe_strbox.json` must report `REFUSE str-as-list`,
and deleting the refusal drops the gate. Prepending was different: it is
a real operation that simply needs a copy, and entry 8 records what happened when the copy
was written.

---

## 6. "The stages agree, so the compiler is right"

**Measured: one stage was wrong about a shape the others never disagreed on.**

Nesting a `cons` in the *tail* of another `cons` — `car(cons(300, cons(2, nil)))` — gives
`300` at the interpreter, `300` at the specialising stage, `300` at the JIT, and **`2`** at
the wasm stage. Nothing crashed.

The cause is one line: the compiler kept the head of a pair in a **single scratch local shared
by every `cons` in the program**. Emitting the tail runs the inner `cons`, which overwrites it.
Nesting has been in the source language since its first day; the compiler had put a local value
in a global box.

🔴 This had shipped, and every gate was green. The gates that shoot this stage run four
programs, and not one of them nests a `cons` in a tail. **A gate covers the programs it runs,
not the language it was written for.**

▲ The line that would have caught it was already on screen. The ladder's summary compares the
interpreter, the specialiser and the JIT; the wasm stage is not in it, and since the previous
entry it says so out loud. Renaming that line added no coverage — it made the missing coverage
*nameable*. This is the entry that used the name.

The nested case is now a probe of its own, and the gate shoots it twice: once to fold it, and
once to run the wasm that came out — `node probe_run.mjs probe_nest.wasm 702`. Before the fix
that run returns `6`; after it, `702`. The same run reports a heap peak of `9 cells` for what
is a three-cell list walked three times: the compiler does not share subexpressions, and now
that is written down instead of assumed.

▲ Removing the dead helper named in entry 5 left a second one behind — a wrapper the compiler
never called. `rustc` had been printing `dead_code` for it the whole time, under a build whose
output we only ever checked for the word `error`.

---

## 7. "Four gates on that stage is decent coverage"

**Measured: a program that reads a pair without ever building one produced a module that
would not load at all — and the ladder called it folded.**

Entry 6 ended on a type: a gate covers the programs it runs, not the language it was written
for. Adding a probe by hand only covers the shape you thought of, so the next step was to stop
thinking of shapes. `fuzz.py` builds small programs from a fixed seed, takes the expected value
from the Python floor, folds each one, and **runs the wasm that comes out** — in all three
reclaim modes.

The first widened run found this: a box holding `nil`, walked by a loop that never iterates.
The loop body reads `car`, so the compiler emits a memory load; nothing ever calls `cons`, so
the bump heap is never claimed; and the module declared its memory only when the heap had been
claimed. The output was a `.wasm` file the ladder announced as folded and no runtime will
instantiate — *"memory index 0 exceeds number of declared memories (0)"*.

🔴 The declaration of a resource was written **next to** its use rather than **derived from**
it. Two places had to agree, and one of them was updated the day memory was introduced and
never again. Memory use now goes through a single call that sets the flag the declaration
reads, so the two cannot drift.

The gate is the fuzzer itself, pinned to a seed so it is reproducible.
Shooting `--n 120 --seed 20260906 --depth 4` agrees on 354 runs, declining 6 of them.
(Those counts move whenever the generator changes, because the shapes drawn change with it.
Entry 14 is why they are not comparable across such a change.) The generator now draws eight
shapes, and a run **fails** if any of the eight came out zero times — the roll call is the gate
on its own coverage. Four seeds at this size agree on 1,440 runs with no disagreement.
The declines are the prepend case from entry 5, still refused rather than compiled; the count
is gated too, so a silent widening of what the compiler declines shows up as a number.

▲ Shooting the fuzzer at the *previous* build was the check that the fuzzer works at all: it
finds the nesting bug of entry 6 in 40 programs. A test suite that has never failed is a test
suite of unknown strength.

▲ Two smaller things, both self-inflicted and both caught by running rather than reading.
Two fuzz runs started at once fought over the same scratch filenames and one deleted the
other's compiler; scratch paths now carry the process id. And the two ledger numbers above
were first written from a guess and corrected to the measurement before they shipped —
the shape of miss this file exists to record, caught one step earlier than usual.

---

## 8. "Refusing was the timid answer; supporting it is the real fix"

**Measured: the support silently truncated. The refusal it replaced had been correct.**

Entry 5 ends with the fast exit declining to put a character in front of a packed string,
because that needs a copy. Declining is honest but costly: one prepend anywhere in a program
and the whole program falls back to the interpreter. So the copy was written — allocate
`len+1` bytes, store the new head, copy the rest, hand back the packed word. The prepend
probe folded and ran and gave the right answer, and every existing gate stayed green.

The fuzzer from entry 7 disagreed within 300 programs. A packed string holds **one byte per
element**; the head being prepended is an arbitrary machine integer. `cons(add(394, 305), s)`
stored 699 through an `i64.store8` and got 187 back. The sum came out 256 short. Nothing
crashed.

🔴 The day before, this file recorded that a hoped-for graceful degradation had never been
implemented. This is the same lesson from the other side: **a correct refusal was replaced by
an implementation that was silently wrong.** "We support it now" is not the same claim as "we
support it correctly", and the second one is the only one worth making.

Prepending is now folded **only when the head is a literal in 0..255** — which is the case
strings are actually built from — and refused otherwise, so the refusal was narrowed rather
than removed. Both halves are gated. Shooting `node probe_run.mjs probe_prepend.wasm 88020`
must agree; and `probe_prependw.json`, whose head is 1000, must report `REFUSE str-prepend-nonbyte`.

The measured result is better than the prediction in entry 5 had hoped for. That prediction
said the program would fall back to cons cells and be *correct but slow*; instead it stays
packed, at one byte per character where a cell costs sixteen. The prepend probe reports
a heap peak of `63` sixteen-byte units where cells would have needed `960`.

▲ The remaining road, unbuilt and written down as unbuilt: expanding a packed string back
into cells would support an arbitrary integer head, at one cell per character. It will be
written when something needs it, not before.

---

## 9. "Register allocation is worth several times"

**Measured: 1.37×.**

The ladder's own summary had been saying, since the JIT was written, that stage F was a naive
stack-machine translation with `push`/`pop` left in, and that *register allocation would still
be worth several times*. That sentence printed on every run and had never been shot.

Keeping only the top of the operand stack in a register — one slot, spilled at every label so
the two sides of a branch agree — removes almost all of the `push`/`pop` traffic. The integer
loop went from `187 bytes` of machine code to `166 bytes`, and from 0.0055 ms to 0.0040 ms.
That is a real win and it is not several times.

🔴 The claim was not wrong about the mechanism, it was wrong about the **cost centre**. The
stack traffic was never the dominant term; local variables still make a round trip to memory
through `rdi` on every read and write. "Several times" describes real register allocation,
which pins locals to registers — the part not built. The summary now says which part is done
and which is not.

▲ The number `187` appeared in six places. Three were the current byte count (a table, a
sentence, and an ASCII diagram); three were history or an unrelated value. This is entry 1's
type, and the only reason it did not repeat is that all six were read before any were changed.

🔴 **Follow-up, same day: the original claim was right and this entry was wrong to close.**
Having named the cost centre — locals making a round trip through the argument pointer on every
read — the obvious thing was to fix it. This function calls nothing, so every caller-saved
register is free; the six busiest slots now live in registers, zeroed on entry because the
caller only clears the memory array. That is the *actual* register allocation the summary line
had meant, and pinning them brings it to `122 bytes` from the 166 above, and 0.0040 ms to
0.0020 ms. Measured end to end against where this entry started: 2.75×.

◆ "Several times" was a correct prediction that two separate measurements disagreed with,
because both measured something narrower than the claim. **A prediction is not refuted by a
measurement of a different thing** — and the way to find that out is to build the thing the
prediction actually described, not to close the entry at the first number.

---

## 10. "The gates cover the claims"

**Measured: they covered the numbers. The entrance page said the wrong thing for two days.**

The README carries a section called *What this does not do*, and its first line read: "No
strings. Integers and lists of integers only. Adding strings is the next rung." Strings had
landed two days earlier, with probes, refusals and gated measurements. Nothing rang.

🔴 The reason is structural, not careless. The claims gate re-derives every **number** in the
documents from the tool that produced it, and refuses to accept an unexplained one. A sentence
about a *capability* carries no number, so it sat outside the ratchet entirely — in the one
section whose whole value is being trustworthy about limits.

▲ It was reaching readers. Asked to summarise the live page, a reader came back with "it
handles only integers and integer lists". The stale line was not merely present; it was working.

Two changes, and the second is the one that matters.

- The section was rewritten so each limit names something a gate shoots: the string limit now
  cites the refusal marker `REFUSE str-prepend-nonbyte`, and the self-sufficiency limit cites
  `NOIMPORT ok`, which the emitter prints only after scanning its own output for an import
  section — an absence turned into a thing that must be present.
- A fourth check was added: in a section declared as needing anchors, every bullet must contain
  a sentence the ledger names, or be listed as unanchored with a reason. Restoring the old "No
  strings" line makes the run fail; the current text passes.

◆ **A ratchet only holds the shape it was cut for.** This one was cut for numbers, so prose
walked straight past it for as long as the prose was wrong.

---

## 11. "The gates cover the repository"

**Measured: the commit message is outside the repository, and no gate reads it.**

The previous projection's message ended with a line saying the ledger had gone from one count
to another. The second number was right. The first was written from memory and was wrong — it
was the starting point of an earlier step, two rungs back. It is now in the public history,
where it cannot be corrected in place, because the shadow's history is appended one projection
at a time and never rewritten.

🔴 The irony is exact: that message's own body describes a ratchet that only held the shape it
was cut for, and the message itself sat outside a different one. Every number *inside* the
tree is re-derived by a gate. A number in the text *about* the tree had nothing watching it.

The projection tool now reads counts out of **both trees** — the one currently published and
the one about to be — and checks any "A → B" claim against them.

▲ The naive version of this check would not have caught it. Verifying only the new value passes
a message whose *old* value is wrong, which is exactly the shape of this miss. The check was
written against the actual failing text first, and only then generalised.

▲ Two things went wrong while building it, both caught by running rather than reading. Pointing
the "before" side at the published clone gave the wrong baseline, because the projection
overwrites that clone before the message is written; the baseline is now read from git by ref.
And the first version printed a pass when it had measured nothing — the one shape this
repository refuses everywhere else, reproduced inside the tool built to prevent it.

---

## 12. "The closure probe covers closures"

**Measured: it covered closures that never had their captures shadowed, which is the only case
that works.**

A closure in this language captures the environment where it is *defined*. The specialiser
inlines the body at the call site, and it was resolving the body's names against the
environment *there*. The two agree until a name the body captured is bound again in between:

    let a = 5 ; let f = ofn(q, q + a) ; let a = 90 ; f(1)

The reference floor gives `6`. The specialiser, the JIT and the wasm stage all gave `91`.
Nothing crashed. The hand-written closure probe exercised closures on every run and never
rebound anything, so the shape simply never occurred.

🔴 Chasing it turned up a second, unrelated hole in the same region. The ownership scan that
decides whether a list can be reclaimed destructures a closure node one level too deep — it had
been written against the shape of a `let`, which nests one deeper than an `ofn`. It reaches that
code only when a program holds **both** a list in a box and a closure, and no probe held both.
Under `--own` and `--own2` such a program made the compiler panic outright.

▲ And the fix for the first hole introduced a third. Restoring the caller's environment after
inlining left the original single `pop` in place, so the call ate one of the caller's bindings;
`let a=57; let f=…; let a=331; if(f(a), a, …)` returned the old `a` from the branch. The fuzzer
found it within 60 programs, about ten minutes after the fix was written. **Add cleanup, remove
the cleanup it replaces** — tidying twice damages the neighbour.

All three are now one probe, shot in three reclaim modes, because the differential fuzzer that
found them cannot run where this repository is published. Shooting
`node probe_run.mjs probe_capture.wasm 414` must agree; under `--own2` it must fold and agree on 414. Against the previous
build that probe reports 499 with no reclaim, and a panic with it.

▲ While writing this entry a ledger number was again taken from the wrong run — a count from
`--n 150` written next to a gate that shoots `--n 120`. The gate rejected it. That is the third
time today for the same class of slip, and the only reason none of them shipped.

---

## 13. "Tighten the branch — it is the hottest thing in a loop"

**Measured: the loop had one branch and no comparisons at all.**

The plan for the next tuning pass was to fuse the comparison and the conditional jump, which is
the textbook peephole. Before writing it, the emitted op stream was counted. In the integer
benchmark there is exactly **one** `Jz` and **zero** `Eq`. What there is, two or three times per
program, is a binary operation whose two operands are each a plain load or a literal — and
between them a `push` and a `pop`, because the stack machine says so and nothing had looked.

Folding those pairs took the integer loop from 0.0020 ms to 0.0009 ms;
folding those operands brings it to `102 bytes`. The closure probe went to 0.0004 ms and 122
bytes. Measured against where this thread started: 6.1×, and 187 bytes down to 102.

◆ **A target chosen from intuition is a target chosen from someone else's program.** The
textbook peephole is right for code that compares; this code mostly moves.

🔴 The first version of the fold segfaulted, then hung. `add rax, imm32` is six bytes and the
length table said five, so every jump after the first fold pointed one byte short. The bug was
one constant. The fix was not.

A separate table of instruction lengths has to agree with the emitter that writes those
instructions, and the two live in different functions. That is the same shape as the memory
section declared beside its use earlier the same day, and it fails the same way: silently, until
something jumps. Both passes now go through **one** emitter — the first pass emits with
placeholder jump targets purely to learn the offsets, and the second emits again with real ones.
Relative jumps are a fixed width, so the lengths are identical by construction. There is no
table left to disagree with.

▲ A JIT bug leaves running processes behind. The hung build spun a core for six minutes while
the next measurement was being taken, which is a good way to record a wrong number about
something unrelated. Shoot a suspect binary under a timeout.

## 14. "The fuzzer draws six shapes, so six shapes are covered"

**Measured: one of the six had never been drawn — 0 out of 200,000.**

The generator picked a shape by walking a chain of cumulative thresholds:

```python
if   c < 0.35: ...      # integer binding
elif c < 0.65: ...      # list binding
elif c < 0.62: ...      # counting while   <- ends below the band before it
```

The third band ends below the second, so it is dead. Behind it is the counting `while`: two
boxes, an integer accumulator, no heap — and it is the only loop the x86-64 stage will accept.
Every run printed how many programs that stage had taken, and the number was never zero, so
nothing looked wrong. Those were straight-line integer programs. The loop was not among them.

The typo is not the miss. The miss is that a generator can carry a dead arm and still print a
number that reads like coverage: *"354 agree"* is agreement **among the shapes that were
drawn**, and a shape that is never drawn does not appear in that number at all. It is absent,
and absence is the one thing this repository keeps rediscovering it cannot read.

The chain is now a weight table, where a band cannot be written unreachable. And every run ends
with a roll call of what it actually produced, which **fails** if any declared shape came out
zero. Same move as the module that scans its own output for imports: turn a claim founded on
absence into one founded on presence, so that breaking it is loud rather than quiet.

▲ The fix changed the random stream, so the ledger row moved with it. A number measured through
a net with a hole in it does not carry across the repair.

## 15. "Packing a list of small integers is an optimization — invisible to everything else"

**Measured: it was visible in the type system, and it refused ordinary programs.**

Stage G‴ packs a statically known chain of byte-sized literals into the data section: one byte
per element instead of a sixteen-byte cell. The value is the same list; only the placement
differs. That is what "optimization" was supposed to mean.

The wasm stage infers a `Kind` for every expression, and a packed string is `Str` while a chain
of cells is `List`. At an `if`, the two branches must agree. So:

```
if c then cons(3, cons(4, nil)) else cons(300, nil)
```

was **declined** — not because the branches return different things, they are both lists, but
because one branch's elements happened to fit in a byte and the other's did not. The predicate
that decided the program's fate was the *magnitude of its constants*.

The differential fuzzer found it within a day of being taught to put lists under an `if` — a
shape it had never drawn before, because `if` had only ever been generated around integers.

The join now demotes: when the two branches differ only as `Str` against `List`, the emitted
bytes are rewound and both branches are emitted again with packing switched off, so they agree
as cells. Three probes that were declined now compile and run to the right answers — the gate shoots one
of them and requires `303`; a branch
whose halves genuinely differ — an integer against a list — is still declined. The refusal was
narrowed, not removed.

◆ **A representation is not a type.** The moment an optimization can be observed by a
correctness check, it is no longer an optimization.

⚠️ The rewind re-emits, rather than predicting the branch kinds with a separate function. A
predictor would be a second mouth describing what the emitter does, and this repository has
already paid for one of those: the table of instruction lengths in entry 13, which disagreed
with the emitter by one byte and cost a segfault. The same emitter runs both times.

## 16. "Floor D is a switch-dispatch VM at its practical limit"

**Measured: 38.7% of its dispatches were a single four-instruction sequence — and that sequence
was the machine's own opcode dispatch, not the floor's.**

Floor D ran the integer benchmark at 4.20 ns per instruction, roughly twelve cycles, and the
tool itself printed that switch-dispatch VMs bottom out around 1–3 ns. That read as *there is a
little left and it is not interesting.*

Counting the 6,103,390 instructions it executed says otherwise. Four opcodes cover **79.5%** of
the stream — `Load` 25.5%, `Lit` 20.3%, `Eq` 19.8%, `JmpF` 13.9% — and they are not scattered.
The dynamic four-gram `Load; Lit; Eq; JmpF` runs **787,565 times: 38.7% of every dispatch, in
one shape.**

That shape is the interpreter comparing an opcode tag against each of fourteen constants. It is
not the floor being coarse; it is the price of the machine being a machine, standing on the
floor and being counted as floor. Folding it into one instruction: **25.6 ms → 17.9 ms (1.43×)**,
with 51.6% of dispatches gone.

⚠️ The fold does not shorten the instruction vector. Jump targets are indices into it, so
shrinking it would create two things that have to agree — the same shape as the instruction
length table in entry 13, which cost a segfault. The folded instruction steps `pc` past its own
remains instead, the set of jump targets is derived from the vector rather than kept beside it,
and a position that can be jumped into is never folded.

🔴 One number had been standing for two quantities. *"Instructions floor D actually ran:
6,103,390"* was both the **thickness of the tower** — the work the machine's meaning demands —
and the **floor's dispatch count**. Folding splits them: the tower is unchanged at 6,103,390,
the dispatches fall to 2,956,119. Counting now runs on the unfolded stream and execution on the
folded one, printed under different names. A gate was anchored on the old sentence and would
have gone quiet the moment the sentence was corrected, so it now anchors on `[D-tower]`.

🔴 A second copy of the same shape was found while writing this up, and it was ours. The
counting run and the executing run were **two loops carrying the same fifteen arms**, differing
only in a counter and a return value — and the fold had just added four more arms to each. Rust's
exhaustiveness check stops an arm from being *missing*; it does not stop two arms from *saying
different things*. That divergence would be silent, and the count it corrupts is a number the
ledger gates. The two are now one function specialised on a constant. ▲ It costs about 6% of
floor D's time, measured, and forcing full specialisation did not give it back. That price is
paid deliberately.

◆ **The ratios in this document that are stated in time shrank. The ratios stated in
instructions did not move.** Stage E is 339–632× faster than floor D where it was 646–740× —
and that spread is two samples of four programs, not a stable figure. Nothing got slower; the
denominator changed. The 407× on the front page is an instruction ratio and has not moved,
because it was never measuring the same thing. ⚠️ A slower denominator flatters the tower, so
these are quoted from the build that includes the 6%, not the faster one.

▲ Two things counted and not taken: inlining the comparison bought nothing outside the noise,
and the next sequence, `Add; Add`, is 11.6% of what remains, which at roughly six nanoseconds
per folded dispatch is a few percent of wall clock.

## 17. "The cell-level reclaim is covered — there is a probe for it, and it passes"

**Measured: every probe hit the side of the branch where it fires. The other side had never
been run.**

Stage G″'s `--own` returns a cell to the bump heap when a list box is consumed by
`x := cdr(x)` — but only if the cell that fell off is **on top of the heap**. It is a bump
allocator; nothing else can be returned. So the reclaim has two outcomes, and which one happens
depends on what was allocated after the list.

Every probe built a list and walked it immediately. In that shape the head is always the most
recent allocation, so the guard always passes. The failing side — the one that decides whether
a program leaks — was reachable in principle and had been run zero times.

Adding the twin: same program, with one more cell allocated after the list is built and read
after the walk, so the head is no longer on top.

| | default | `--own` |
|---|---|---|
| walk a list built immediately before, so the guard passes | 1,000 cells | **50 cells** |
| one more cell allocated after it, so the guard rejects | 1,020 cells | **1,020 cells** |

Both are now gated, and the second gate requires the peak **not** to fall. A gate that only
demands the peak drop cannot tell a working guard from an absent one — both look like a pass on
the shape that was shot.

◆ **An optimisation with a condition in it is not covered until both answers to that condition
have been run.** Coverage of the code is not coverage of the decision.

---

## Misses of a different kind

The seventeen above are predictions about the system. These are about us, and they recur:

| what happened | the type |
|---|---|
| a rung fixed a number in a table and left it stale in the prose and a diagram — twice, unnoticed for five rungs | **A rung touches the whole document. If a person checks that, it gets missed.** |
| a commit message claimed a document had been updated when the edit had failed | **"I wrote it" is not "it is written." Read the result back before claiming it.** |
| a rule was quoted from memory although the current version was on screen | **Quote the rule by shooting it, not by remembering it.** |
| a capability was asserted because the tool appeared in a list; it returned 403 | **A tool existing and a tool working are different questions.** |
| a gate's own patterns were written without ever running that gate; three of them were wrong | **The contents of a gate you have never fired are unverified.** |
| a summary line read "all stages agree" while one of the stages had never been run | **A check must not claim more ground in its name than it covers in its body.** |
| a compiler warning had been printing on every build for as long as anyone could remember | **A warning nobody reads is not a warning. Build clean, or it is decoration.** |
| a resource was declared in one place and used in another, and the two drifted | **Derive the declaration from the use. Two places that must agree will not.** |
| a lesson written down one day was walked into the next, by the person who wrote it | **Writing the rule down is not obeying it. Only a gate obeys.** |
| a generator carried a branch that could never be reached, and the run still printed a coverage-shaped number | **A count of agreements is a count among the shapes that were drawn. Shapes never drawn are absent, and absence is unreadable.** |
| a new shape was added to the generator; the compiler declined it, then the reference floor declined it, so it reached the code under test zero times | **Emitting a shape is not shooting it. A refusal is not coverage.** |
| a roll call was added to prove every shape was drawn; it counted shapes drawn, and a shape can be drawn into a program that is then declined | **Every count answers one question. Check which one, and whether it is the question you needed answered.** |

◆ All twelve are the same shape: **existing and working are different.** The gates in this
repository exist because of them.
