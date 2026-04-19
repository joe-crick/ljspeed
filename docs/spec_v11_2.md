# Specification: JS-Syntax Macro System for LJSP Targets (v11.2)

## 16.2.2 `recur` inside `try` / `catch` / `finally`

`recur(...)` is a compile-time control transfer, not an ordinary runtime call. Because JavaScript `try` / `catch` / `finally` has strict control-flow semantics, the compiler MUST define explicit rules for `recur` in these contexts.

### Conservative validity rule

`recur(...)` is **invalid** in any position whose execution would transfer control across a `finally` clause.

This means:

- `recur` is invalid inside a `finally` block
- `recur` is invalid inside a `try` block if that `try` has an attached `finally` and the `recur` would exit through that `finally`
- `recur` is invalid inside a `catch` block if that `catch` has an attached `finally` and the `recur` would exit through that `finally`

These cases MUST produce a compile-time `AnalysisError` or `LoweringError`.

### Allowed cases

`recur(...)` MAY be allowed in:

- plain tail position outside `try` / `catch` / `finally`
- tail position inside `try` when there is no `finally`
- tail position inside `catch` when there is no `finally`

provided all other `recur` validity rules are satisfied.

### Rationale

A direct `recur` lowering is effectively a loop jump such as parameter reassignment plus `continue`. Jumping across a `finally` boundary is not equivalent to ordinary loop control unless the compiler introduces additional internal control machinery to preserve JavaScript semantics exactly.

### Optional advanced implementation path

An implementation MAY support `recur` across `finally` boundaries, but only if it lowers such cases through an **internal control protocol** that preserves JavaScript `try` / `catch` / `finally` behavior exactly.

In such an implementation:

- `finally` MUST run exactly once per control transfer
- if `finally` throws, that throw MUST override the recur transfer exactly as it would in normal JavaScript control flow
- the internal recur-control representation MUST remain fully inside compiler-generated scaffolding and MUST NOT leak into emitted user-visible expression results

### Conceptual shape of advanced lowering

A compiler may lower a recur-capable function into a driver loop that evaluates the function body inside a control wrapper:

```js
while (true) {
  const outcome = (() => {
    try {
      return body();
    } finally {
      cleanup();
    }
  })();

  if (isRecur(outcome)) {
    [n, acc] = outcome.values;
    continue;
  }

  return outcome;
}
```

This example is illustrative only. Implementations MAY use a different internal representation so long as semantics are preserved.

### Examples

#### Valid

```js
function fact(n, acc) {
  try {
    if (n === 0) return acc;
    return recur(n - 1, acc * n);
  } catch (e) {
    throw e;
  }
}
```

This is valid only if there is no `finally` and the implementation otherwise supports tail-position `recur` in `try` / `catch`.

#### Invalid

```js
function fact(n, acc) {
  try {
    if (n === 0) return acc;
    return recur(n - 1, acc * n);
  } finally {
    cleanup();
  }
}
```

This is invalid under the conservative rule because the `recur` would transfer control across a `finally` clause.

#### Invalid

```js
function f(n) {
  try {
    work();
  } finally {
    return recur(n - 1);
  }
}
```

This is always invalid under the conservative rule.

---

## Summary

v11.2 adds an explicit control-flow rule for `recur` with `try` / `catch` / `finally`:

- conservative baseline: forbid `recur` across `finally`
- optional advanced support: allow it only through an internal control protocol that preserves JavaScript semantics exactly
