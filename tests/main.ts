const libName = `../src/target/debug/src.dll`;

import {
  greet,
  init,
  register_function,
  print_function_list,
  poll_task,
} from '../src/bindings/bindings.ts';

type callback = [string, () => any];

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

// console.log(greet('Brudi'));

print_function_list();

console.log('--');

register_function(...find(callbacks, 'demo'));

print_function_list();

init();

while (true) {
  let id = poll_task();
  while (id != -1) {
    const callback = callbacks[id];

    callback[1]();

    id = poll_task();
  }

  await new Promise((resolve) => setTimeout(resolve, 100));
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
