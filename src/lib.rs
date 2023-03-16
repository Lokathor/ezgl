#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(clippy::unit_arg)]
#![allow(clippy::result_unit_err)]
#![warn(clippy::missing_inline_in_public_items)]

pub use gl_constants;

use core::{
  num::NonZeroU32,
  ops::{Deref, DerefMut, Range},
  ptr::null,
  slice::from_raw_parts as slice_from_raw_parts,
};
use gl_constants::*;
use gl_struct_loader::*;
use gl_types::*;
use pixel_formats::{r32g32b32a32_Sfloat, r8g8b8a8_Srgb};

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

#[repr(transparent)]
pub struct EzGl(GlFns);
impl EzGl {
  #[inline]
  pub fn new_boxed() -> Box<Self> {
    unsafe { core::mem::transmute(GlFns::new_boxed()) }
  }
}
impl Deref for EzGl {
  type Target = GlFns;
  #[inline]
  #[must_use]
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
impl DerefMut for EzGl {
  #[inline]
  #[must_use]
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}
impl EzGl {
  #[inline]
  pub fn set_stderr_debug_message_callback(&self) -> Result<(), ()> {
    if self.has_loaded().DebugMessageCallback() {
      Ok(unsafe {
        self.DebugMessageCallback(Some(stderr_debug_message_callback), null())
      })
    } else if self.has_loaded().DebugMessageCallbackKHR() {
      // GLES uses an alternate name but the extension operates the same.
      Ok(unsafe {
        self
          .DebugMessageCallbackKHR(Some(stderr_debug_message_callback), null())
      })
    } else {
      Err(())
    }
  }
  #[inline]
  pub fn gen_vertex_array(&self) -> Result<VertexArrayObject, ()> {
    let mut obj = 0;
    unsafe { self.GenVertexArrays(1, &mut obj) };
    NonZeroU32::new(obj).ok_or(()).map(VertexArrayObject)
  }
  #[inline]
  pub fn bind_vertex_array(&self, vao: &VertexArrayObject) {
    unsafe { self.BindVertexArray(vao.0.get()) };
  }
  #[inline]
  pub fn clear_vertex_array_binding(&self) {
    unsafe { self.BindVertexArray(0) };
  }
  #[inline]
  pub fn delete_vertex_array(&self, vao: VertexArrayObject) {
    unsafe { self.DeleteVertexArrays(1, &vao.0.get()) };
  }
  #[inline]
  pub fn gen_buffer(&self) -> Result<BufferObject, ()> {
    let mut obj = 0;
    unsafe { self.GenBuffers(1, &mut obj) };
    NonZeroU32::new(obj).ok_or(()).map(BufferObject)
  }
  #[inline]
  pub fn bind_buffer(&self, target: BufferTarget, buffer: &BufferObject) {
    unsafe { self.BindBuffer(target as GLenum, buffer.0.get()) };
  }
  /// Allocate new storage for the buffer bound to `target` and copy this data
  /// into it.
  ///
  /// Khronos: [glBufferData](https://registry.khronos.org/OpenGL-Refpages/gl4/html/glBufferData.xhtml)
  #[inline]
  pub fn buffer_data(
    &self, target: BufferTarget, data: &[u8], usage: BufferUsageHint,
  ) {
    unsafe {
      self.BufferData(
        target as GLenum,
        data.len().try_into().unwrap(),
        data.as_ptr().cast::<c_void>(),
        usage as GLenum,
      )
    }
  }
  #[inline]
  pub fn create_shader(
    &self, shader_type: ShaderType,
  ) -> Result<ShaderObject, ()> {
    NonZeroU32::new(unsafe { self.CreateShader(shader_type as GLenum) })
      .ok_or(())
      .map(ShaderObject)
  }
  #[inline]
  pub fn set_shader_source(&self, shader: &ShaderObject, src: &str) {
    let s: *const GLchar = src.as_ptr().cast();
    let len: GLint = src.len().try_into().unwrap();
    unsafe { self.ShaderSource(shader.0.get(), 1, &s, &len) }
  }
  #[inline]
  pub fn compile_shader(&self, shader: &ShaderObject) {
    unsafe { self.CompileShader(shader.0.get()) }
  }
  #[inline]
  pub fn get_shader_compile_success(&self, shader: &ShaderObject) -> bool {
    let mut success = 0;
    unsafe { self.GetShaderiv(shader.0.get(), GL_COMPILE_STATUS, &mut success) }
    success != 0
  }
  #[inline]
  pub fn get_shader_info_log(&self, shader: &ShaderObject) -> Box<str> {
    let mut len = 0;
    unsafe { self.GetShaderiv(shader.0.get(), GL_INFO_LOG_LENGTH, &mut len) }
    if len == 0 {
      String::new().into_boxed_str()
    } else {
      let mut v: Vec<u8> = Vec::with_capacity(len.try_into().unwrap());
      let mut bytes_written = 0;
      unsafe {
        self.GetShaderInfoLog(
          shader.0.get(),
          v.capacity().try_into().unwrap(),
          &mut bytes_written,
          v.as_mut_ptr().cast::<GLchar>(),
        );
        v.set_len(bytes_written.try_into().unwrap());
      }
      String::from_utf8_lossy(&v).into_owned().into_boxed_str()
    }
  }
  #[inline]
  pub fn create_program(&self) -> Result<ProgramObject, ()> {
    NonZeroU32::new(unsafe { self.CreateProgram() })
      .ok_or(())
      .map(ProgramObject)
  }
  #[inline]
  pub fn attach_shader(&self, program: &ProgramObject, shader: &ShaderObject) {
    unsafe { self.AttachShader(program.0.get(), shader.0.get()) }
  }
  #[inline]
  pub fn link_program(&self, program: &ProgramObject) {
    unsafe { self.LinkProgram(program.0.get()) }
  }
  #[inline]
  pub fn get_program_link_success(&self, program: &ProgramObject) -> bool {
    let mut success = 0;
    unsafe { self.GetProgramiv(program.0.get(), GL_LINK_STATUS, &mut success) }
    success != 0
  }
  #[inline]
  pub fn get_program_info_log(&self, program: &ProgramObject) -> Box<str> {
    let mut len = 0;
    unsafe { self.GetProgramiv(program.0.get(), GL_INFO_LOG_LENGTH, &mut len) }
    if len == 0 {
      String::new().into_boxed_str()
    } else {
      let mut v: Vec<u8> = Vec::with_capacity(len.try_into().unwrap());
      let mut bytes_written = 0;
      unsafe {
        self.GetProgramInfoLog(
          program.0.get(),
          v.capacity().try_into().unwrap(),
          &mut bytes_written,
          v.as_mut_ptr().cast::<GLchar>(),
        );
        v.set_len(bytes_written.try_into().unwrap());
      }
      String::from_utf8_lossy(&v).into_owned().into_boxed_str()
    }
  }
  #[inline]
  pub fn use_program(&self, program: &ProgramObject) {
    unsafe { self.UseProgram(program.0.get()) }
  }
  #[inline]
  pub fn delete_shader(&self, shader: ShaderObject) {
    unsafe { self.DeleteShader(shader.0.get()) }
  }
  #[inline]
  pub fn delete_program(&self, program: ProgramObject) {
    unsafe { self.DeleteProgram(program.0.get()) }
  }
  #[inline]
  pub fn enable_vertex_attrib_array(&self, index: GLuint) {
    unsafe { self.EnableVertexAttribArray(index) }
  }
  #[inline]
  pub fn disable_vertex_attrib_array(&self, index: GLuint) {
    unsafe { self.DisableVertexAttribArray(index) }
  }
  #[inline]
  pub fn set_clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
    unsafe { self.ClearColor(red, green, blue, alpha) }
  }
  /// Clears one or more buffers.
  ///
  /// Bits can be from the following list:
  /// * `GL_COLOR_BUFFER_BIT`
  /// * `GL_DEPTH_BUFFER_BIT`
  /// * `GL_STENCIL_BUFFER_BIT`
  #[inline]
  pub fn clear(&self, mask: GLbitfield) {
    unsafe { self.Clear(mask) }
  }
  #[inline]
  pub fn clear_color_buffer(&self) {
    self.clear(GL_COLOR_BUFFER_BIT)
  }
  #[inline]
  pub fn clear_color_and_depth_buffer(&self) {
    self.clear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT)
  }
  #[inline]
  pub fn get_uniform_location(
    &self, program: &ProgramObject, name: &str,
  ) -> Option<ShaderLocation> {
    let name_z = format!("{name}\0");
    let r = unsafe {
      self.GetUniformLocation(program.0.get(), name_z.as_ptr().cast::<GLchar>())
    };
    if r != -1 {
      Some(ShaderLocation(r))
    } else {
      None
    }
  }
  #[inline]
  pub fn set_uniform_4f(
    &self, loc: ShaderLocation, v0: f32, v1: f32, v2: f32, v3: f32,
  ) {
    unsafe { self.Uniform4f(loc.0, v0, v1, v2, v3) };
  }
  #[inline]
  pub fn set_texture_wrap_s(&self, target: TextureTarget, wrap: TextureWrap) {
    unsafe {
      self.TexParameteri(target as GLenum, GL_TEXTURE_WRAP_S, wrap as GLint)
    }
  }
  #[inline]
  pub fn set_texture_wrap_t(&self, target: TextureTarget, wrap: TextureWrap) {
    unsafe {
      self.TexParameteri(target as GLenum, GL_TEXTURE_WRAP_T, wrap as GLint)
    }
  }
  #[inline]
  pub fn set_texture_border_color(
    &self, target: TextureTarget, color: &r32g32b32a32_Sfloat,
  ) {
    unsafe {
      self.TexParameterfv(
        target as GLenum,
        GL_TEXTURE_BORDER_COLOR,
        color as *const r32g32b32a32_Sfloat as *const f32,
      )
    }
  }
  #[inline]
  pub fn set_texture_min_filter(
    &self, target: TextureTarget, filter: MinFilter,
  ) {
    unsafe {
      self.TexParameteri(
        target as GLenum,
        GL_TEXTURE_MIN_FILTER,
        filter as GLint,
      )
    }
  }
  #[inline]
  pub fn set_texture_mag_filter(
    &self, target: TextureTarget, filter: MagFilter,
  ) {
    unsafe {
      self.TexParameteri(
        target as GLenum,
        GL_TEXTURE_MAG_FILTER,
        filter as GLint,
      )
    }
  }
  #[inline]
  pub fn gen_texture(&self) -> Result<TextureObject, ()> {
    let mut obj = 0;
    unsafe { self.GenTextures(1, &mut obj) };
    NonZeroU32::new(obj).ok_or(()).map(TextureObject)
  }
  #[inline]
  pub fn bind_texture(&self, target: TextureTarget, texture: &TextureObject) {
    unsafe { self.BindTexture(target as GLenum, texture.0.get()) };
  }
  #[inline]
  pub fn delete_texture(&self, texture: TextureObject) {
    unsafe { self.DeleteTextures(1, &texture.0.get()) }
  }
  #[inline]
  pub fn tex_image_2d(
    &self, target: TextureTarget, level: GLint, width: usize, height: usize,
    pixels: &[r8g8b8a8_Srgb],
  ) {
    assert!(width.checked_mul(height).unwrap() == pixels.len());
    unsafe {
      self.TexImage2D(
        target as GLenum,
        level,
        GL_SRGB8_ALPHA8 as GLint,
        width.try_into().unwrap(),
        height.try_into().unwrap(),
        0,
        GL_RGBA,
        GL_UNSIGNED_BYTE,
        pixels.as_ptr().cast::<c_void>(),
      )
    }
  }
  #[inline]
  pub fn generate_mipmap(&self, target: TextureTarget) {
    unsafe { self.GenerateMipmap(target as GLenum) };
  }
  #[inline]
  pub fn enable_framebuffer_srgb(&self, enabled: bool) {
    if enabled {
      unsafe { self.Enable(GL_FRAMEBUFFER_SRGB) };
    } else {
      unsafe { self.Disable(GL_FRAMEBUFFER_SRGB) };
    }
  }
  #[inline]
  pub fn enable_multisample(&self, enabled: bool) {
    if enabled {
      unsafe { self.Enable(GL_MULTISAMPLE) };
    } else {
      unsafe { self.Disable(GL_MULTISAMPLE) };
    }
  }
  #[inline]
  pub fn enable_depth_test(&self, enabled: bool) {
    if enabled {
      unsafe { self.Enable(GL_DEPTH_TEST) };
    } else {
      unsafe { self.Disable(GL_DEPTH_TEST) };
    }
  }
  #[inline]
  pub fn set_pixel_store_unpack_alignment(&self, align: usize) {
    debug_assert!([1, 2, 4, 8].contains(&align));
    unsafe { self.PixelStorei(GL_UNPACK_ALIGNMENT, align as GLint) }
  }
  #[inline]
  pub fn create_shader_with_source(
    &self, shader_type: ShaderType, src: &str,
  ) -> Result<ShaderObject, Box<str>> {
    let vertex_shader = self.create_shader(shader_type).map_err(|_| {
      String::from("Couldn't create a new shader ID.").into_boxed_str()
    })?;
    self.set_shader_source(&vertex_shader, src);
    self.compile_shader(&vertex_shader);
    if self.get_shader_compile_success(&vertex_shader) {
      Ok(vertex_shader)
    } else {
      let log = self.get_shader_info_log(&vertex_shader);
      self.delete_shader(vertex_shader);
      Err(log)
    }
  }
  #[inline]
  pub fn create_vertex_fragment_program(
    &self, vertex_src: &str, fragment_src: &str,
  ) -> Result<ProgramObject, Box<str>> {
    let v =
      self.create_shader_with_source(ShaderType::VertexShader, vertex_src)?;
    let f = self
      .create_shader_with_source(ShaderType::FragmentShader, fragment_src)?;
    let program = self.create_program().map_err(|_| {
      String::from("Couldn't create a new program ID.").into_boxed_str()
    })?;
    self.attach_shader(&program, &v);
    self.attach_shader(&program, &f);
    self.link_program(&program);
    if self.get_program_link_success(&program) {
      Ok(program)
    } else {
      let log = self.get_program_info_log(&program);
      self.delete_program(program);
      Err(log)
    }
  }
  /// The maximum number of texture image units that the sampler in the
  /// **fragment** shader can access.
  #[inline]
  pub fn get_max_fragment_texture_image_units(&self) -> i32 {
    // For legacy reasons this uses `GL_MAX_TEXTURE_IMAGE_UNITS`, but all other
    // shaders use enums called "max_{stage}_texture_image_units", so for
    // consistency this method has a name that matches that pattern even though
    // the enum name doesn't match.
    let mut out = 0;
    unsafe { self.GetIntegerv(GL_MAX_TEXTURE_IMAGE_UNITS, &mut out) };
    out
  }
  /// The total number of texture units that can be used from all active
  /// programs.
  #[inline]
  pub fn get_max_combined_texture_image_units(&self) -> u32 {
    let mut out = 0;
    unsafe { self.GetIntegerv(GL_MAX_COMBINED_TEXTURE_IMAGE_UNITS, &mut out) };
    out.try_into().unwrap()
  }
  /// Sets the active texture unit value.
  ///
  /// This should be from 0 to one less than
  /// [`get_max_combined_texture_image_units`](Self::get_max_combined_texture_image_units).
  #[inline]
  pub fn set_active_texture_unit(&self, unit: u32) {
    unsafe { self.ActiveTexture(GL_TEXTURE0 + unit) };
  }
  #[inline]
  pub fn get_active_texture_unit(&self) -> u32 {
    let mut out = 0;
    unsafe { self.GetIntegerv(GL_ACTIVE_TEXTURE, &mut out) };
    u32::try_from(out).unwrap() - GL_TEXTURE0
  }
}

