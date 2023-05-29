const libName = `../src/target/debug/src.dll`;

import { mul, Input, greet } from '../src/bindings/bindings.ts';

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

console.log(mul({ a: 12, b: 3 }));
console.log(greet('Brudi'));

// console.log(js_runtime.how_many_characters('göes to élevên'));

// js_runtime.registerMethod('demo', 0);

// js_runtime.init();

while (true) {
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
