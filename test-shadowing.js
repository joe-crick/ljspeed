import { unless } from "./control.macro.js";

unless(x > 10, console.log("this is a macro"));

function test(unless) {
  unless(x > 10, console.log("this is a local function, NOT a macro"));
}
