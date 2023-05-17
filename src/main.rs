use chrono::{Local, Timelike};
use json::JsonValue;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::prelude::*;

use glutin::dpi::PhysicalSize;
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;

fn get_palette_time(timeline: &BTreeMap<u32, &str>, seconds: u32) -> u32 {
    let mut last_time: &u32 = timeline.keys().last().unwrap();
    for (time, _) in timeline.iter() {
        if time > &seconds {
            return *last_time;
        }
        last_time = time;
    }
    *timeline.keys().last().unwrap()
}

fn get_palette(
    all_palettes: &JsonValue,
    timeline: &BTreeMap<u32, &str>,
    seconds: u32,
) -> [[u8; 3]; 256] {
    let time = get_palette_time(timeline, seconds);
    let palette_raw: &JsonValue = &all_palettes[*timeline.get(&time).unwrap()];

    let colors = palette_raw["colors"]
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
    palette
}

fn main() -> std::io::Result<()> {
    // Json processing
    let mut file = File::open("scenes/April-Clear.json").unwrap();
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
    println!("{} x {}", width, height);

    let palettes: JsonValue = parsed["palettes"].clone();

    let timeline: BTreeMap<u32, &str> = parsed["timeline"]
        .entries()
        .map(|(time, palette)| (time.parse::<u32>().unwrap(), palette.as_str().unwrap()))
        .collect();

    let now = Local::now();
    let seconds = now.num_seconds_from_midnight();

    let palette = get_palette(&palettes, &timeline, seconds);

    // GL
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new().with_inner_size(PhysicalSize::new(800, 600));
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
                // Flip the y coordinate
                vec2 flippedUV = vec2(uv.x, 1.0 - uv.y);
                // Get the color index of the current fragment
                float index = texture(color_indices, flippedUV).r;
                // Get the color from the palette using the index
                vec3 color = texture(palette, index).rgb;
    
                // Set the final fragment color
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
            gl::R8 as i32, // Assuming color index data is single-channel
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
