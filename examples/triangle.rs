use beryllium::{
  events::Event,
  init::InitFlags,
  video::{CreateWinArgs, GlContextFlags, GlProfile},
  Sdl,
};
use ezgl::set_stderr_debug_message_callback;
use gl_struct_loader::GL;

fn main() {
  // Initializes SDL2
  let sdl = Sdl::init(InitFlags::EVERYTHING);
  if cfg!(target_arch = "macos") {
    // For Mac, just ask for the best core profile supported.
    sdl.set_gl_profile(GlProfile::Core).unwrap();
    sdl.set_gl_context_major_version(4).unwrap();
    sdl.set_gl_context_minor_version(1).unwrap();
  } else {
    // anywhere else we'll run as GLES-3.1, which desktops with GL-4.5 can
    // provide, and this lets the app be Raspberry Pi friendly.
    sdl.set_gl_profile(GlProfile::ES).unwrap();
    sdl.set_gl_context_major_version(3).unwrap();
    sdl.set_gl_context_minor_version(1).unwrap();
  }
  // optimistically assume that we can use multisampling.
  sdl.set_gl_multisample_buffers(1).unwrap();
  sdl.set_gl_multisample_count(4).unwrap();
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
      ..Default::default()
    })
    .unwrap();
  unsafe { GL.write().unwrap().load(|name| win.get_proc_address(name)) }
  if cfg!(debug_assertions) && win.supports_extension("GL_KHR_debug") {
    if set_stderr_debug_message_callback().is_ok() {
      println!("Set the stderr GL debug callback.");
    } else {
      println!("`GL_KHR_debug` should be supported, but couldn't enable the debug callback.");
    }
  }

  let mut controllers = Vec::new();

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
                "Opened `{name}` (type: {type_:?})",
                name = controller.get_name(),
                type_ = controller.get_type()
              );
              controllers.push(controller);
            }
            Err(msg) => println!("Couldn't open {index}: {msg:?}"),
          }
        }
        Event::JoystickAxis { .. }
        | Event::ControllerAxis { .. }
        | Event::MouseMotion { .. } => (),
        _ => (),
      }
    }

    // TODO: post-events drawing

    // TODO: swap buffers.
  }

  // All the cleanup is handled by the various drop impls.
}
