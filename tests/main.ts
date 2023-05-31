const libName = `../src/target/debug/src.dll`;

import {
  greet,
  init,
  register_function,
  print_function_list,
  theme_song_generate,
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
      console.log('i am a demo');
    },
  ],
  [
    'sayHi',
    (name: string) => {
      console.log(`Hi, ${name} 👋`);
    },
  ],
  [
    'multiply',
    (a: number, b: number) => {
      return a * b;
    },
  ],
];

// Open library and define exported symbols

// const dylib = Deno.dlopen(libName, {
//   add: { parameters: ['isize', 'isize'], result: 'isize' },
//   init: { parameters: [], result: 'void' },
//   how_many_characters: { parameters: [] },
// } as const);

// const js_runtime = dylib.symbols;

/**
 * TODO:
 * globalThis.player.setPosition = function(...args) {
 *    bridge.call({{id}}, JSON.stringify(...args))
 * }
 *
 */

const dylib = Deno.dlopen('../src/target/debug/js_for_anything.dll', {
  return_buffer: { parameters: [], result: 'pointer' },
  return_string_buffer: { parameters: [], result: 'pointer' },
  poll_task: { parameters: [], result: 'pointer' },
  // is_null_ptr: { parameters: ['pointer'], result: 'u8' },
});

if (false) {
  // console.log(greet('Brudi'));
  console.log('*');
  const ptr = dylib.symbols.return_buffer();
  const ptrView = new Deno.UnsafePointerView(ptr);
  const into = new Uint8Array(8);
  ptrView.copyInto(into);
  console.log(into);
  console.log();
}

if (false) {
  console.log('======================');
  const ptr = dylib.symbols.return_string_buffer();
  console.log(ptr);

  const ptrView = new Deno.UnsafePointerView(ptr);
  const into = new Uint8Array(1024);
  ptrView.copyInto(into);
  console.log(into);
  const s = new TextDecoder().decode(into);
  console.log(s);
  console.log('======================');
  // console.log(n);
}

// console.log(theme_song_generate(3));

print_function_list();

console.log('--');

callbacks.forEach((callback, i) => {
  register_function(callback[0], i + 1);
});

print_function_list();

init();

console.log('--initttttttttt');

function poll_task_2() {
  const ptr = dylib.symbols.poll_task();

  console.log(ptr);
  let id = 0;
  {
    console.log('============ METHOD ID');
    // read function id
    const ptrView = new Deno.UnsafePointerView(ptr);
    console.log(ptrView);
    const into = new Uint8Array(1);
    ptrView.copyInto(into);
    console.log(into);
    // const s = new TextDecoder().decode(into);
    // console.log(s);
    id = into[0];
  }

  if (id == 0) {
    return;
  }

  let arg_length = 0;
  {
    // read args length
    console.log('============ ARG LENGTH');
    const ptrView = new Deno.UnsafePointerView(ptr);
    console.log(ptrView);
    const into = new Uint8Array(4);
    ptrView.copyInto(into, 1);
    console.log(into);

    arg_length = new Uint32Array(into.buffer)[0];
    console.log('length', arg_length);

    // const s = new TextDecoder().decode(into);
    // console.log(s);
  }

  {
    console.log('============ ARGS');
    const ptrView = new Deno.UnsafePointerView(ptr);

    const into = new Uint8Array(arg_length + 4 - (arg_length % 4));
    ptrView.copyInto(into, 5);
    console.log(into);

    const s = new TextDecoder().decode(into.slice(0, arg_length));
    console.log('arguments:', s);

    const args = JSON.parse(s);
    console.log({ args });
  }

  return;
}

console.log('polled');

while (true) {
  // let id = poll_task();
  // while (id != -1) {
  //   const callback = callbacks[id];

  //   callback[1]();

  //   let id = poll_task();
  // }

  poll_task_2();

  await new Promise((resolve) => setTimeout(resolve, 1000));
}

// const ops = {
//   playerSetPosition: (json_args) => {
//     const args = JSON.parse(json_args);

//     console.log(`Player is now at ${args.x}/${args.y}/${args.z}`);
//   },
// };

// while (true) {
//   const tasks = js_runtime.pollTasks();

//   tasks.forEach((task) => {
//     switch (task.id) {
//       case 0:
//         ops.playerSetPosition(task.arguments);
//         break;

//       default:
//         break;
//     }
//   });

//   await new Promise((resolve) => setTimeout(resolve, 100));
// }

// const result = dylib.symbols.add(35, 34);

// console.log(`Result from external addition of 35 and 34: ${result}`);
