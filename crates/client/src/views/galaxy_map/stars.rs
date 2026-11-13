//! Draws the galaxy map's point clouds with OpenGL, inside egui's frame (a paint callback).
//!
//! Three static vertex buffers of `[x, y, z, w]` points: the density cells (w = systems in
//! the cell), an even sample of the systems around the focus and every system close to it
//! (w = 1 when populated). Each layer owns a sphere around the focus: the coarser layer
//! leaves it out and the finer one fades out at its edge, so zooming hands over between
//! them without seams. Buffers are uploaded once per data change and drawn as round point
//! sprites with additive blending, three draw calls a frame whatever the camera does.
//! Everything here is a few MB of VRAM at most, which matters next to the game.

use std::sync::Arc;

use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use tracing::{error, info};

/// Points to draw, tagged so the GPU copy is replaced only when the data changes.
#[derive(Clone, Default)]
pub struct PointCloud {
    /// `[x, y, z, w]` per point.
    pub data: Arc<Vec<[f32; 4]>>,
    /// Bumped on every change.
    pub generation: u64,
}

/// What one frame draws: the camera and both clouds with their uniforms.
pub struct Frame {
    pub view_proj: [f32; 16],
    /// Physical pixels per ly at unit depth.
    pub px_per_ly: f32,
    /// Camera distance to its target, ly: sizes stars relative to the focus.
    pub focus_depth: f32,
    pub density: PointCloud,
    pub cell_ly: f32,
    /// `ln(1 + systems)` of the fullest cell, to normalise brightness.
    pub max_log: f32,
    pub density_alpha: f32,
    /// Sphere (centre, radius; 0 when unused) holding every system, drawn singly.
    pub near: [f32; 4],
    pub stars: PointCloud,
    pub stars_alpha: f32,
    /// Sphere holding the sampled systems.
    pub sampled_sphere: [f32; 4],
    pub sampled: PointCloud,
    pub sampled_alpha: f32,
}

#[derive(Default)]
pub struct StarRenderer {
    gpu: Option<Gpu>,
    /// Shader setup failed once; don't retry (and log) every frame.
    broken: bool,
}

struct Buffer {
    buffer: glow::Buffer,
    generation: Option<u64>,
    len: i32,
}

struct Gpu {
    program: glow::Program,
    vao: Option<glow::VertexArray>,
    density: Buffer,
    sampled: Buffer,
    stars: Buffer,
    /// Desktop compatibility contexts only hand out `gl_PointCoord` with point sprites on.
    point_sprites: bool,
    u_mvp: Option<glow::UniformLocation>,
    u_mode: Option<glow::UniformLocation>,
    u_px_per_ly: Option<glow::UniformLocation>,
    u_cell_ly: Option<glow::UniformLocation>,
    u_max_log: Option<glow::UniformLocation>,
    u_alpha: Option<glow::UniformLocation>,
    u_near: Option<glow::UniformLocation>,
    u_sampled: Option<glow::UniformLocation>,
    u_cover: Option<glow::UniformLocation>,
    u_focus_depth: Option<glow::UniformLocation>,
}

const VERTEX: &str = r#"
#if NEW_SHADER_INTERFACE
    #define I in
    #define O out
#else
    #define I attribute
    #define O varying
#endif
#ifdef GL_ES
    precision highp float;
#endif

uniform mat4 u_mvp;
uniform float u_mode;        // 0: density cells, 1: every system, 2: sampled systems
uniform float u_px_per_ly;
uniform float u_cell_ly;
uniform float u_max_log;
uniform float u_alpha;
uniform vec4 u_near;         // xyz centre, w radius (0: layer not shown)
uniform vec4 u_sampled;
uniform vec2 u_cover;        // alpha of the every-system and the sampled layer
uniform float u_focus_depth;

bool inside(vec3 p, vec4 sphere, float frac) {
    vec3 d = p - sphere.xyz;
    float r = sphere.w * frac;
    return dot(d, d) < r * r;
}

// 1 well inside the sphere, falling to 0 over its outer part.
float edge_fade(vec3 p, vec4 sphere, float from) {
    return 1.0 - smoothstep(from, 1.0, length(p - sphere.xyz) / max(sphere.w, 1.0));
}

I vec4 a_point;
O vec4 v_color;
O float v_sharp;

