use std::rc::Rc;

use deno_bindgen::deno_bindgen;
use deno_core::{anyhow::Error, error::AnyError, include_js_files, op, Extension};

use libc::c_char;
use std::ffi::CStr;

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

#[deno_bindgen]
fn greet(name: &str) -> String {
    println!("Hello, {}!", name);

    let a = "ussee!";

    a.into()
}

#[deno_bindgen]
struct Input {
    a: i32,
    b: i32,
}

#[deno_bindgen]
fn mul(input: Input) -> i32 {
    input.a * input.b
}

#[no_mangle]
pub extern "C" fn register_method() {}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn how_many_characters(s: *const c_char) -> u32 {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };

    let r_str = c_str.to_str().unwrap();
    r_str.chars().count() as u32
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
