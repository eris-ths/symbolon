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

## Misses of a different kind

The five above are predictions about the system. These are about us, and they recur:

| what happened | the type |
|---|---|
| a rung fixed a number in a table and left it stale in the prose and a diagram — twice, unnoticed for five rungs | **A rung touches the whole document. If a person checks that, it gets missed.** |
| a commit message claimed a document had been updated when the edit had failed | **"I wrote it" is not "it is written." Read the result back before claiming it.** |
| a rule was quoted from memory although the current version was on screen | **Quote the rule by shooting it, not by remembering it.** |
| a capability was asserted because the tool appeared in a list; it returned 403 | **A tool existing and a tool working are different questions.** |
| a gate's own patterns were written without ever running that gate; three of them were wrong | **The contents of a gate you have never fired are unverified.** |
| a summary line read "all stages agree" while one of the stages had never been run | **A check must not claim more ground in its name than it covers in its body.** |

◆ All six are the same shape: **existing and working are different.** The gates in this
repository exist because of them.
