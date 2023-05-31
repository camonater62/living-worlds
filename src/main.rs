use chrono::{Datelike, Local, Timelike};
use glutin::dpi::PhysicalSize;
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;
use json::JsonValue;
use std::collections::{BTreeMap, HashMap};

// Struct for a cycle of a palette
struct Cycle {
    reverse: u32,
    rate: u32,
    low: u32,
    high: u32,
}

fn main() -> std::io::Result<()> {
    // Json processing
    let month = Local::now().month();
    let file = match month {
        1 => include_str!("../scenes/January-Clear.json"),
        2 => include_str!("../scenes/February-Clear.json"),
        3 => include_str!("../scenes/March-Clear.json"),
        4 => include_str!("../scenes/April-Clear.json"),
        5 => include_str!("../scenes/May-Clear.json"),
        6 => include_str!("../scenes/June-Clear.json"),
        7 => include_str!("../scenes/July-Clear.json"),
        8 => include_str!("../scenes/August-Clear.json"),
        9 => include_str!("../scenes/September-Clear.json"),
        10 => include_str!("../scenes/EarlyOctober-Clear.json"),
        11 => include_str!("../scenes/November-Clear.json"),
        12 => include_str!("../scenes/December-Clear.json"),
        _ => panic!("Invalid month"),
    };

    let parsed: JsonValue = json::parse(&file).unwrap();

    let base: HashMap<&str, &JsonValue> = parsed["base"].entries().collect();

    let width: u32 = base["width"].as_u32().unwrap();
    let height: u32 = base["height"].as_u32().unwrap();
    let color_indices: Vec<u8> = base["pixels"]
        .members()
        .map(|x: &JsonValue| x.as_u8().unwrap())
        .collect();

    let timeline: BTreeMap<u32, String> = parsed["timeline"]
        .entries()
        .map(|(time, palette)| {
            (
                time.parse::<u32>().unwrap(),
                palette.as_str().unwrap().to_string(),
            )
        })
        .collect();

    let palettes_raw = &parsed["palettes"];
    let mut palettes: HashMap<String, [[u8; 3]; 256]> = HashMap::new();
    let mut cycles: HashMap<String, Vec<Cycle>> = HashMap::new();
    let cycle_speed = 280;
    for (name, value) in palettes_raw.entries() {
        let cycles_vec: Vec<Cycle> = value["cycles"]
            .members()
            .map(|value: &JsonValue| {
                let reverse: u32 = value["reverse"].as_u32().unwrap();
                let rate: u32 = value["rate"].as_u32().unwrap();
                let low: u32 = value["low"].as_u32().unwrap();
                let high: u32 = value["high"].as_u32().unwrap();
                Cycle {
                    reverse,
                    rate,
                    low,
                    high,
                }
            })
            .collect();
        cycles.insert(name.to_string(), cycles_vec);
        let colors = value["colors"]
            .members()
            .map(|x| {
                let r = x[0].as_u8().unwrap();
                let g = x[1].as_u8().unwrap();
                let b = x[2].as_u8().unwrap();
                [r, g, b]
            })
            .collect::<Vec<[u8; 3]>>();
        let mut palette = [[0; 3]; 256];
        for (i, color) in colors.iter().enumerate() {
            palette[i] = *color;
        }
        palettes.insert(name.to_string(), palette);
    }

    // GL
    let event_loop: EventLoop<()> = EventLoop::new();
    let window_builder: WindowBuilder = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(width, height))
        .with_title("living-worlds")
        .with_decorations(false);
    let context_builder = ContextBuilder::new().with_vsync(true);
    let gl_window = context_builder
        .build_windowed(window_builder, &event_loop)
        .unwrap();
    let gl_window = unsafe { gl_window.make_current().unwrap() };

    gl::load_with(|symbol: &str| gl_window.get_proc_address(symbol) as *const _);

    let vertex_shader: u32 = compile_shader(include_str!("vertex.glsl"), gl::VERTEX_SHADER);
    let fragment_shader: u32 = compile_shader(include_str!("fragment.glsl"), gl::FRAGMENT_SHADER);

    let shader_program: u32 = unsafe { gl::CreateProgram() };
    unsafe {
        // Attach the vertex and fragment shaders to the program
        gl::AttachShader(shader_program, vertex_shader);
        gl::AttachShader(shader_program, fragment_shader);

        // Link the shader program
        gl::LinkProgram(shader_program);

        // Check if the shader program was linked successfully
        let mut success: i32 = 0;
        gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);

        if success == gl::FALSE as i32 {
            let mut len: i32 = 0;
            gl::GetProgramiv(shader_program, gl::INFO_LOG_LENGTH, &mut len);
            let error = create_whitespace_cstring_with_len(len as usize);
            gl::GetProgramInfoLog(
                shader_program,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut _,
            );
            panic!(
                "Failed to link shader program: {}",
                error.to_string_lossy().into_owned()
            );
        }

        // Delete the vertex and fragment shaders once linked
        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);
    }

    let mut color_indices_texture = 0;
    let mut palette_texture = 0;
    unsafe {
        gl::Viewport(0, 0, width as i32, height as i32);
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);

        // Create the color index texture
        gl::GenTextures(1, &mut color_indices_texture);
        gl::BindTexture(gl::TEXTURE_2D, color_indices_texture);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::R8 as i32,
            width as i32,
            height as i32,
            0,
            gl::RED,
            gl::UNSIGNED_BYTE,
            color_indices.as_ptr() as *const _,
        );

        // Set texture parameters
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

        // Create the palette texture
        gl::GenTextures(1, &mut palette_texture);
        gl::BindTexture(gl::TEXTURE_1D, palette_texture);

        // Set texture parameters
        gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
    }

    let vertices: [f32; 24] = [
        // Positions // Texture Coords
        -1.0, -1.0, 0.0, 0.0, // Bottom-left
        1.0, -1.0, 1.0, 0.0, // Bottom-right
        1.0, 1.0, 1.0, 1.0, // Top-right
        1.0, 1.0, 1.0, 1.0, // Top-right
        -1.0, 1.0, 0.0, 1.0, // Top-left
        -1.0, -1.0, 0.0, 0.0, // Bottom-left
    ];

    let mut vao: u32 = 0;
    let mut vbo: u32 = 0;

    unsafe {
        gl::GenBuffers(1, &mut vbo);
        gl::GenVertexArrays(1, &mut vao);

        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
            vertices.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        // Position attribute
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            2,
            gl::FLOAT,
            gl::FALSE as gl::types::GLboolean,
            4 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            std::ptr::null(),
        );
        // Texture coord attribute
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE as gl::types::GLboolean,
            4 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            (2 * std::mem::size_of::<f32>()) as *const _,
        );
    }

    event_loop.run(move |event: Event<()>, _, control_flow: &mut ControlFlow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => unsafe {
                    gl::Viewport(0, 0, size.width as i32, size.height as i32);
                },
                _ => (),
            },

            _ => (),
        }

        let now: chrono::DateTime<Local> = Local::now();
        let seconds: u32 = now.num_seconds_from_midnight();
        let milliseconds: u32 = std::u32::MAX - (150 * now.timestamp_millis()) as u32;
        let time: u32 = get_palette_time(&timeline, seconds);
        let palette_id: &String = &timeline[&time];
        let mut palette: [[u8; 3]; 256] = palettes[palette_id];
        let cycles: &Vec<Cycle> = &cycles[palette_id];

        for cycle in cycles {
            if cycle.rate > 0 {
                let cycle_size: u32 = (cycle.high - cycle.low) + 1;
                let cycle_rate: u32 = cycle.rate / cycle_speed;
                let mut cycle_amount: u32 = 0;

                if cycle.reverse < 3 {
                    cycle_amount = (milliseconds / 1000 / cycle_rate) % cycle_size
                } else if cycle.reverse == 3 {
                    cycle_amount = (milliseconds / 1000 / cycle_rate) % (cycle_size * 2);
                    if cycle_amount >= cycle_size {
                        cycle_amount = cycle_size * 2 - cycle_amount;
                    }
                } else if cycle.reverse < 6 {
                    cycle_amount = (milliseconds / 1000 / cycle_rate) % cycle_size;
                    let cycle_amount_float: f32 =
                        ((cycle_amount as f32 * std::f32::consts::PI * 2.0) / cycle_size as f32)
                            .sin()
                            + 1.0;
                    if cycle.reverse == 4 {
                        cycle_amount = (cycle_amount_float * cycle_size as f32 / 4.0) as u32;
                    } else if cycle.reverse == 5 {
                        cycle_amount = (cycle_amount_float * cycle_size as f32 / 2.0) as u32;
                    } else {
                        cycle_amount = cycle_amount_float as u32;
                    }
                }

                if cycle.reverse == 2 {
                    for i in 0..cycle_amount {
                        let temp: [u8; 3] = palette[cycle.low as usize + i as usize];
                        palette[cycle.low as usize + i as usize] =
                            palette[cycle.high as usize - i as usize];
                        palette[cycle.high as usize - i as usize] = temp;
                    }
                }
                for _ in 0..cycle_amount {
                    let temp: [u8; 3] = palette[cycle.low as usize];
                    for j in cycle.low..=cycle.high - 1 {
                        let j: usize = j as usize;
                        palette[j] = palette[j + 1];
                    }
                    palette[cycle.high as usize] = temp;
                }
                if cycle.reverse == 2 {
                    for i in 0..cycle_amount {
                        let temp: [u8; 3] = palette[cycle.low as usize + i as usize];
                        palette[cycle.low as usize + i as usize] =
                            palette[cycle.high as usize - i as usize];
                        palette[cycle.high as usize - i as usize] = temp;
                    }
                }
            }
        }

        unsafe {
            // Activate the shader program
            gl::UseProgram(shader_program);

            // Set uniforms
            let palette_loc: i32 =
                gl::GetUniformLocation(shader_program, "palette\0".as_ptr() as *const _);
            let color_indices_loc: i32 =
                gl::GetUniformLocation(shader_program, "color_indices\0".as_ptr() as *const _);

            gl::Uniform1i(palette_loc, 0);
            gl::Uniform1i(color_indices_loc, 1);

            // Bind textures
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_1D, palette_texture);
            gl::TexImage1D(
                gl::TEXTURE_1D,
                0,
                gl::RGB8 as i32,
                256,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                palette.as_ptr() as *const _,
            );

            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, color_indices_texture);

            // Render the quad
            gl::BindVertexArray(vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }

        std::thread::sleep(std::time::Duration::from_millis(1000 / 15));

        gl_window.swap_buffers().unwrap();
    });
}

