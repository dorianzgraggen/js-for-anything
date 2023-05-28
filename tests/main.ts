const libName = `../src/target/debug/src.dll`;

// Open library and define exported symbols
const dylib = Deno.dlopen(libName, {
  add: { parameters: ['isize', 'isize'], result: 'isize' },
} as const);

const result = dylib.symbols.add(35, 34);

console.log(`Result from external addition of 35 and 34: ${result}`);