void main() {
    vec4 clip = u_mvp * vec4(a_point.xyz, 1.0);
    gl_Position = clip;
    float depth = max(clip.w, 0.001);
    if (u_mode < 0.5) {
        float b = clamp(log(1.0 + a_point.w) / u_max_log, 0.0, 1.0);
        float size = u_cell_ly * 1.8 * u_px_per_ly / depth;
        float a = u_alpha * (0.035 + 0.55 * b * b);
        // Seen from afar the cells are small glows that add up to the galaxy's shape; up
        // close each would cover the screen, so they thin out to a haze as they grow.
        a *= min(1.0, pow(12.0 / size, 1.5));
        size = min(size, 160.0);
        // Inside a finer layer's sphere the cells give way to it, as far as it is shown.
        if (inside(a_point.xyz, u_sampled, 1.0)) {
            a *= 1.0 - u_cover.y;
        } else if (inside(a_point.xyz, u_near, 1.0)) {
            a *= 1.0 - u_cover.x;
        }
        v_color = vec4(mix(vec3(0.30, 0.42, 0.85), vec3(1.0, 0.86, 0.66), b), a);
        v_sharp = 2.2;
        gl_PointSize = max(size, 1.5);
    } else if (u_mode < 1.5) {
        float populated = step(0.5, a_point.w);
        float near = u_focus_depth / depth;
        float size = clamp(3.6 * sqrt(near), 1.5, 7.0) + 0.6 * populated;
        vec3 col = mix(vec3(0.74, 0.84, 1.0), vec3(1.0, 0.64, 0.28), populated);
        // Stars behind the focus dim, so the neighbourhood reads in depth.
        float a = u_alpha * clamp(1.3 * near * sqrt(near), 0.12, 1.0) * edge_fade(a_point.xyz, u_near, 0.85);
        v_color = vec4(col, a);
        v_sharp = 4.0;
        gl_PointSize = size;
    } else {
        float populated = step(0.5, a_point.w);
        float near = u_focus_depth / depth;
        float size = clamp(2.6 * sqrt(near), 1.2, 4.0);
        vec3 col = mix(vec3(0.70, 0.80, 1.0), vec3(1.0, 0.64, 0.28), populated);
        float a = u_alpha * clamp(0.8 * near, 0.25, 0.8) * edge_fade(a_point.xyz, u_sampled, 0.75);
        // Where every system is drawn, the sample would only double up.
        if (inside(a_point.xyz, u_near, 0.85)) {
            a *= 1.0 - u_cover.x;
        }
        v_color = vec4(col, a);
        v_sharp = 4.0;
        gl_PointSize = size;
    }
}
"#;

const FRAGMENT: &str = r#"
#ifdef GL_ES
    precision mediump float;
#endif
#if NEW_SHADER_INTERFACE
    in vec4 v_color;
    in float v_sharp;
    out vec4 f_color;
    #define gl_FragColor f_color
#else
    varying vec4 v_color;
    varying float v_sharp;
#endif

void main() {
    vec2 d = gl_PointCoord * 2.0 - 1.0;
    float r2 = dot(d, d);
    if (r2 > 1.0) {
        discard;
    }
    float a = v_color.a * exp(-r2 * v_sharp);
    gl_FragColor = vec4(v_color.rgb * a, a);
}
"#;

/// `glow::POINT_SPRITE` (compatibility profiles only, so glow has no constant for it).
const POINT_SPRITE: u32 = 0x8861;
const CONTEXT_PROFILE_MASK: u32 = 0x9126;
const CONTEXT_COMPATIBILITY_PROFILE_BIT: i32 = 0x2;

impl StarRenderer {
    /// Draws `frame` into the current viewport (egui has set it to the map's rect).
    pub fn paint(&mut self, gl: &glow::Context, frame: &Frame) {
        if self.broken {
            return;
        }
        if self.gpu.is_none() {
            match unsafe { Gpu::new(gl) } {
                Ok(gpu) => self.gpu = Some(gpu),
                Err(e) => {
                    error!("Galaxy map: OpenGL setup failed, the map stays empty: {e}");
                    self.broken = true;
                    return;
                }
            }
        }
        let gpu = self.gpu.as_mut().unwrap();
        unsafe { gpu.paint(gl, frame) };
    }
}

