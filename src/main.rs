extern crate sdl2;
extern crate wgpu;

use sdl2::event::Event;
use anyhow::*;
use my_engine::wgpu_engine::{WgpuEngine, WindowSize};

#[tokio::main]
async fn main() -> Result<()> {
    // Show logs from wgpu
    env_logger::init();
    let map_str = |e: String| {anyhow!(e)};

    let sdl_context = sdl2::init().map_err(map_str)?;
    let video_subsystem = sdl_context.video().map_err(map_str)?;
    let window = video_subsystem
        .window("Raw Window Handle Example", 801, 600)
        .position_centered()
        .metal_view()
        .resizable()
        .build()?;

    let mut engine = WgpuEngine::new(&window).await?;

    let mut event_pump = sdl_context.event_pump().map_err(map_str)?;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { timestamp } => {
                    println!("Quit in {timestamp}");
                    break 'running;
                }
                Event::Window { win_event, .. } => match win_event {
                    sdl2::event::WindowEvent::Resized(width, height) => {
                        engine.resize(WindowSize{width: width as u32, height: height as u32});
                    },
                    _ => (),
                }
                _ => ()    
            }
            engine.input(&event);
        }
        //controller.update(&mut renderer.camera.camera_position);
        engine.update();
        engine.render()?;
    }

    Ok(())
}