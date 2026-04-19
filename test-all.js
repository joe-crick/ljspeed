import { unless } from "./control.macro.js";

function fact(n, acc) {
  if (n === 0) return acc;
  return recur(n - 1, acc * n);
}

const f = (n, acc) => recur(n - 1, acc * n); // Should error

try {
  recur(1, 2);
} finally {
  console.log("done");
} // Should error

unless(x > 10, console.log("low"));
