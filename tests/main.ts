import {
  init,
  register_function,
  print_function_list,
  send_result,
} from '../src/bindings/bindings.ts';

type callback = [string, (...args: any[]) => any];

export function find(callbacks: callback[], name: string): [string, number] {
  const index = callbacks.findIndex((f) => f[0] == name);
  return [name, index];
}

const callbacks: callback[] = [
  [
    'demo',
    () => {
      log('i am a demo');
    },
  ],
  [
    'sayHi',
    (name: string) => {
      log(`Hi, ${name} 👋`);
    },
  ],
  [
    'multiply',
    (a: number, b: number) => {
      return a * b;
    },
  ],
  [
    'returnObject',
    () => {
      return { a: 22, b: 'meow' };
    },
  ],
];

const dylib = Deno.dlopen('../src/target/debug/js_for_anything.dll', {
  poll_pending_invocations: { parameters: [], result: 'pointer' },
});

print_function_list();

callbacks.forEach((callback, i) => {
  register_function(callback[0], i + 1);
});

print_function_list();

init();

function pollPendingInvocations() {
  const ptr = dylib.symbols.poll_pending_invocations();

  // @ts-ignore idk
  const ptrView = new Deno.UnsafePointerView(ptr);

  const id = get_id();

  if (id == 0) {
    return {
      id,
      args: [],
    };
  }

  const arg_length = get_arg_length();
  const args = get_args();

  function get_id() {
    const into = new Uint8Array(1);
    ptrView.copyInto(into);
    return into[0];
  }

  function get_arg_length() {
    const into = new Uint8Array(4);
    ptrView.copyInto(into, 1);
    return new Uint32Array(into.buffer)[0];
  }

  function get_args() {
    const into = new Uint8Array(arg_length + 4 - (arg_length % 4));
    ptrView.copyInto(into, 5);
    const s = new TextDecoder().decode(into.slice(0, arg_length));
    return JSON.parse(s);
  }

  return { id, args };
}

log('polled');

while (true) {
  let { id, args } = pollPendingInvocations();
  while (id != 0) {
    const callback = callbacks[id - 1];
    log('calling', callback[0], 'with args', args);
    // TODO: 0 doesnt work as a return value
    const result = callback[1](...args) || '';

    log('result is', result);

    // TODO: use separate function when callback had no return value
    send_result(JSON.stringify(result));

    // TODO: wait

    const t = pollPendingInvocations();
    id = t.id;
    args = t.args;
    log('id is now', id, 'args is', args);
  }

  await new Promise((resolve) => setTimeout(resolve, 100));
}

function log(...args: any[]) {
  console.log('%c[TS]', 'color: blue', ...args);
}
