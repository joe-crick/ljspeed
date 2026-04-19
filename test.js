import { unless, nested } from "./control.macro.js";

nested(x > 10, console.log("nested expansion worked"));
