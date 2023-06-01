const { core } = Deno;
const { ops } = core;

function argsToMessage(...args) {
  return args.map((arg) => JSON.stringify(arg)).join(" ");
}

const console = {
  log: (...args) => {
    ops.op_print(argsToMessage(...args));
  }
};

globalThis.console2 = console;

globalThis.writeFile = (path, contents) => {
  return ops.op_write_file(path, contents);
};



globalThis.task = (id, ...args) => {
  const res_string = ops.op_task(id, JSON.stringify(args));
  return JSON.parse(res_string);
}

