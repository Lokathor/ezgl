#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(clippy::unit_arg)]
#![allow(clippy::result_unit_err)]
#![warn(clippy::missing_inline_in_public_items)]

use core::{ptr::null, slice::from_raw_parts as slice_from_raw_parts};
use std::sync::{LockResult, PoisonError};

use gl_constants::*;
use gl_struct_loader::*;
use gl_types::*;

#[inline]
pub fn set_stderr_debug_message_callback() -> Result<(), ()> {
  let gl = GL.read().unwrap_or_else(PoisonError::into_inner);
  if gl.has_loaded().DebugMessageCallback() {
    Ok(unsafe {
      gl.DebugMessageCallback(Some(stderr_debug_message_callback), null())
    })
  } else if gl.has_loaded().DebugMessageCallbackKHR() {
    // GLES uses an alternate name but the extension operates the same.
    Ok(unsafe {
      gl.DebugMessageCallbackKHR(Some(stderr_debug_message_callback), null())
    })
  } else {
    Err(())
  }
}

unsafe extern "system" fn stderr_debug_message_callback(
  source: GLenum, ty: GLenum, id: GLuint, severity: GLenum, length: GLsizei,
  message: *const GLchar, _user_data: *const c_void,
) {
  // assert the correct signature
  const _: GLDEBUGPROC = Some(stderr_debug_message_callback);
  //
  let source = match source {
    GL_DEBUG_SOURCE_API => "API",
    GL_DEBUG_SOURCE_WINDOW_SYSTEM => "Window",
    GL_DEBUG_SOURCE_SHADER_COMPILER => "ShaderCompiler",
    GL_DEBUG_SOURCE_THIRD_PARTY => "3rdParty",
    GL_DEBUG_SOURCE_APPLICATION => "App",
    _ => "OtherSrc",
  };
  let ty = match ty {
    GL_DEBUG_TYPE_ERROR => "Error",
    GL_DEBUG_TYPE_DEPRECATED_BEHAVIOR => "Deprecated",
    GL_DEBUG_TYPE_UNDEFINED_BEHAVIOR => "Undefined",
    GL_DEBUG_TYPE_PORTABILITY => "Portability",
    GL_DEBUG_TYPE_PERFORMANCE => "Performance",
    GL_DEBUG_TYPE_MARKER => "Marker",
    _ => "OtherTy",
  };
  let severity = match severity {
    GL_DEBUG_SEVERITY_HIGH => "High",
    GL_DEBUG_SEVERITY_MEDIUM => "Medium",
    GL_DEBUG_SEVERITY_LOW => "Low",
    GL_DEBUG_SEVERITY_NOTIFICATION => "Note",
    _ => "OtherSeverity",
  };
  let message_bytes = unsafe {
    slice_from_raw_parts(message.cast::<u8>(), length.try_into().unwrap())
  };
  let message = String::from_utf8_lossy(message_bytes);
  eprintln!("{source}>{ty}>{id}>{severity}>{message}");
}
