use deno_core::error::custom_error;
use deno_core::v8::Boolean;
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, sync::Mutex, time::SystemTime};

use libc::c_char;
use std::ffi::CStr;

use colored::Colorize;

use deno_bindgen::deno_bindgen;
use deno_core::{anyhow::Error, error::AnyError, include_js_files, op, Extension};

static FUNCTION_MAP: Lazy<Mutex<HashMap<u32, (String, bool)>>> = Lazy::new(|| {
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
    // rs_log("[RS]: args: {args}".into());
    // let mut v = TASKS.lock().unwrap();
    // v.push_back((id, args));
    let now = SystemTime::now();

    if let Some(callback) = unsafe { task_callback } {
        // Create a sample string to pass to the callback
        let c_string = std::ffi::CString::new(args).unwrap();

        let result = unsafe {
            let c_string_ptr = callback(id, c_string.as_ptr());
            CStr::from_ptr(c_string_ptr).to_string_lossy().into_owned()
        };

        // rs_log(format!("___ returned string: {}", result));
        // rs_log(format!(
        //     "+++++++++++++ task callback took: {} microsec",
        //     now.elapsed().unwrap().as_micros()
        // ));
        return Ok(result);
    }

    return Err(custom_error("OP_Error", "i just couldn't"));

    // unsafe {
    //     if let Some(callback) = global_callback {
    //         let result = callback(99, 700);
    //         rs_log(format!("--------- new result {}", result));
    //     }
    // }

    let now = SystemTime::now();
    {
        let mut current_function = CURRENT_FUNCTION.lock().unwrap();
        current_function.0 = id;
        current_function.1 = args;
        *WAITING.lock().unwrap() = true;
    }
    rs_log(format!(
        "current_function {} ms",
        now.elapsed().unwrap().as_millis()
    ));

    rs_log("[RS]: started WAITING".into());
    while *WAITING.lock().unwrap() {}
    rs_log("[RS]: stopped waiting in op_task".into());

    rs_log(format!("waiting {} ms", now.elapsed().unwrap().as_millis()));

    let result = { CURRENT_RESULT.lock().unwrap().clone() };
    rs_log(format!("[RS]: received {} in op_task", result));
    rs_log(format!(
        "task took {} ms",
        now.elapsed().unwrap().as_millis()
    ));

    Ok(result)
}

#[op]
fn op_print(msg: String) -> Result<(), AnyError> {
    let formatted = format!("{} {}", "[JS]".yellow(), msg);
    rs_log(formatted);
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

#[op]
fn op_should_exit() -> Result<bool, AnyError> {
    let should_exit = SHOULD_EXIT.load(Ordering::Relaxed);
    Ok(should_exit)
}

#[deno_bindgen]
fn send_event(event_type: &str, data: &str) {
    real_send_event(event_type.to_string(), data.to_string());
}

#[no_mangle]
pub extern "C" fn set_task_callback(callback: extern "C" fn(u8, *const c_char) -> *const c_char) {
    unsafe { task_callback = Some(callback) };
    // // Create a sample string to pass to the callback
    // let my_string = "Hello from Rust!";
    // let c_string = std::ffi::CString::new(my_string).unwrap();

    // rs_log(format!("struggling"));

    // let returned_string = unsafe {
    //     let c_string_ptr = callback(22, c_string.as_ptr());
    //     CStr::from_ptr(c_string_ptr).to_string_lossy().into_owned()
    // };

    // rs_log(format!("___ returned string: {}", returned_string));
}

static mut global_callback: Option<extern "C" fn(i32, i32) -> i32> = None;
static mut task_callback: Option<extern "C" fn(u8, *const c_char) -> *const c_char> = None;

#[no_mangle]
pub extern "C" fn my_rust_function(callback: extern "C" fn(i32, i32) -> i32) {
    // Call the provided callback function
    let result = callback(10, 20);
    unsafe { global_callback = Some(callback) };
    // Do something with the result
    // ...
    rs_log(format!("************ result: {}", result));
}

#[no_mangle]
pub unsafe extern "C" fn send_event_c_str(event_type: *const c_char, data: *const c_char) {
    real_send_event(c_str_to_rust_string(event_type), c_str_to_rust_string(data))
}

fn real_send_event(event_type: String, data: String) {
    rs_log(format!("got event_type {} and data {}", event_type, data));

    let id = {
        let callbacks = CALLBACKS.lock().unwrap();
        *callbacks.get(&event_type).unwrap()
    };
    let mut events = PENDING_EVENTS.lock().unwrap();
    events.push((id, data));

    rs_log("[RS]: has set waiting to false!".into());
}

#[deno_bindgen]
fn send_result(result: &str) {
    send_result_real(result.to_string());
}

#[no_mangle]
pub unsafe extern "C" fn send_result_c_str(s: *const c_char) {
    send_result_real(c_str_to_rust_string(s));
}

fn send_result_real(result: String) {
    let mut current_result = CURRENT_RESULT.lock().unwrap();
    *current_result = result;
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

    if id != 0 {
        rs_log(format!("[RS]: pending: id({}), args({})", id, args));
    }

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
}

#[deno_bindgen]
pub extern "C" fn register_function(name: &str, id: u32) {
    real_register_function(name.to_string(), id, false);
}

fn real_register_function(name: String, id: u32, is_constructor: bool) {
    let mut c = FUNCTION_MAP.lock().unwrap();
    rs_log(format!("xxxxxxxxxxxxxxxx [RS]: Registering: {}", id));
    c.insert(id, (name, is_constructor));
}

#[no_mangle]
pub unsafe extern "C" fn register_function_c_str(s: *const c_char, id: u32, is_constructor: bool) {
    real_register_function(c_str_to_rust_string(s), id, is_constructor);
}

#[deno_bindgen]
pub extern "C" fn init() {
    real_init("app.js".to_string());
}

#[no_mangle]
pub unsafe extern "C" fn init_from_path(path: *const c_char) {
    real_init(c_str_to_rust_string(path));
}

fn real_init(path: String) {
    std::thread::spawn(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .thread_name("js_plugin thread")
            .enable_all()
            .build()
            .unwrap();

        if let Err(error) = runtime.block_on(start_runtime(path)) {
            rs_log(format!("[RS]: blocked, error: {error}"));
        }
    });
}

async fn start_runtime(path: String) -> Result<(), AnyError> {
    let copy = path.clone().replace(".js", ".copy.js");
    std::fs::copy(path, &copy)?;
    let contents = std::fs::read_to_string(&copy)?;

    rs_log("copied".into());

    let prelude = build_prelude();

    let both = format!("{}{}", prelude, contents);

    rs_log(both.clone());

    // std::thread::sleep(Duration::from_millis(15000));

    std::fs::write(&copy, both)?;

    let main_module = deno_core::resolve_path(&copy)?;

    let runjs_extension = Extension::builder("runjs")
        .esm(include_js_files!("runtime.js",))
        .ops(vec![
            op_write_file::decl(),
            op_task::decl(),
            op_print::decl(),
            op_register_callback::decl(),
            op_get_events::decl(),
            op_set_timeout::decl(),
            op_should_exit::decl(),
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
    for (id, (name, is_constructor)) in functions.into_iter() {
        to_insert = format!("{}['{}', {}, {}],", to_insert, name, id, is_constructor);
    }

    raw_prelude.replace("/** will be populated before it runs */", &to_insert)
}

// don't know how to pass around function pointers and stuff so using this as a workaround
static mut RS_LOG: Lazy<Mutex<[u8; 4096]>> = Lazy::new(|| Mutex::new([0; 4096]));
static mut RS_LOG_CLONE: [u8; 4096] = [0; 4096];

#[no_mangle]
pub extern "C" fn clear_log_file() {
    match OpenOptions::new()
        .create(true)
        .write(true)
        .open("C:\\projects\\game-engine\\unity-js\\Assets\\rs-log.txt")
    {
        Ok(file) => {
            file.set_len(0).unwrap();
        }
        Err(err) => {
            rs_log(format!("Error getting log file {:?}", err));
        }
    }
}

static LOG_TO_FILE: AtomicBool = AtomicBool::new(false);
static SHOULD_EXIT: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "C" fn set_log_to_file(log_to_file: bool) {
    LOG_TO_FILE.store(log_to_file, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn stop() {
    SHOULD_EXIT.store(true, Ordering::Relaxed);
}

fn rs_log(mut txt: String) {
    // return;
    if LOG_TO_FILE.load(Ordering::Relaxed) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("C:\\projects\\game-engine\\unity-js\\Assets\\rs-log.txt")
            .unwrap();

        writeln!(file, "[RS]\n{}", txt).unwrap();
        return;
    }

    txt.push_str("\n");
    // let formatted = format!("[RS] {} \n", txt);
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

fn c_str_to_rust_string(s: *const c_char) -> String {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };

    c_str.to_str().unwrap().to_string()
}
