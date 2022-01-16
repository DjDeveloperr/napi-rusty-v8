use libloading::os::unix::Library;
use libloading::os::unix::*;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_void;
use std::ptr;

struct Env<'a, 'b, 'c> {
  scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
}

type napi_env = *mut c_void;
type napi_value = *mut c_void;
type napi_callback_info = *mut c_void;

pub type napi_callback = Option<
  unsafe extern "C" fn(env: napi_env, info: napi_callback_info) -> napi_value,
>;

// default = 0
pub type napi_property_attributes = i32;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct napi_property_descriptor {
  pub utf8name: *const c_char,
  pub name: napi_value,
  pub method: napi_callback,
  pub getter: napi_callback,
  pub setter: napi_callback,
  pub value: napi_value,
  pub attributes: napi_property_attributes,
  pub data: *mut c_void,
}

#[no_mangle]
pub unsafe extern "C" fn napi_define_properties(
  env: napi_env,
  obj: napi_value,
  property_count: usize,
  properties: *const napi_property_descriptor,
) {
  let mut env = &mut *(env as *mut Env);
  println!("napi_define_properties");

  let object: v8::Local<v8::Object> = *(obj as *mut v8::Local<v8::Object>);
  let properties = std::slice::from_raw_parts(properties, property_count);
  for property in properties {
    let name = CStr::from_ptr(property.utf8name).to_str().unwrap();

    let name = v8::String::new(env.scope, name).unwrap();
    if let Some(method) = property.method {
      let external = v8::External::new(env.scope, method as *mut c_void);
      let method = v8::Function::builder(
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         _: v8::ReturnValue| {
          // This probably won't work. WIP. Replace it with a `println` to see if its reachable.
          let data = args.data().unwrap();
          let method_ptr = v8::Local::<v8::External>::try_from(data).unwrap();
          let cb =
            unsafe { *(method_ptr.value() as *mut napi_callback) }.unwrap();
          // TODO: Stuff
          unsafe { cb(ptr::null_mut(), ptr::null_mut()) };
        },
      )
      .data(external.into())
      .build(env.scope)
      .unwrap();

      object.set(env.scope, name.into(), method.into()).unwrap();
    }
  }
}

#[no_mangle]
pub unsafe extern "C" fn napi_module_register() {
  println!("napi_module_register");
  // no-op.
}

fn main() {
  // Initialize V8.
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();

  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());

  // Initializer callback.
  let library = unsafe {
    Library::open(
      Some("./example_module/target/release/libexample_module.so"),
      RTLD_LAZY,
    )
    .unwrap()
  };
  let init = unsafe {
    library
            .get::<unsafe extern "C" fn(env: napi_env, exports: napi_value) -> napi_value>(
                b"napi_register_module_v1",
            )
            .unwrap()
  };

  let mut handle_scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(handle_scope);
  let scope = &mut v8::ContextScope::new(handle_scope, context);

  let mut exports = v8::Object::new(scope);
  let mut env = Env { scope };
  unsafe {
    init(
      &mut env as *mut _ as *mut c_void,
      &mut exports as *mut _ as *mut c_void,
    )
  };
  let exports_str = v8::String::new(scope, "exports").unwrap();
  context
    .global(scope)
    .set(scope, exports_str.into(), exports.into())
    .unwrap();
  let script = v8::String::new(scope, "exports.hello()").unwrap();

  let script =
    v8::Script::compile(scope, script, None).expect("failed to compile script");

  script.run(scope).unwrap();
}
