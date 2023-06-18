let functions = [/** will be populated before it runs */];

functions.forEach(([str, id, is_constructor]) => {
  if (is_constructor) {
    globalThis[str] = function (...args) {

      const parsed = task(id, ...args);

      console2.log("parsed:" + parsed)

      const object_id = parsed.id;
      const methods = parsed.methods;

      Object.entries(methods).forEach(method => {
        this[method.name] = (...args) => task(method.id, ...args);
      });
    }

  } else {

    globalThis[str] = (...args) => task(id, ...args);
  }
});
