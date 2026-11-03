// core/tools/lib/color.mjs — tiny ANSI helper with NO_COLOR support.

const enabled =
  process.env.NO_COLOR === undefined &&
  process.env.FORCE_COLOR !== "0" &&
  process.stdout.isTTY === true;

const wrap = (code, text) => (enabled ? `\u001b[${code}m${text}\u001b[0m` : text);

export const bold = (s) => wrap("1", s);
export const dim = (s) => wrap("2", s);
export const red = (s) => wrap("31", s);
export const green = (s) => wrap("32", s);
export const yellow = (s) => wrap("33", s);
export const blue = (s) => wrap("34", s);
export const cyan = (s) => wrap("36", s);

export const pass = (s) => green(`PASS ${s}`);
export const fail = (s) => red(`FAIL ${s}`);
export const warn = (s) => yellow(`WARN ${s}`);
export const info = (s) => cyan(s);
export const heading = (s) => bold(blue(s));
