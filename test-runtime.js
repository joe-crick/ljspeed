import { useMap } from "./control.macro.js";

const result = useMap([1, 2, 3], x => x * 2);
console.log(result);

const evens = filter(x => x % 2 === 0, [1, 2, 3, 4, 5, 6]);
console.log(evens);
