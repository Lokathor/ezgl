use beryllium::{
  events::Event,
  init::InitFlags,
  video::{CreateWinArgs, GlContextFlags, GlProfile, GlSwapInterval},
  Sdl,
};
use bytemuck::cast_slice;
use core::mem::size_of;
use ezgl::{
  BlendEquationSeparate, BlendFuncSeparate, BufferTarget::*,
  BufferUsageHint::*, DrawMode, EzGl, MagFilter, MinFilter, TextureTarget::*,
  TextureWrap,
};
use imagine::{image::Bitmap, pixel_formats::RGBA8888};
use pixel_formats::{r32g32b32a32_Sfloat, r8g8b8a8_Srgb};

macro_rules! check {
  ($gl:ident.$method:ident) => {
    eprintln!(concat!(stringify!($method), ": {}"), $gl.$method());
  };
}

const USE_GLES: bool =
  cfg!(target_arch = "aarch64") || cfg!(target_arch = "arm");

const GL_SHADER_HEADER: &str = "#version 410
";

const GLES_SHADER_HEADER: &str = "#version 310 es
precision mediump float;
";

const VERTEX_SRC: &str = "
  layout (location = 0) in vec3 aPos;
  layout (location = 1) in vec2 aTexCoord;
  
  out vec2 TexCoord;
  
  void main()
  {
    gl_Position = vec4(aPos, 1.0);
    TexCoord = aTexCoord;
  }";

const FRAGMENT_SRC: &str = "
  out vec4 FragColor;

  in vec2 TexCoord;
  
  uniform sampler2D ourTexture;
  
  void main()
  {
    FragColor = texture(ourTexture, TexCoord);
  }";

const GLIDER_BYTES: &[u8] = include_bytes!("../assets/glider-big-rainbow.png");