impl Gpu {
    unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        let version = egui_glow::ShaderVersion::get(gl);
        let header = format!("{}\n#define NEW_SHADER_INTERFACE {}\n", version.version_declaration(), version.is_new_shader_interface() as i32);
        let program = unsafe { gl.create_program()? };
        let mut shaders = Vec::new();
        for (kind, src) in [(glow::VERTEX_SHADER, VERTEX), (glow::FRAGMENT_SHADER, FRAGMENT)] {
            let shader = unsafe { gl.create_shader(kind)? };
            unsafe {
                gl.shader_source(shader, &format!("{header}{src}"));
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    return Err(format!("shader: {}", gl.get_shader_info_log(shader)));
                }
                gl.attach_shader(program, shader);
            }
            shaders.push(shader);
        }
        unsafe {
            gl.bind_attrib_location(program, 0, "a_point");
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(format!("link: {}", gl.get_program_info_log(program)));
            }
            for s in shaders {
                gl.detach_shader(program, s);
                gl.delete_shader(s);
            }
        }

        let ver = gl.version();
        let compat = !ver.is_embedded
            && ((ver.major, ver.minor) < (3, 2)
                || unsafe { gl.get_parameter_i32(CONTEXT_PROFILE_MASK) } & CONTEXT_COMPATIBILITY_PROFILE_BIT != 0);
        // Asking an old context for its profile is itself an error; don't leave it for egui.
        while unsafe { gl.get_error() } != glow::NO_ERROR {}
        let vao = unsafe { gl.create_vertex_array() }.ok();
        info!(?ver, compat, vao = vao.is_some(), "Galaxy map: OpenGL ready ({version:?})");

        let buffer = || -> Result<Buffer, String> { Ok(Buffer { buffer: unsafe { gl.create_buffer()? }, generation: None, len: 0 }) };
        let loc = |name: &str| unsafe { gl.get_uniform_location(program, name) };
        Ok(Self {
            program,
            vao,
            density: buffer()?,
            sampled: buffer()?,
            stars: buffer()?,
            point_sprites: compat,
            u_mvp: loc("u_mvp"),
            u_mode: loc("u_mode"),
            u_px_per_ly: loc("u_px_per_ly"),
            u_cell_ly: loc("u_cell_ly"),
            u_max_log: loc("u_max_log"),
            u_alpha: loc("u_alpha"),
            u_near: loc("u_near"),
            u_sampled: loc("u_sampled"),
            u_cover: loc("u_cover"),
            u_focus_depth: loc("u_focus_depth"),
        })
    }

    unsafe fn paint(&mut self, gl: &glow::Context, f: &Frame) {
        unsafe {
            gl.use_program(Some(self.program));
            if let Some(vao) = self.vao {
                gl.bind_vertex_array(Some(vao));
            }
            if !gl.version().is_embedded {
                gl.enable(glow::PROGRAM_POINT_SIZE);
            }
            if self.point_sprites {
                gl.enable(POINT_SPRITE);
            }
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE);

            gl.uniform_matrix_4_f32_slice(self.u_mvp.as_ref(), false, &f.view_proj);
            gl.uniform_1_f32(self.u_px_per_ly.as_ref(), f.px_per_ly);
            gl.uniform_1_f32(self.u_cell_ly.as_ref(), f.cell_ly);
            gl.uniform_1_f32(self.u_max_log.as_ref(), f.max_log.max(1.0));
            gl.uniform_4_f32_slice(self.u_near.as_ref(), &f.near);
            gl.uniform_4_f32_slice(self.u_sampled.as_ref(), &f.sampled_sphere);
            gl.uniform_2_f32(self.u_cover.as_ref(), f.stars_alpha, f.sampled_alpha);
            gl.uniform_1_f32(self.u_focus_depth.as_ref(), f.focus_depth);

            for (mode, buf, cloud, alpha) in [
                (0.0, &mut self.density, &f.density, f.density_alpha),
                (2.0, &mut self.sampled, &f.sampled, f.sampled_alpha),
                (1.0, &mut self.stars, &f.stars, f.stars_alpha),
            ] {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(buf.buffer));
                if buf.generation != Some(cloud.generation) {
                    let bytes = std::slice::from_raw_parts(cloud.data.as_ptr() as *const u8, std::mem::size_of_val(cloud.data.as_slice()));
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
                    buf.generation = Some(cloud.generation);
                    buf.len = cloud.data.len() as i32;
                }
                if buf.len == 0 || alpha <= 0.0 {
                    continue;
                }
                gl.enable_vertex_attrib_array(0);
                gl.vertex_attrib_pointer_f32(0, 4, glow::FLOAT, false, 16, 0);
                gl.uniform_1_f32(self.u_mode.as_ref(), mode);
                gl.uniform_1_f32(self.u_alpha.as_ref(), alpha);
                gl.draw_arrays(glow::POINTS, 0, buf.len);
            }

            gl.disable_vertex_attrib_array(0);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            if self.vao.is_some() {
                gl.bind_vertex_array(None);
            }
            if self.point_sprites {
                gl.disable(POINT_SPRITE);
            }
            if !gl.version().is_embedded {
                gl.disable(glow::PROGRAM_POINT_SIZE);
            }
        }
    }
}
