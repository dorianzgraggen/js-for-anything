const libName = `../src/target/debug/src.dll`;

// Open library and define exported symbols
const dylib = Deno.dlopen(libName, {
  add: { parameters: ['isize', 'isize'], result: 'isize' },
  init: { parameters: [], result: 'void' },
} as const);

const js_runtime = dylib.symbols;

/**
 * TODO:
 * globalThis.player.setPosition = function(...args) {
 *    bridge.call({{id}}, JSON.stringify(...args))
 * }
 *
 */
// js_runtime.registerMethod('player.setPosition', 0);

js_runtime.init();

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
