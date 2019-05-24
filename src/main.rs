#[macro_use] extern crate gfx;

extern crate gfx_window_glutin;
extern crate glutin;

use gfx::traits::FactoryExt;
use gfx::Device;
use gfx_window_glutin as gfx_glutin;
use glutin::*;

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "a_Pos",
        uv: [f32; 2] = "a_Uv",
        color: [f32; 3] = "a_Color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        awesome: gfx::TextureSampler<[f32; 4]> = "t_Awesome",
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

pub fn main() {
    let mut window_size = (800.0, 800.0);

    let mut cube = Pseudocube::new();
    cube.add_square(0.0, 0.0, 1.0, WHITE);

    let events_loop = glutin::EventsLoop::new();
    let builder = glutin::WindowBuilder::new()
        .with_title("Thomas's awesome window".to_string())
        .with_dimensions(window_size.0 as u32, window_size.1 as u32)
        .with_vsync();
    let (window, mut device, mut factory, main_color, mut main_depth) =
        gfx_glutin::init::<ColorFormat, DepthFormat>(builder, &events_loop);

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let pso = factory.create_pipeline_simple(
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/rect_150.glslv")),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/rect_150.glslf")),
        pipe::new()
    ).unwrap();
    let (vertices, indices) = cube.get_vertices_indices();
    let (_vertex_buffer, mut slice) =
        factory.create_vertex_buffer_with_slice(&vertices, &*indices);
    
    let texture = load_texture(&mut factory, "assets/Tooltips.png");
    let sampler = factory.create_sampler_linear();

    let mut data = pipe::Data {
        vbuf: _vertex_buffer,
        awesome: (texture, sampler),
        out: main_color
    };

    let mut running = true;
    let mut needs_update = false;  

    while running {
        if needs_update {
            let (vs, is) = cube.get_vertices_indices();
            let (vbuf, sl) = factory.create_vertex_buffer_with_slice(&vs, &*is);

            data.vbuf = vbuf;
            slice = sl;

            needs_update = false
        }
        events_loop.poll_events(|glutin::Event::WindowEvent{window_id: _, event}| {
            use glutin::WindowEvent::*;
            match event {
                KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape), _)
                | Closed => running = false,
                Resized(w, h) => {
                    gfx_glutin::update_views(&window, &mut data.out, &mut main_depth);
                    cube.update_ratio(w as f32 / h as f32);
                    window_size = (w as f32, h as f32);
                    needs_update = true
                },
                MouseMoved(x, y) => {
                    cube.update_cursor_position(
                        x as f32 / window_size.0,
                        y as f32 / window_size.1
                    );
                    needs_update = true
                },
                MouseInput(ElementState::Pressed, MouseButton::Left) =>
                    cube.start_growing(),
                MouseInput(ElementState::Released, MouseButton::Left) =>
                    cube.stop_growing(),
                _ => (),
            }

            cube.tick();
        });

        encoder.clear(&data.out, BLACK);
        encoder.draw(&slice, &pso, &data);
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}

#[derive(Debug, Clone, Copy)]
struct Square {
    pub pos: (f32, f32),
    pub size: f32,
    pub color: [f32; 3]
}

// A cube is a pile of infinitely (as continuum) many squares
// This data stucture is finite, so we call it “pseudo”
#[derive(Debug)]
struct Pseudocube {
    squares: Vec<Square>,
    ratio: f32,
    cursor: Cursor,
}

impl Pseudocube {
    pub fn new() -> Self {
        Pseudocube {
            squares: vec![],
            ratio: 1.0,
            cursor: Cursor::Growing((1.0, 1.0), 1.0, WHITE),
        }
    }

    pub fn add_square(&mut self, x: f32, y: f32, size: f32, color: [f32; 3]) {
        let sq = Square {
            pos: (x, y),
            size, color
        };
        self.squares.push(sq);
    }

    pub fn get_vertices_indices(&self) -> (Vec<Vertex>, Vec<u16>) {
        let (mut vs, mut is) = (vec![], vec![]);
        for (i, sq) in self.squares.iter().enumerate() {
            let (pos, half) = (sq.pos, 0.5 * sq.size);
            let i = i as u16;

            let (hx, hy);
            if self.ratio > 1.0 {
                hx = half / self.ratio;
                hy = half;
            }
            else {
                hx = half;
                hy = half * self.ratio;
            }

            vs.extend(&[
                Vertex { pos: [pos.0 + hx, pos.1 - hy], uv: [1.0, 1.0], color: sq.color },
                Vertex { pos: [pos.0 - hx, pos.1 - hy], uv: [0.0, 1.0], color: sq.color },
                Vertex { pos: [pos.0 - hx, pos.1 + hy], uv: [0.0, 0.0], color: sq.color },
                Vertex { pos: [pos.0 + hx, pos.1 + hy], uv: [1.0, 0.0], color: sq.color },
            ]);
            is.extend(&[
                4*i, 4*i + 1, 4*i + 2, 4*i + 2, 4*i + 3, 4*i
            ]);
        }

        (vs, is)
    }

    pub fn update_ratio(&mut self, ratio: f32) {
        self.ratio = ratio
    }

    pub fn update_cursor_position(&mut self, x: f32, y: f32) {
        let x = 2.0*x - 1.0;
        let y = -2.0*y + 1.0;
        let cursor = match self.cursor {
            Cursor::Plain(_, color) => Cursor::Plain((x, y), color),
            Cursor::Growing(_, size, color) => Cursor::Growing((x, y), size, color),
        };
        self.cursor = cursor;
    }

    pub fn start_growing(&mut self) {
        if let Cursor::Plain(xy, color) = self.cursor {
            self.cursor = Cursor::Growing(xy, 0.05, color)
        }
    }

    pub fn stop_growing(&mut self) {
        if let Cursor::Growing(xy, size, color) = self.cursor {
            self.squares.push (Cursor::Growing(xy, size, color).to_square());
            self.cursor = Cursor::Plain(xy, rand::random())
        }
    }

    pub fn tick(&mut self) {
        if let Cursor::Growing(xy, size, color) = self.cursor {
            self.cursor = Cursor::Growing(xy, size + 0.01, color)
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Cursor {
    Plain((f32, f32), [f32; 3]),
    Growing((f32, f32), f32, [f32; 3])
}

impl Cursor {
    fn to_square(self) -> Square {
        match self {
            Cursor::Plain(xy, color) => Square { pos: xy, size: 0.05, color },
            Cursor::Growing(xy, size, color) => Square { pos: xy, size, color },
        }
    }
}


fn load_texture<F, R>(factory: &mut F, path: &str) -> gfx::handle::ShaderResourceView<R, [f32; 4]>
    where F: gfx::Factory<R>, R: gfx::Resources
{
    let img = image::open(path).unwrap().to_rgba();
    let (width, height) = img.dimensions();
    let kind = gfx::texture::Kind::D2(width as u16, height as u16, gfx::texture::AaMode::Single);
    let (_, view) = factory.create_texture_immutable_u8::<ColorFormat>(kind, &[&img]).unwrap();
    view
}