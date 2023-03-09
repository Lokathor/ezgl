use core::mem::size_of;

use beryllium::{
  events::Event,
  init::InitFlags,
  video::{CreateWinArgs, GlContextFlags, GlProfile, GlSwapInterval},
  Sdl,
};
use bytemuck::cast_slice;
use ezgl::{
  BufferTarget::*, BufferUsageHint::*, EzGl, MagFilter, MinFilter,
  ShaderType::*, TextureTarget::*, TextureWrap,
};
use gl_constants::*;
use imagine::{image::Bitmap, pixel_formats::RGBA8888};

const VERTEX_SRC: &str = "#version 310 es
  precision mediump float;
  layout (location = 0) in vec3 aPos;
  layout (location = 1) in vec3 aColor;
  layout (location = 2) in vec2 aTexCoord;
  
  out vec3 ourColor;
  out vec2 TexCoord;
  
  void main()
  {
      gl_Position = vec4(aPos, 1.0);
      ourColor = aColor;
      TexCoord = aTexCoord;
  }";

const FRAGMENT_SRC: &str = "#version 310 es
  precision mediump float;
  out vec4 FragColor;
    
  in vec3 ourColor;
  in vec2 TexCoord;
  
  uniform sampler2D ourTexture;
  
  void main()
  {
      FragColor = texture(ourTexture, TexCoord) * vec4(ourColor, 1.0);
  }";

const GLIDER_BYTES: &[u8] = include_bytes!("../assets/glider-big-rainbow.png");

pub fn get_ticks() -> u32 {
  unsafe { fermium::prelude::SDL_GetTicks() }
}

