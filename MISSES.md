# Misses

A record of predictions we wrote down **before** measuring, and what the measurement said.

◆ **Why this file exists.** Results are cheap to publish; error rates are not. Every entry
below was written into a source file or a commit message *before the run*, so git holds the
timestamp. That is the only thing that makes a prediction a prediction.

⚠️ We are wrong often enough that this file is the honest summary of the work.

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

The fast exit now refuses both, so such a program falls back to the interpreter stage and is
merely slow — the behaviour the prediction had claimed. Both refusals are gated by positive
evidence: shooting `probe_prepend.json` must report `REFUSE str-prepend`, and
`probe_strbox.json` must report `REFUSE str-as-list`. Delete the refusal and the gate falls.

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
Shooting `--n 120 --seed 20260906 --depth 4` agrees on 351 runs, declining 9 of them.
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

## Misses of a different kind

The seven above are predictions about the system. These are about us, and they recur:

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

◆ All eight are the same shape: **existing and working are different.** The gates in this
repository exist because of them.