fn main() {
  let mut glider: Bitmap<RGBA8888> =
    Bitmap::try_from_png_bytes(GLIDER_BYTES).unwrap();
  glider.vertical_flip();

  // Initializes SDL2
  let sdl = Sdl::init(InitFlags::VIDEO);
  if USE_GLES {
    // In GLES mode we'll ask for 3.1, which is what Raspberry Pi 4 can do.
    sdl.set_gl_profile(GlProfile::ES).unwrap();
    sdl.set_gl_context_major_version(3).unwrap();
    sdl.set_gl_context_minor_version(1).unwrap();
  } else {
    // For plain GL we will use GL-4.1, which is the best that Mac can offer.
    sdl.set_gl_profile(GlProfile::Core).unwrap();
    sdl.set_gl_context_major_version(4).unwrap();
    sdl.set_gl_context_minor_version(1).unwrap();
  }
  // optimistically assume that we can use multisampling.
  sdl.set_gl_multisample_buffers(1).unwrap();
  sdl.set_gl_multisample_count(if USE_GLES { 4 } else { 8 }).unwrap();
  sdl.set_gl_framebuffer_srgb_capable(true).unwrap();
  let mut flags = GlContextFlags::default();
  if cfg!(target_os = "macos") {
    flags |= GlContextFlags::FORWARD_COMPATIBLE;
  }
  if cfg!(debug_assertions) {
    flags |= GlContextFlags::DEBUG;
  }
  sdl.set_gl_context_flags(flags).unwrap();

  // Makes the window with a GL Context.
  let win = sdl
    .create_gl_window(CreateWinArgs {
      title: "Example GL Window",
      width: glider.width.try_into().unwrap(),
      height: glider.height.try_into().unwrap(),
      ..Default::default()
    })
    .unwrap();
  win.set_swap_interval(GlSwapInterval::AdaptiveVsync).ok();
  let gl = {
    let mut temp = EzGl::new_boxed();
    unsafe { temp.load(|name| win.get_proc_address(name)) }
    temp
  };
  if cfg!(debug_assertions) {
    if win.supports_extension("GL_KHR_debug") {
      if gl.set_stderr_debug_message_callback().is_ok() {
        eprintln!("Set the stderr GL debug callback.");
      } else {
        eprintln!("`GL_KHR_debug` should be supported, but couldn't enable the debug callback.");
      }
    } else {
      eprintln!("Running in debug mode but `GL_KHR_debug` is not available.")
    }
    check!(gl.get_max_combined_texture_image_units);
    check!(gl.get_active_texture_unit);
  }

  if !USE_GLES {
    gl.enable_multisample(true);
    gl.enable_framebuffer_srgb(true);
  }
  gl.set_pixel_store_unpack_alignment(1);
  gl.set_clear_color(1.0, 0.0, 1.0, 1.0);
  gl.enable_depth_test(true);
  // https://www.realtimerendering.com/blog/gpus-prefer-premultiplication/
  gl.set_blend_equation_separate(
    BlendEquationSeparate::Add,
    BlendEquationSeparate::Add,
  );
  gl.set_blend_func_separate(
    BlendFuncSeparate::One,
    BlendFuncSeparate::OneMinusSrcAlpha,
    BlendFuncSeparate::One,
    BlendFuncSeparate::OneMinusSrcAlpha,
  );

  let vao = gl.gen_vertex_array().unwrap();
  gl.bind_vertex_array(&vao);

  type Vertex = [f32; 5];
  #[rustfmt::skip]
  let vertices: &[Vertex] = &[
    // positions      // texture coords
    [1.0, 1.0, 0.0,   1.0, 1.0], // top right
    [1.0, -1.0, 0.0,  1.0, 0.0], // bottom right
    [-1.0, -1.0, 0.0, 0.0, 0.0], // bottom left
    [-1.0, 1.0, 0.0,  0.0, 1.0], // top left
  ];
  type TriElement = [u32; 3];
  let indices: &[TriElement] = &[[0, 1, 3], [1, 2, 3]];

  let vbo = gl.gen_buffer().unwrap();
  gl.bind_buffer(ArrayBuffer, &vbo);
  gl.buffer_data(ArrayBuffer, cast_slice(vertices), StaticDraw);

  let ebo = gl.gen_buffer().unwrap();
  gl.bind_buffer(ElementArrayBuffer, &ebo);
  gl.buffer_data(ElementArrayBuffer, cast_slice(indices), StaticDraw);

  gl.enable_vertex_attrib_array(0);
  gl.vertex_attrib_f32_pointer::<[f32; 3]>(
    0,
    size_of::<Vertex>(),
    size_of::<[f32; 0]>(),
  );
  gl.enable_vertex_attrib_array(1);
  gl.vertex_attrib_f32_pointer::<[f32; 2]>(
    1,
    size_of::<Vertex>(),
    size_of::<[f32; 3]>(),
  );

  let shader_header =
    if USE_GLES { GLES_SHADER_HEADER } else { GL_SHADER_HEADER };
  let vertex_src = format!("{shader_header}\n{VERTEX_SRC}");
  let fragment_src = format!("{shader_header}\n{FRAGMENT_SRC}");
  let program =
    gl.create_vertex_fragment_program(&vertex_src, &fragment_src).unwrap();
  gl.use_program(&program);

  let yellow = r32g32b32a32_Sfloat { r: 1.0, g: 1.0, b: 0.0, a: 1.0 };
  let texture = gl.gen_texture().unwrap();
  gl.bind_texture(Texture2D, &texture);
  gl.set_texture_wrap_s(Texture2D, TextureWrap::MirroredRepeat);
  gl.set_texture_wrap_t(Texture2D, TextureWrap::MirroredRepeat);
  gl.set_texture_border_color(Texture2D, &yellow);
  gl.set_texture_min_filter(Texture2D, MinFilter::LinearMipmapLinear);
  gl.set_texture_mag_filter(Texture2D, MagFilter::Linear);
  gl.tex_image_2d(
    Texture2D,
    0,
    glider.width.try_into().unwrap(),
    glider.height.try_into().unwrap(),
    cast_slice::<_, r8g8b8a8_Srgb>(&glider.pixels),
  );
  gl.generate_mipmap(Texture2D);

  //let loc = gl.get_uniform_location(&program, "ourColor").unwrap();

  // program "main loop".
  'the_loop: loop {
    // Process events from this frame.
    #[allow(clippy::never_loop)]
    #[allow(clippy::single_match)]
    while let Some((event, _timestamp)) = sdl.poll_events() {
      match event {
        Event::Quit => break 'the_loop,
        _ => (),
      }
    }

    gl.clear_color_and_depth_buffer();
    //unsafe { gl.draw_arrays(DrawMode::Triangles, 0..3) };
    unsafe { gl.draw_elements::<u32>(DrawMode::Triangles, 0..6) };

    win.swap_window();
  }
}
