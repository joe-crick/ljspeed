export const useMap = defineMacro((ctx, arr, fn) => {
  return ctx.syntax.expression`map(${arr}, ${fn})`;
});