impl EzGl {
  /// Draws using the array buffer data.
  ///
  /// * `mode` is the geometric primitive to assemble.
  /// * `range` is the `low..high` index span of array data to draw.
  ///
  /// ## Panics
  /// * `assert!(range.start < range.end);`
  ///
  /// ## Safety
  /// * The attrib pointers must have been properly configured
  /// * The arguments to this function must not cause the buffer data to be read
  ///   out of bounds.
  #[inline]
  pub unsafe fn draw_arrays(&self, mode: DrawMode, range: Range<usize>) {
    assert!(range.start < range.end);
    let first = range.start;
    let count = range.end - range.start;
    self.DrawArrays(
      mode as GLenum,
      first.try_into().unwrap(),
      count.try_into().unwrap(),
    )
  }

  /// Draws using the element buffer.
  ///
  /// * `T` is the type of data in the elements buffer.
  /// * `mode` is the geometric primitive to assemble.
  /// * `range` is the `low..high` index span of elements data to draw.
  ///
  /// ## Panics
  /// * `assert!(range.start < range.end);`
  ///
  /// ## Safety
  /// * The vertex attrib pointers must have been properly configured to match
  ///   the vertex buffer data.
  /// * The arguments to this function must not cause the vertex or element
  ///   buffers to be read out of bounds, or you must have configured the
  ///   context for robust buffer access.
  #[inline]
  pub unsafe fn draw_elements<T: DrawElementsType>(
    &self, mode: DrawMode, range: Range<usize>,
  ) {
    assert!(range.start < range.end);
    let base = range.start;
    let count = range.end - range.start;
    self.DrawElements(
      mode as GLenum,
      count.try_into().unwrap(),
      T::ENUM,
      base as *const c_void,
    )
  }
}