fn create_whitespace_cstring_with_len(len: usize) -> std::ffi::CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { std::ffi::CString::from_vec_unchecked(buffer) }
}

fn compile_shader(source: &str, shader_type: gl::types::GLenum) -> gl::types::GLuint {
    let shader: u32 = unsafe { gl::CreateShader(shader_type) };

    unsafe {
        gl::ShaderSource(
            shader,
            1,
            &(source.as_ptr() as *const _),
            &(source.len() as i32),
        );
        gl::CompileShader(shader);
    }

    let mut success: i32 = 0;
    unsafe { gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success) };

    if success == gl::FALSE as i32 {
        let mut len: i32 = 0;
        unsafe { gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len) };
        let error: std::ffi::CString = create_whitespace_cstring_with_len(len as usize);
        unsafe {
            gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), error.as_ptr() as *mut _)
        };
        panic!(
            "Failed to compile shader: {}",
            error.to_string_lossy().into_owned()
        );
    }

    shader
}

fn get_palette_time(timeline: &BTreeMap<u32, String>, seconds: u32) -> u32 {
    let mut last_time: &u32 = timeline.keys().last().unwrap();
    for (time, _) in timeline.iter() {
        if time > &seconds {
            return *last_time;
        }
        last_time = time;
    }
    *timeline.keys().last().unwrap()
}
