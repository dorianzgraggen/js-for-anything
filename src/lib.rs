use std::rc::Rc;

use deno_core::{anyhow::Error, error::AnyError, include_js_files, op, Extension};

#[no_mangle]
pub extern "C" fn add(a: isize, b: isize) -> isize {
    a + b
}

#[op]
fn op_write_file(path: String, contents: String) -> Result<(), AnyError> {
    let res = std::fs::write(path, contents);

    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::new(e)),
    }
}

#[no_mangle]
pub extern "C" fn init() {
    std::thread::spawn(|| {
        let file_path = "./app.js";

        let runtime = tokio::runtime::Builder::new_current_thread()
            .thread_name("js_plugin thread")
            .enable_all()
            .build()
            .unwrap();

        if let Err(error) = runtime.block_on(start_runtime(file_path)) {
            eprintln!("error: {error}");
        }
    });
}

async fn start_runtime(file_path: &str) -> Result<(), AnyError> {
    let main_module = deno_core::resolve_path(file_path)?;
    let runjs_extension = Extension::builder("runjs")
        .esm(include_js_files!("runtime.js",))
        .ops(vec![op_write_file::decl()])
        .build();

    let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: vec![runjs_extension],
        ..Default::default()
    });

    let mod_id = js_runtime.load_main_module(&main_module, None).await?;

    let result = js_runtime.mod_evaluate(mod_id);

    js_runtime.run_event_loop(false).await?;
    result.await?
}
