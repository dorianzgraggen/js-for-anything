use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};

use colored::Colorize;

use deno_bindgen::deno_bindgen;
use deno_core::{anyhow::Error, error::AnyError, include_js_files, op, Extension};

static FUNCTION_MAP: Lazy<Mutex<HashMap<u32, String>>> = Lazy::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

static CURRENT_FUNCTION: Lazy<Mutex<(u8, String)>> = Lazy::new(|| Mutex::new((0, String::new())));
static CURRENT_RESULT: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));
static WAITING: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

// doing this bookkeeping because at some point i want to have event listeners on objects
static CALLBACKS: Lazy<Mutex<HashMap<String, u8>>> = Lazy::new(|| Mutex::new(HashMap::new()));

static PENDING_EVENTS: Lazy<Mutex<Vec<(u8, String)>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[op]
fn op_write_file(path: String, contents: String) -> Result<(), AnyError> {
    let res = std::fs::write(path, contents);

    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::new(e)),
    }
}

#[op]
fn op_task(id: u8, args: String) -> Result<String, AnyError> {
    rs_log("[RS]: args: {args}".into());
    // let mut v = TASKS.lock().unwrap();
    // v.push_back((id, args));

    {
        let mut current_function = CURRENT_FUNCTION.lock().unwrap();
        current_function.0 = id;
        current_function.1 = args;
        *WAITING.lock().unwrap() = true;
    }

    rs_log("[RS]: started WAITING".into());
    while *WAITING.lock().unwrap() {}
    rs_log("[RS]: stopped waiting in op_task".into());
    let result = { CURRENT_RESULT.lock().unwrap().clone() };
    rs_log(format!("[RS]: received {} in op_task", result));

    Ok(result)
}

#[op]
fn op_print(msg: String) -> Result<(), AnyError> {
    let formatted = format!("{} {}", "[JS]".yellow(), msg);
    rs_log(format!("{}", formatted));
    Ok(())
}

#[op]
async fn op_set_timeout(delay: u64) -> Result<(), AnyError> {
    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    Ok(())
}

#[op]
fn op_register_callback(callback_type: String, index: u8) -> Result<(), AnyError> {
    let mut callbacks = CALLBACKS.lock().unwrap();
    callbacks.insert(callback_type, index);
    rs_log(format!("[RS]: callbacks {:#?}", callbacks));
    Ok(())
}

#[op]
fn op_get_events() -> Result<Vec<(u8, String)>, AnyError> {
    let pending_events = {
        let mut events = PENDING_EVENTS.lock().unwrap();
        let clone = events.clone();
        events.clear();
        clone
    };

    Ok(pending_events)
}

#[deno_bindgen]
fn send_event(event_type: &str, data: &str) {
    let id = {
        let callbacks = CALLBACKS.lock().unwrap();
        *callbacks.get(event_type).unwrap()
    };
    let mut events = PENDING_EVENTS.lock().unwrap();
    events.push((id, String::from(data)));

    rs_log("[RS]: has set waiting to false!".into());
}

#[deno_bindgen]
fn send_result(result: &str) {
    let mut current_result = CURRENT_RESULT.lock().unwrap();
    *current_result = result.to_string();
    rs_log("[RS]: will set waiting to false".into());
    {
        let mut current_function = CURRENT_FUNCTION.lock().unwrap();
        current_function.0 = 0;
        current_function.1 = String::new();
    }

    *WAITING.lock().unwrap() = false;

    rs_log("[RS]: has set waiting to false!".into());
}

static mut JSON_ARGS_BUFFER: [u8; 1024] = [0; 1024];

#[no_mangle]
fn poll_pending_invocations() -> *const u8 {
    let (id, args) = { CURRENT_FUNCTION.lock().unwrap().clone() };

    rs_log(format!("[RS]: pending: id({}), args({})", id, args));

    unsafe {
        if id != 0 {
            JSON_ARGS_BUFFER[0] = id;
            // rs_log("[RS]: id is: {:#?}", id);

            let len_in_bytes: [u8; 4] = (args.bytes().len() as u32).to_ne_bytes();
            // rs_log("[RS]: len_in_bytes: {:#?}", len_in_bytes);

            JSON_ARGS_BUFFER[1] = len_in_bytes[0];
            JSON_ARGS_BUFFER[2] = len_in_bytes[1];
            JSON_ARGS_BUFFER[3] = len_in_bytes[2];
            JSON_ARGS_BUFFER[4] = len_in_bytes[3];

            // rs_log("[RS]: bytes: {:#?}", args.bytes());
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
    let fn_map = FUNCTION_MAP.lock().unwrap();
    rs_log(format!("[RS]: {:?}", fn_map));

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open("C:\\projects\\game-engine\\unity-js\\Assets\\Plugins\\rs-log.txt")
        .unwrap();

    writeln!(file, "A new line! {:?}", fn_map).unwrap();
}

#[deno_bindgen]
pub extern "C" fn register_function(name: &str, id: u32) {
    let mut c = FUNCTION_MAP.lock().unwrap();
    rs_log(format!("[RS]: Registering: {}", id));
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
            rs_log(format!("[RS]: error: {error}"));
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
        .ops(vec![
            op_write_file::decl(),
            op_task::decl(),
            op_print::decl(),
            op_register_callback::decl(),
            op_get_events::decl(),
            op_set_timeout::decl(),
        ])
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

// don't know how to pass around function pointers and stuff so using this as a workaround
static mut RS_LOG: Lazy<Mutex<[u8; 4096]>> = Lazy::new(|| Mutex::new([0; 4096]));
static mut RS_LOG_CLONE: [u8; 4096] = [0; 4096];

fn rs_log(mut txt: String) {
    // let formatted = format!("[RS] {} \n", txt);
    txt.push_str("\n");
    let len = txt.bytes().len() as u32;

    // println!("logging {}", txt);

    let mut buf = unsafe { RS_LOG.lock().unwrap() };
    let current_len = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);

    let combined_len = len + current_len;
    // println!(
    //     "current_len {}, len {}, combined_len {}",
    //     current_len, len, combined_len
    // );

    let len_in_bytes: [u8; 4] = combined_len.to_ne_bytes();

    buf[0] = len_in_bytes[0];
    buf[1] = len_in_bytes[1];
    buf[2] = len_in_bytes[2];
    buf[3] = len_in_bytes[3];

    for (i, byte) in txt.bytes().enumerate() {
        buf[i + 4 + (current_len as usize)] = byte;
    }
}

#[no_mangle]
pub unsafe extern "C" fn get_rs_log() -> *const u8 {
    RS_LOG_CLONE = unsafe { *RS_LOG.lock().unwrap() };
    let mut buf = unsafe { RS_LOG.lock().unwrap() };

    let zero: usize = 0;
    let len_in_bytes = zero.to_ne_bytes();

    buf[0] = len_in_bytes[0];
    buf[1] = len_in_bytes[1];
    buf[2] = len_in_bytes[2];
    buf[3] = len_in_bytes[3];

    RS_LOG_CLONE.as_ptr()
}