/// Types that can be used with a "draw elements" call.
///
/// ## Safety
/// * You cannot implement this trait.
pub unsafe trait DrawElementsType {
  const ENUM: GLenum;
}
unsafe impl DrawElementsType for u8 {
  const ENUM: GLenum = GL_UNSIGNED_BYTE;
}
unsafe impl DrawElementsType for u16 {
  const ENUM: GLenum = GL_UNSIGNED_SHORT;
}
unsafe impl DrawElementsType for u32 {
  const ENUM: GLenum = GL_UNSIGNED_INT;
}

impl EzGl {
  /// Declares attribute info for attributes that will be float vecs within the
  /// shader.
  ///
  /// The `BufferTy` generic should be the array type of the data *in the
  /// buffer* for this attribute. The data within the shader will be an equal
  /// length vector of floats. The GPU will transform the data on load as
  /// necessary.
  ///
  /// * `index`: The attribute pointer index to change
  /// * `stride`: The size of an entire vertex (all attributes combined).
  /// * `offset`: The offset of this attribute within the vertex.
  #[inline]
  pub fn vertex_attrib_f32_pointer<BufferTy: VertexAttribPointerTy>(
    &self, index: GLuint, stride: usize, offset: usize,
  ) {
    unsafe {
      self.VertexAttribPointer(
        index,
        BufferTy::SIZE,
        BufferTy::TY,
        BufferTy::NORMALIZED,
        stride.try_into().unwrap(),
        offset as *const c_void,
      )
    }
  }
}

