const { core } = Deno;
const { ops } = core;

function argsToMessage(...args) {
  return args.map((arg) => JSON.stringify(arg)).join(" ");
}

const console = {
  log: (...args) => {
    core.print(`[out]: ${argsToMessage(...args)}\n`, false);
  },
  error: (...args) => {
    core.print(`[err]: ${argsToMessage(...args)}\n`, true);
  },
};

globalThis.console2 = console;

globalThis.writeFile = (path, contents) => {
  return ops.op_write_file(path, contents);
};

globalThis.task = (id) => {
  return ops.op_task(id);
}

