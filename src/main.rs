use chrono::{Datelike, Local, Timelike};
use glutin::dpi::PhysicalSize;
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;
use json::JsonValue;
use rand::Rng;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::prelude::*;

// Struct for a cycle of a palette
struct Cycle {
    reverse: u32,
    rate: u32,
    low: u32,
    high: u32,
}

fn main() -> std::io::Result<()> {
    // Json processing
    let mut rng = rand::thread_rng();
    let rand = rng.gen::<u32>();
    let month = Local::now().month();
    let mut file = match month {
        1 => match rand % 2 {
            0 => File::open("scenes/January-Clear.json").unwrap(),
            1 => File::open("scenes/January-Snow.json").unwrap(),
            _ => panic!("Lol aint no way"),
        },
        2 => match rand % 2 {
            0 => File::open("scenes/February-Clear.json").unwrap(),
            1 => File::open("scenes/February-Cloudy.json").unwrap(),
            _ => panic!("Lol aint no way"),
        },
        3 => match rand % 1 {
            0 => File::open("scenes/March-Clear.json").unwrap(),
            _ => panic!("Lol aint no way"),
        },
        4 => match rand % 2 {
            0 => File::open("scenes/April-Clear.json").unwrap(),
            1 => File::open("scenes/April-Rain.json").unwrap(),
            _ => panic!("Lol aint no way"),
        },
        5 => match rand % 3 {
            0 => File::open("scenes/May-Clear.json").unwrap(),
            1 => File::open("scenes/May-Cloudy.json").unwrap(),
            2 => File::open("scenes/May-Rain.json").unwrap(),
            _ => panic!("Lol aint no way"),
        },
        6 => File::open("scenes/June-Clear.json").unwrap(),
        7 => match rand % 2 {
            0 => File::open("scenes/July-Clear.json").unwrap(),
            1 => File::open("scenes/July-Cloudy.json").unwrap(),
            _ => panic!("Lol aint no way"),
        },
        8 => File::open("scenes/August-Clear.json").unwrap(),
        9 => match rand % 2 {
            0 => File::open("scenes/September-Clear.json").unwrap(),
            1 => File::open("scenes/September-Cloudy.json").unwrap(),
            _ => panic!("Lol aint no way"),
        },
        10 => match rand % 3 {
            0 => File::open("scenes/EarlyOctober-Clear.json").unwrap(),
            1 => File::open("scenes/LateOctober-Clear.json").unwrap(),
            2 => File::open("scenes/LateOctober-Rain.json").unwrap(),
            _ => panic!("Lol aint no way"),
        },
        11 => match rand % 2 {
            0 => File::open("scenes/November-Clear.json").unwrap(),
            1 => File::open("scenes/November-Rain.json").unwrap(),
            _ => panic!("Lol aint no way"),
        },
        12 => File::open("scenes/December-Clear.json").unwrap(),
        _ => panic!("Lol aint no way"),
    };
    let mut contents: String = String::new();
    _ = file.read_to_string(&mut contents);

    let parsed: JsonValue = json::parse(&contents).unwrap();

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

    let palettes_raw: JsonValue = parsed["palettes"].clone();
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
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(width, height))
        .with_title("living-worlds")
        .with_decorations(false);
    let context_builder = ContextBuilder::new().with_vsync(true);
    let gl_window = context_builder
        .build_windowed(window_builder, &event_loop)
        .unwrap();
    let gl_window = unsafe { gl_window.make_current().unwrap() };

    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    let vertex_shader_src = r#"
            #version 330 core
    
            layout (location = 0) in vec2 position;
            layout (location = 1) in vec2 texCoord;

            out vec2 uv;
    
            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
                uv = texCoord;
            }
        "#;

    let fragment_shader_src = r#"
            #version 330 core
    
            uniform sampler1D palette;
            uniform sampler2D color_indices;
    
            in vec2 uv;

            out vec4 fragColor;
    
            void main() {    
                vec2 flippedUV = vec2(uv.x, 1.0 - uv.y);
                float index = texture(color_indices, flippedUV).r;
                vec3 color = texture(palette, index).rgb;
    
                fragColor = vec4(color, 1.0);
            }
        "#;

    let vertex_shader = compile_shader(vertex_shader_src, gl::VERTEX_SHADER);
    let fragment_shader = compile_shader(fragment_shader_src, gl::FRAGMENT_SHADER);

    let shader_program = unsafe { gl::CreateProgram() };
    unsafe {
        // Attach the vertex and fragment shaders to the program
        gl::AttachShader(shader_program, vertex_shader);
        gl::AttachShader(shader_program, fragment_shader);

        // Link the shader program
        gl::LinkProgram(shader_program);

        // Check if the shader program was linked successfully
        let mut success = 0;
        gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);

        if success == gl::FALSE as i32 {
            let mut len = 0;
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

    let mut vao = 0;
    let mut vbo = 0;

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

    event_loop.run(move |event, _, control_flow| {
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

        let now = Local::now();
        let seconds = now.num_seconds_from_midnight();
        let milliseconds = std::u32::MAX - 100 * now.timestamp_millis() as u32;
        let time = get_palette_time(&timeline, seconds);
        let palette_id = &timeline[&time];
        let mut palette = palettes[palette_id].clone();
        let cycles = &cycles[palette_id];

        for cycle in cycles {
            if cycle.rate != 0 {
                let cycle_size = (cycle.high - cycle.low) + 1;
                let cycle_rate = cycle.rate / cycle_speed;
                let mut cycle_amount = 0;

                if cycle.reverse < 3 {
                    cycle_amount = (milliseconds / 1000 / cycle_rate) % cycle_size
                } else if cycle.reverse == 3 {
                    cycle_amount = (milliseconds / 1000 / cycle_rate) % (cycle_size * 2);
                    if cycle_amount >= cycle_size {
                        cycle_amount = cycle_size * 2 - cycle_amount;
                    }
                } else if cycle.reverse < 6 {
                    cycle_amount = (milliseconds / 1000 / cycle_rate) % cycle_size;
                    cycle_amount = (((cycle_amount as f32 * std::f32::consts::PI * 2.0)
                        / cycle_size as f32)
                        .sin()
                        + 1.0) as u32;
                    if cycle.reverse == 4 {
                        cycle_amount *= cycle_size / 4;
                    } else if cycle.reverse == 5 {
                        cycle_amount *= cycle_size / 2;
                    }
                }

                if cycle.reverse == 2 {
                    for i in 0..cycle_amount {
                        let temp = palette[cycle.low as usize + i as usize].clone();
                        palette[cycle.low as usize + i as usize] =
                            palette[cycle.high as usize - i as usize].clone();
                        palette[cycle.high as usize - i as usize] = temp;
                    }
                }
                for _ in 0..cycle_amount {
                    let temp = palette[cycle.low as usize].clone();
                    for j in cycle.low..=cycle.high - 1 {
                        let j = j as usize;
                        palette[j] = palette[j + 1].clone();
                    }
                    palette[cycle.high as usize] = temp;
                }
                if cycle.reverse == 2 {
                    for i in 0..cycle_amount {
                        let temp = palette[cycle.low as usize + i as usize].clone();
                        palette[cycle.low as usize + i as usize] =
                            palette[cycle.high as usize - i as usize].clone();
                        palette[cycle.high as usize - i as usize] = temp;
                    }
                }
            }
        }

        unsafe {
            // Activate the shader program
            gl::UseProgram(shader_program);

            // Set uniforms
            let palette_loc =
                gl::GetUniformLocation(shader_program, "palette\0".as_ptr() as *const _);
            let color_indices_loc =
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

        gl_window.swap_buffers().unwrap();
    });
}

fn create_whitespace_cstring_with_len(len: usize) -> std::ffi::CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { std::ffi::CString::from_vec_unchecked(buffer) }
}

fn compile_shader(source: &str, shader_type: gl::types::GLenum) -> gl::types::GLuint {
    let shader = unsafe { gl::CreateShader(shader_type) };

    unsafe {
        gl::ShaderSource(
            shader,
            1,
            &(source.as_ptr() as *const _),
            &(source.len() as i32),
        );
        gl::CompileShader(shader);
    }

    let mut success = 0;
    unsafe { gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success) };

    if success == gl::FALSE as i32 {
        let mut len = 0;
        unsafe { gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len) };
        let error = create_whitespace_cstring_with_len(len as usize);
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
