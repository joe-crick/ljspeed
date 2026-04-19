function fact(n, acc) {
  if (n === 0) return acc;
  return recur(n - 1, acc * n);
}

console.log(fact(5, 1));
