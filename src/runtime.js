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

globalThis.callbacks = [];

globalThis.addEventListener = (type, callback) => {
  let index = callbacks.length;
  callbacks.push(callback)

  ops.op_register_callback(type, index);
}

globalThis.handle_events = () => {
  const events = ops.op_get_events();
  // console2.log("pending events:");
  // console2.log(events);

  events.forEach(([index, stringified_data]) => {
    console2.log("getting event index: " + index + " with data: " + stringified_data)
    const callback = callbacks[index];
    callback(JSON.parse(stringified_data));
  });
}

globalThis.setTimeout = (callback, delay) => {
  core.opAsync("op_set_timeout", delay).then(callback);
};

globalThis.task = (id, ...args) => {
  const res_string = ops.op_task(id, JSON.stringify(args));
  return JSON.parse(res_string);
}

globalThis.shouldExit = () => {
  return ops.op_should_exit();
}
