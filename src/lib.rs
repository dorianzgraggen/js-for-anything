use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::rc::Rc;
use std::{collections::HashMap, sync::Mutex};

use deno_bindgen::deno_bindgen;
use deno_core::{anyhow::Error, error::AnyError, include_js_files, op, Extension};

static FUNCTION_MAP: Lazy<Mutex<HashMap<u32, String>>> = Lazy::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

static TASKS: Lazy<Mutex<VecDeque<(u8, String)>>> = Lazy::new(|| {
    let v = VecDeque::new();
    Mutex::new(v)
});

#[op]
fn op_write_file(path: String, contents: String) -> Result<(), AnyError> {
    let res = std::fs::write(path, contents);

    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::new(e)),
    }
}

#[op]
fn op_task(id: u8, args: String) -> Result<(), AnyError> {
    println!("args: {args}");
    let mut v = TASKS.lock().unwrap();
    v.push_back((id, args));
    Ok(())
}

static mut JSON_ARGS_BUFFER: [u8; 1024] = [0; 1024];

#[no_mangle]
fn poll_task() -> *const u8 {
    let mut queue = TASKS.lock().unwrap();
    // println!("hahahahaha");

    unsafe {
        if let Some((id, args)) = queue.pop_front() {
            JSON_ARGS_BUFFER[0] = id;
            // println!("id is: {:#?}", id);

            let len_in_bytes: [u8; 4] = std::mem::transmute(args.bytes().len() as u32);
            // println!("len_in_bytes: {:#?}", len_in_bytes);

            JSON_ARGS_BUFFER[1] = len_in_bytes[0];
            JSON_ARGS_BUFFER[2] = len_in_bytes[1];
            JSON_ARGS_BUFFER[3] = len_in_bytes[2];
            JSON_ARGS_BUFFER[4] = len_in_bytes[3];

            // println!("bytes: {:#?}", args.bytes());
            for (i, byte) in args.bytes().enumerate() {
                JSON_ARGS_BUFFER[i + 5] = byte;
            }
        } else {
            JSON_ARGS_BUFFER[0] = 0;
        }

        JSON_ARGS_BUFFER.as_ptr()
    }
}

#[deno_bindgen]
fn print_function_list() {
    println!("{:?}", FUNCTION_MAP.lock().unwrap());
}

#[deno_bindgen]
pub extern "C" fn register_function(name: &str, id: u32) {
    let mut c = FUNCTION_MAP.lock().unwrap();
    println!("Registering: {}", id);
    c.insert(id, String::from(name));
}

#[deno_bindgen]
pub extern "C" fn init() {
    std::thread::spawn(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .thread_name("js_plugin thread")
            .enable_all()
            .build()
            .unwrap();

        if let Err(error) = runtime.block_on(start_runtime()) {
            eprintln!("error: {error}");
        }
    });
}

async fn start_runtime() -> Result<(), AnyError> {
    std::fs::copy("app.js", "copy.js")?;
    let contents = std::fs::read_to_string("copy.js")?;

    let prelude = build_prelude();
    let both = format!("{}{}", prelude, contents);
    std::fs::write("copy.js", both)?;

    let main_module = deno_core::resolve_path("copy.js")?;

    let runjs_extension = Extension::builder("runjs")
        .esm(include_js_files!("runtime.js",))
        .ops(vec![op_write_file::decl(), op_task::decl()])
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

fn build_prelude() -> String {
    let raw_prelude = include_str!("prelude.js");
    let functions = { FUNCTION_MAP.lock().unwrap().clone() };

    // builds list with elements like this: ['functionName', 0]
    let mut to_insert = String::from("");
    for (id, name) in functions.into_iter() {
        to_insert = format!("{}['{}',{}],", to_insert, name, id);
    }

    raw_prelude.replace("/** will be populated before it runs */", &to_insert)
}