/// ## Safety
/// * You are not allowed to implement this trait.
pub unsafe trait VertexAttribPointerTy {
  const SIZE: GLint;
  const TY: GLenum;
  const NORMALIZED: GLboolean;
}
unsafe impl VertexAttribPointerTy for [f32; 2] {
  const SIZE: GLint = 2;
  const NORMALIZED: GLboolean = GLboolean::FALSE;
  const TY: GLenum = GL_FLOAT;
}
unsafe impl VertexAttribPointerTy for [f32; 3] {
  const SIZE: GLint = 3;
  const NORMALIZED: GLboolean = GLboolean::FALSE;
  const TY: GLenum = GL_FLOAT;
}

#[derive(Debug)]
#[repr(transparent)]
pub struct VertexArrayObject(NonZeroU32);

#[derive(Debug)]
#[repr(transparent)]
pub struct BufferObject(NonZeroU32);

#[derive(Debug)]
#[repr(transparent)]
pub struct ShaderObject(NonZeroU32);

#[derive(Debug)]
#[repr(transparent)]
pub struct ProgramObject(NonZeroU32);

#[derive(Debug)]
#[repr(transparent)]
pub struct TextureObject(NonZeroU32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ShaderLocation(GLint);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BufferTarget {
  ArrayBuffer = GL_ARRAY_BUFFER,
  AtomicCounterBuffer = GL_ATOMIC_COUNTER_BUFFER,
  CopyReadBuffer = GL_COPY_READ_BUFFER,
  CopyWriteBuffer = GL_COPY_WRITE_BUFFER,
  DispatchIndirectBuffer = GL_DISPATCH_INDIRECT_BUFFER,
  DrawIndirectBuffer = GL_DRAW_INDIRECT_BUFFER,
  ElementArrayBuffer = GL_ELEMENT_ARRAY_BUFFER,
  PixelPackBuffer = GL_PIXEL_PACK_BUFFER,
  PixelUnpackBuffer = GL_PIXEL_UNPACK_BUFFER,
  QueryBuffer = GL_QUERY_BUFFER,
  ShaderStorageBuffer = GL_SHADER_STORAGE_BUFFER,
  TextureBuffer = GL_TEXTURE_BUFFER,
  TransformFeedbackBuffer = GL_TRANSFORM_FEEDBACK_BUFFER,
  UniformBuffer = GL_UNIFORM_BUFFER,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureTarget {
  Texture1D = GL_TEXTURE_1D,
  Texture1DArray = GL_TEXTURE_1D_ARRAY,
  Texture2D = GL_TEXTURE_2D,
  Texture2DArray = GL_TEXTURE_2D_ARRAY,
  Texture2DMultisample = GL_TEXTURE_2D_MULTISAMPLE,
  Texture2DMultisampleArray = GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
  Texture3D = GL_TEXTURE_3D,
  TextureCubeMap = GL_TEXTURE_CUBE_MAP,
  TextureCubeMapArray = GL_TEXTURE_CUBE_MAP_ARRAY,
  TextureRectangle = GL_TEXTURE_RECTANGLE,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureWrap {
  ClampToEdge = GL_CLAMP_TO_EDGE,
  ClampToBorder = GL_CLAMP_TO_BORDER,
  MirroredRepeat = GL_MIRRORED_REPEAT,
  Repeat = GL_REPEAT,
  MirrorClampToEdge = GL_MIRROR_CLAMP_TO_EDGE,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DrawMode {
  Points = GL_POINTS,
  LineStrip = GL_LINE_STRIP,
  LineLoop = GL_LINE_LOOP,
  Lines = GL_LINES,
  LineStripAdjacency = GL_LINE_STRIP_ADJACENCY,
  LinesAdjacency = GL_LINES_ADJACENCY,
  TriangleStrip = GL_TRIANGLE_STRIP,
  TriangleFan = GL_TRIANGLE_FAN,
  Triangles = GL_TRIANGLES,
  TriangleStripAdjacency = GL_TRIANGLE_STRIP_ADJACENCY,
  TrianglesAdjacency = GL_TRIANGLES_ADJACENCY,
  Patches = GL_PATCHES,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MinFilter {
  Nearest = GL_NEAREST,
  Linear = GL_LINEAR,
  NearestMipmapNearest = GL_NEAREST_MIPMAP_NEAREST,
  LinearMipmapNearest = GL_LINEAR_MIPMAP_NEAREST,
  NearestMipmapLinear = GL_NEAREST_MIPMAP_LINEAR,
  LinearMipmapLinear = GL_LINEAR_MIPMAP_LINEAR,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MagFilter {
  Nearest = GL_NEAREST,
  Linear = GL_LINEAR,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BufferUsageHint {
  StreamDraw = GL_STREAM_DRAW,
  StreamRead = GL_STREAM_READ,
  StreamCopy = GL_STREAM_COPY,
  StaticDraw = GL_STATIC_DRAW,
  StaticRead = GL_STATIC_READ,
  StaticCopy = GL_STATIC_COPY,
  DynamicDraw = GL_DYNAMIC_DRAW,
  DynamicRead = GL_DYNAMIC_READ,
  DynamicCopy = GL_DYNAMIC_COPY,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ShaderType {
  ComputeShader = GL_COMPUTE_SHADER,
  VertexShader = GL_VERTEX_SHADER,
  TessControlShader = GL_TESS_CONTROL_SHADER,
  TessEvaluationShader = GL_TESS_EVALUATION_SHADER,
  GeometryShader = GL_GEOMETRY_SHADER,
  FragmentShader = GL_FRAGMENT_SHADER,
}
