let functions = [/** will be populated before it runs */];

functions.forEach(f => {
  let str = f[0];
  let id = f[1];

  globalThis[str] = (...args) => task(id, ...args);
});