fn main() {
  let glider: Bitmap<RGBA8888> =
    Bitmap::try_from_png_bytes(GLIDER_BYTES).unwrap();

  // Initializes SDL2
  let sdl = Sdl::init(InitFlags::EVERYTHING);
  if cfg!(target_os = "macos") {
    // For Mac, just ask for the best core profile supported.
    sdl.set_gl_profile(GlProfile::Core).unwrap();
    sdl.set_gl_context_major_version(4).unwrap();
    sdl.set_gl_context_minor_version(1).unwrap();
  } else {
    // anywhere else we'll run as GLES-3.1, which a desktop's GL-4.5 can
    // provide, but which is also available on Raspberry Pi.
    sdl.set_gl_profile(GlProfile::ES).unwrap();
    sdl.set_gl_context_major_version(3).unwrap();
    sdl.set_gl_context_minor_version(1).unwrap();
  }
  // optimistically assume that we can use multisampling.
  sdl.set_gl_multisample_buffers(1).unwrap();
  sdl.set_gl_multisample_count(8).unwrap();
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
      width: 800,
      height: 800,
      ..Default::default()
    })
    .unwrap();
  win.set_swap_interval(GlSwapInterval::AdaptiveVsync).ok();
  let gl = {
    let mut temp = EzGl::new_boxed();
    unsafe { temp.load(|name| win.get_proc_address(name)) }
    temp
  };
  if cfg!(debug_assertions) && win.supports_extension("GL_KHR_debug") {
    if gl.set_stderr_debug_message_callback().is_ok() {
      println!("Set the stderr GL debug callback.");
    } else {
      println!("`GL_KHR_debug` should be supported, but couldn't enable the debug callback.");
    }
  } else {
    println!("Running in debug mode but `GL_KHR_debug` is not available.")
  }

  let mut controllers = Vec::new();

  gl.set_clear_color(0.2, 0.3, 0.3, 1.0);

  let vao = gl.gen_vertex_array().unwrap();
  gl.bind_vertex_array(&vao);

  type Vertex = [f32; 8];
  let vertices: &[Vertex] = &[
    // positions        // colors        // texture coords
    [0.5, 0.5, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0], // top right
    [0.5, -0.5, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0], // bottom right
    [-0.5, -0.5, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0], // bottom left
    [-0.5, 0.5, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0], // top left
  ];
  type TriElement = [u32; 3];
  let indices: &[TriElement] = &[[0, 1, 3], [1, 2, 3]];

  let vbo = gl.gen_buffer().unwrap();
  gl.bind_buffer(ArrayBuffer, &vbo);
  gl.alloc_buffer_data(ArrayBuffer, cast_slice(vertices), StaticDraw);

  let ebo = gl.gen_buffer().unwrap();
  gl.bind_buffer(ElementArrayBuffer, &ebo);
  gl.alloc_buffer_data(ElementArrayBuffer, cast_slice(indices), StaticDraw);

  gl.enable_vertex_attrib_array(0);
  gl.vertex_attrib_f32_pointer::<[f32; 3]>(
    0,
    size_of::<Vertex>(),
    size_of::<[f32; 0]>(),
  );
  gl.enable_vertex_attrib_array(1);
  gl.vertex_attrib_f32_pointer::<[f32; 3]>(
    1,
    size_of::<Vertex>(),
    size_of::<[f32; 3]>(),
  );
  gl.enable_vertex_attrib_array(2);
  gl.vertex_attrib_f32_pointer::<[f32; 2]>(
    2,
    size_of::<Vertex>(),
    size_of::<[f32; 6]>(),
  );

  let vertex_shader = gl.create_shader(VertexShader).unwrap();
  gl.set_shader_source(&vertex_shader, VERTEX_SRC);
  gl.compile_shader(&vertex_shader);
  if !gl.get_shader_compile_success(&vertex_shader) {
    let log = gl.get_shader_info_log(&vertex_shader);
    panic!("Vertex Shader Error: {log}");
  }

  let fragment_shader = gl.create_shader(FragmentShader).unwrap();
  gl.set_shader_source(&fragment_shader, FRAGMENT_SRC);
  gl.compile_shader(&fragment_shader);
  if !gl.get_shader_compile_success(&fragment_shader) {
    let log = gl.get_shader_info_log(&fragment_shader);
    panic!("Vertex Shader Error: {log}");
  }

  let program = gl.create_program().unwrap();
  gl.attach_shader(&program, &vertex_shader);
  gl.attach_shader(&program, &fragment_shader);
  gl.link_program(&program);
  if !gl.get_program_link_success(&program) {
    let log = gl.get_program_info_log(&program);
    panic!("Program Link Error: {log}");
  }
  gl.use_program(&program);

  let texture = gl.gen_texture().unwrap();
  gl.bind_texture(Texture2D, &texture);
  gl.set_texture_wrap_s(Texture2D, TextureWrap::MirroredRepeat);
  gl.set_texture_wrap_t(Texture2D, TextureWrap::MirroredRepeat);
  gl.set_texture_border_color(Texture2D, &[1.0, 1.0, 0.0, 1.0]);
  gl.set_texture_min_filter(Texture2D, MinFilter::LinearMipmapLinear);
  gl.set_texture_mag_filter(Texture2D, MagFilter::Linear);
  gl.alloc_tex_image_2d(Texture2D, 0, &glider);
  gl.generate_mipmap(Texture2D);

  //let loc = gl.get_uniform_location(&program, "ourColor").unwrap();

  // program "main loop".
  'the_loop: loop {
    // Process events from this frame.
    #[allow(clippy::never_loop)]
    while let Some((event, _timestamp)) = sdl.poll_events() {
      match event {
        Event::Quit => break 'the_loop,
        Event::ControllerAdded { index } => {
          match sdl.open_game_controller(index) {
            Ok(controller) => {
              println!(
                "Opened `{name}` (type: {ty:?})",
                name = controller.get_name(),
                ty = controller.get_type()
              );
              controllers.push(controller);
            }
            Err(msg) => println!("Couldn't open {index}: {msg:?}"),
          }
        }
        Event::ControllerButton { ctrl_id, button, pressed } => {
          println!("{ctrl_id}: {button:?}={pressed}");
        }
        _ => (),
      }
    }

    gl.clear(GL_COLOR_BUFFER_BIT);
    //unsafe { gl.draw_arrays(GL_TRIANGLES, 0, 3) };
    unsafe { gl.draw_elements(GL_TRIANGLES, 6, GL_UNSIGNED_INT, 0) };

    win.swap_window();
  }
}
