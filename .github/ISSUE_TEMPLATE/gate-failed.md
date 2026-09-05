---
name: A gate failed
about: promote.sh or claims.py reported a failure on your machine
labels: gates
---

**Command and full output:**

```
$ bash symbolon/promote.sh
```

**Environment:** OS / rustc / node / python3 versions

▲ Skips are expected: two gates need things that are not published. A **failure** is not
expected and is worth reporting even if you think it is your environment — a gate that fails
for an unclear reason is a defect in the gate.
