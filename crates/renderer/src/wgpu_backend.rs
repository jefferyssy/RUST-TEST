//! wgpu 渲染后端 —— 支持 DX12 / Vulkan / Metal
//!
//! wgpu 是 Rust 原生跨平台图形 API 抽象层。
//! Phase 0 实现矩形填充和基本文本占位。

use crate::RenderBackend;
use crate::TextRenderer;
use dom::Color;
use render_tree::{DisplayList, PaintCommand};

/// 矩形顶点着色器 —— WGSL 源码
/// 使用实例化渲染：1 个 vertex buffer（单位四边形）+ 1 个 instance buffer（每矩形数据）
const RECT_VERTEX_SHADER: &str = r#"
struct ScreenUniform {
    screen_w: f32,
    screen_h: f32,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniform;

struct VertexInput {
    @location(0) pos: vec2<f32>,
}

struct InstanceInput {
    @location(1) data0: vec4<f32>,  // rect_x, rect_y, rect_w, rect_h
    @location(2) data1: vec4<f32>,  // r, g, b, a
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    // 将 0..1 映射到 rect 的实际像素位置
    let px = inst.data0.x + vert.pos.x * inst.data0.z;
    let py = inst.data0.y + vert.pos.y * inst.data0.w;
    // 像素坐标 → NDC（-1 到 1）
    let ndc_x = (px / screen.screen_w) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / screen.screen_h) * 2.0;

    return VertexOutput(
        vec4<f32>(ndc_x, ndc_y, 0.0, 1.0),
        inst.data1,
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// 文字顶点着色器 —— 使用字形图集纹理采样
const TEXT_VERTEX_SHADER: &str = r#"
struct ScreenUniform {
    screen_w: f32,
    screen_h: f32,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniform;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct VertexInput {
    @location(0) pos: vec2<f32>,
}

struct InstanceInput {
    @location(1) data0: vec4<f32>,  // dst_x, dst_y, dst_w, dst_h
    @location(2) data1: vec4<f32>,  // src_u, src_v, src_w, src_h
    @location(3) data2: vec4<f32>,  // r, g, b, a
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    let px = inst.data0.x + vert.pos.x * inst.data0.z;
    let py = inst.data0.y + vert.pos.y * inst.data0.w;
    let ndc_x = (px / screen.screen_w) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / screen.screen_h) * 2.0;

    let u = inst.data1.x + vert.pos.x * inst.data1.z;
    let v = inst.data1.y + vert.pos.y * inst.data1.w;

    return VertexOutput(
        vec4<f32>(ndc_x, ndc_y, 0.0, 1.0),
        vec2<f32>(u, v),
        inst.data2,
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas, atlas_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;

/// 圆角矩形着色器 —— SDF 渲染填充 + 可选边框
const BORDER_VERTEX_SHADER: &str = r#"
struct ScreenUniform {
    screen_w: f32,
    screen_h: f32,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniform;

struct VertexInput {
    @location(0) pos: vec2<f32>,
}

struct InstanceInput {
    @location(1) data0: vec4<f32>,  // rect_x, rect_y, rect_w, rect_h
    @location(2) data1: vec4<f32>,  // r_tl, r_tr, r_br, r_bl
    @location(3) data2: vec4<f32>,  // fill_r, fill_g, fill_b, fill_a
    @location(4) data3: vec4<f32>,  // border_w, border_r, border_g, border_b
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) radii: vec4<f32>,
    @location(3) fill_color: vec4<f32>,
    @location(4) border_data: vec4<f32>,
}

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    let px = inst.data0.x + vert.pos.x * inst.data0.z;
    let py = inst.data0.y + vert.pos.y * inst.data0.w;
    let ndc_x = (px / screen.screen_w) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / screen.screen_h) * 2.0;

    return VertexOutput(
        vec4<f32>(ndc_x, ndc_y, 0.0, 1.0),
        vert.pos,
        vec2<f32>(inst.data0.z * 0.5, inst.data0.w * 0.5),
        inst.data1,
        inst.data2,
        inst.data3,
    );
}

fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
    let max_r = min(half_size.x, half_size.y);
    let cr = clamp(r, 0.0, max_r);
    let q = abs(p) - half_size + cr;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - cr;
}

fn per_corner_radius(p: vec2<f32>, r: vec4<f32>) -> f32 {
    if p.x >= 0.0 && p.y >= 0.0 {
        return r.x;
    } else if p.x <= 0.0 && p.y >= 0.0 {
        return r.y;
    } else if p.x <= 0.0 && p.y <= 0.0 {
        return r.z;
    } else {
        return r.w;
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = (in.local_pos - 0.5) * in.half_size * 2.0;
    let aa = 1.0;

    // 外层 SDF（填充/边框外沿）
    let r_outer = per_corner_radius(p, in.radii);
    let d_outer = rounded_rect_sdf(p, in.half_size, r_outer);

    let border_w = in.border_data.x;
    let border_r = in.border_data.y;
    let border_g = in.border_data.z;
    let border_a = in.border_data.w;
    let fill_color = in.fill_color.rgb;
    let fill_alpha = in.fill_color.a;

    if border_w > 0.0 {
        // 内层 SDF（边框内沿）
        let border_half = max(in.half_size - border_w, vec2<f32>(0.0));
        let r_inner = clamp(r_outer - border_w, 0.0, min(border_half.x, border_half.y));
        let d_inner = rounded_rect_sdf(p, border_half, r_inner);

        // outer_t: 1 = 在填充/边框外部矩形内, 0 = 在外面
        let outer_t = 1.0 - smoothstep(-aa, aa, d_outer);
        // inner_t: 1 = 在内层矩形内, 0 = 在外面
        let inner_t = 1.0 - smoothstep(-aa, aa, d_inner);

        // 边框 = 外层内 ∩ 内层外
        let border_t = outer_t * (1.0 - inner_t);
        // 填充 = 内层内
        let fill_t = inner_t;

        let border_rgb = vec3<f32>(border_r, border_g, border_r);
        let alpha = fill_t * fill_alpha + border_t * border_a;
        let rgb = (fill_color * fill_alpha * fill_t + border_rgb * border_a * border_t)
                  / max(alpha, 0.001);
        return vec4<f32>(rgb, alpha);
    } else {
        // 仅填充（无边框）
        let alpha = fill_alpha * (1.0 - smoothstep(-aa, 0.0, d_outer));
        return vec4<f32>(fill_color, alpha);
    }
}
"#;

/// 阴影着色器 —— SDF 圆角矩形 + 高斯模糊衰减
const SHADOW_VERTEX_SHADER: &str = r#"
struct ScreenUniform {
    screen_w: f32,
    screen_h: f32,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniform;

struct VertexInput {
    @location(0) pos: vec2<f32>,
}

struct InstanceInput {
    @location(1) data0: vec4<f32>,  // rect_x, rect_y, rect_w, rect_h (含 offset+spread+blur 的完整范围)
    @location(2) data1: vec4<f32>,  // r_tl, r_tr, r_br, r_bl
    @location(3) data2: vec4<f32>,  // shadow_r, shadow_g, shadow_b, shadow_a
    @location(4) data3: vec4<f32>,  // blur_radius, 0, 0, 0
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) radii: vec4<f32>,
    @location(3) shadow_color: vec4<f32>,
    @location(4) blur_radius: f32,
}

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    let px = inst.data0.x + vert.pos.x * inst.data0.z;
    let py = inst.data0.y + vert.pos.y * inst.data0.w;
    let ndc_x = (px / screen.screen_w) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / screen.screen_h) * 2.0;

    return VertexOutput(
        vec4<f32>(ndc_x, ndc_y, 0.0, 1.0),
        vert.pos,
        vec2<f32>(inst.data0.z * 0.5, inst.data0.w * 0.5),
        inst.data1,
        inst.data2,
        inst.data3.x,
    );
}

fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
    let max_r = min(half_size.x, half_size.y);
    let cr = clamp(r, 0.0, max_r);
    let q = abs(p) - half_size + cr;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - cr;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let blur = max(in.blur_radius, 0.5);
    let solid_half = max(in.half_size - blur, vec2<f32>(0.0));
    let r = clamp(in.radii.x, 0.0, min(solid_half.x, solid_half.y));

    let p = (in.local_pos - 0.5) * in.half_size * 2.0;
    let d = rounded_rect_sdf(p, solid_half, r);

    // 使用 0.6 * blur 使过渡更集中，阴影更明显
    let falloff = blur * 0.6;
    let alpha = in.shadow_color.a * (1.0 - smoothstep(0.0, falloff, d));
    return vec4<f32>(in.shadow_color.rgb, alpha);
}
"#;

/// wgpu 渲染后端 —— 矩形渲染 + 文本渲染
pub struct WgpuBackend {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// 物理像素尺寸（surface 配置用）
    physical_size: (u32, u32),
    /// 逻辑像素尺寸（shader NDC 转换用，与布局坐标一致）
    logical_size: (u32, u32),
    // 矩形渲染管线
    rect_pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    instance_capacity: usize,
    screen_uniform_buf: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,
    // 文字渲染
    text_renderer: TextRenderer,
    text_pipeline: wgpu::RenderPipeline,
    text_bind_group: wgpu::BindGroup,
    text_instance_buf: wgpu::Buffer,
    text_instance_capacity: usize,
    // Phase 1: 圆角矩形渲染管线
    border_pipeline: wgpu::RenderPipeline,
    border_instance_buf: wgpu::Buffer,
    border_instance_capacity: usize,
    // Phase 1: 阴影渲染管线
    shadow_pipeline: wgpu::RenderPipeline,
    shadow_instance_buf: wgpu::Buffer,
    shadow_instance_capacity: usize,
    // Phase 1+: 新增渲染管线
    image_pipeline: Option<wgpu::RenderPipeline>,
    /// 裁剪栈（嵌套 clip 区域）
    clip_stack: Vec<dom::Rect<f32>>,
    /// 透明度栈（嵌套 opacity 值）
    opacity_stack: Vec<f32>,
    /// P1-9: 上次写入的实例数（用于增量 buffer 写入）
    last_rect_count: usize,
    last_border_count: usize,
    last_shadow_count: usize,
    last_text_count: usize,
    last_screen_size: (u32, u32),
    /// P1-9: DisplayList 版本号追踪（检测 DL 是否发生变更）
    last_dl_generation: u64,
}

impl WgpuBackend {
    /// 创建 wgpu 后端（异步初始化）
    ///
    /// window: winit 窗口引用
    pub async fn new(window: &winit::window::Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        // Safety: wgpu 23 中 Surface<'window> 的生命周期与窗口引用绑定。
        // 调用方（WebWindow::run）确保窗口的 Rc 和渲染器的 Rc 在同一闭包中存活，
        // 因此窗口一定比 Surface 更长寿。
        let surface = unsafe {
            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(
                instance.create_surface(window).unwrap(),
            )
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Browser Engine Device"),
                    required_features: wgpu::Features::PUSH_CONSTANTS,
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .unwrap();

        let physical_size = window.inner_size();
        let phys_w = physical_size.width.max(1);
        let phys_h = physical_size.height.max(1);

        // 逻辑尺寸 = 物理尺寸 / 缩放因子（与布局坐标一致）
        let scale = window.scale_factor();
        let log_w = (physical_size.width as f64 / scale).max(1.0) as u32;
        let log_h = (physical_size.height as f64 / scale).max(1.0) as u32;

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: phys_w,
            height: phys_h,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        eprintln!("[diag] wgpu init: physical={}x{}, logical={}x{}, scale={}, format={:?}", phys_w, phys_h, log_w, log_h, scale, format);

        // 创建单位四边形顶点缓冲
        let quad_verts: [f32; 12] = [
            0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
            1.0, 0.0, 1.0, 1.0, 0.0, 1.0,
        ];
        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Vertex Buffer"),
            size: std::mem::size_of_val(&quad_verts) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buf, 0, bytemuck::cast_slice(&quad_verts));

        // 实例数据缓冲（动态大小，在 render 时按需重建）
        let initial_cap = 256;
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Data Buffer"),
            size: (initial_cap * 8 * std::mem::size_of::<f32>()) as u64, // 8 floats/rect
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 屏幕尺寸 uniform 缓冲
        let screen_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screen Uniform Buffer"),
            size: 8, // 2 × f32
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Uniform bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Screen Uniform BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Screen BindGroup"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_uniform_buf.as_entire_binding(),
            }],
        });

        // 创建渲染管线（vertex buffer + instance buffer + uniform）
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(RECT_VERTEX_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rect Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // 顶点布局：buffer 0 = 单位四边形顶点，buffer 1 = 实例数据
        let vertex_buffers: [wgpu::VertexBufferLayout; 2] = [
            // Buffer 0: 单位四边形顶点 (vec2<f32>)
            wgpu::VertexBufferLayout {
                array_stride: 8, // 2 × f32
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            // Buffer 1: 实例数据 (vec4 + vec4 = 8 f32s)
            wgpu::VertexBufferLayout {
                array_stride: 32, // 8 × f32
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16, // 4 × f32
                        shader_location: 2,
                    },
                ],
            },
        ];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ---- 圆角矩形渲染管线（SDF）----
        let border_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Border Shader"),
            source: wgpu::ShaderSource::Wgsl(BORDER_VERTEX_SHADER.into()),
        });

        let border_vertex_buffers: [wgpu::VertexBufferLayout; 2] = [
            wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: 64, // 4 vec4s = 16 f32s
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 32,
                        shader_location: 3,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 48,
                        shader_location: 4,
                    },
                ],
            },
        ];

        let border_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Border Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let border_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Border Render Pipeline"),
                layout: Some(&border_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &border_shader,
                    entry_point: Some("vs_main"),
                    buffers: &border_vertex_buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &border_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let border_init_cap = 256;
        let border_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Border Instance Buffer"),
            size: (border_init_cap * 16 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---- 阴影渲染管线（SDF 圆角矩形 + 模糊）----
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_VERTEX_SHADER.into()),
        });

        let shadow_vertex_buffers: [wgpu::VertexBufferLayout; 2] = [
            wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: 64, // 4 vec4s = 16 f32s
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 32,
                        shader_location: 3,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 48,
                        shader_location: 4,
                    },
                ],
            },
        ];

        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let shadow_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Shadow Render Pipeline"),
                layout: Some(&shadow_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shadow_shader,
                    entry_point: Some("vs_main"),
                    buffers: &shadow_vertex_buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shadow_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let shadow_init_cap = 128;
        let shadow_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shadow Instance Buffer"),
            size: (shadow_init_cap * 16 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---- 文字渲染管线 ----
        let text_renderer = TextRenderer::new(&device, config.format);

        let text_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Text BGL"),
                entries: &[
                    // @binding(0): screen uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // @binding(1): glyph atlas texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // @binding(2): glyph atlas sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
            });

        let text_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text BindGroup"),
            layout: &text_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: screen_uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        text_renderer.atlas_texture_view(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(
                        text_renderer.atlas_sampler(),
                    ),
                },
            ],
        });

        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(TEXT_VERTEX_SHADER.into()),
        });

        let text_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Text Pipeline Layout"),
                bind_group_layouts: &[&text_bind_group_layout],
                push_constant_ranges: &[],
            });

        let text_vertex_buffers: [wgpu::VertexBufferLayout; 2] = [
            // Buffer 0: 单位四边形顶点（与矩形管线共享 vertex_buf）
            wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            // Buffer 1: 字形实例数据（12 f32s = 48 bytes）
            wgpu::VertexBufferLayout {
                array_stride: 48,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 32,
                        shader_location: 3,
                    },
                ],
            },
        ];

        let text_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Text Render Pipeline"),
                layout: Some(&text_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &text_shader,
                    entry_point: Some("vs_main"),
                    buffers: &text_vertex_buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &text_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let text_init_cap = 512;
        let text_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Instance Buffer"),
            size: (text_init_cap * 12 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            surface,
            device,
            queue,
            config,
            physical_size: (phys_w, phys_h),
            logical_size: (log_w, log_h),
            rect_pipeline: pipeline,
            vertex_buf,
            instance_buf,
            instance_capacity: initial_cap,
            screen_uniform_buf,
            screen_bind_group,
            text_renderer,
            text_pipeline,
            text_bind_group,
            text_instance_buf,
            text_instance_capacity: text_init_cap,
            border_pipeline,
            border_instance_buf,
            border_instance_capacity: border_init_cap,
            shadow_pipeline,
            shadow_instance_buf,
            shadow_instance_capacity: shadow_init_cap,
            image_pipeline: None,
            clip_stack: Vec::new(),
            opacity_stack: Vec::new(),
            last_rect_count: 0,
            last_border_count: 0,
            last_shadow_count: 0,
            last_text_count: 0,
            last_screen_size: (0, 0),
            last_dl_generation: 0,
        }
    }

    /// 窗口尺寸变更（含逻辑尺寸更新）
    pub fn resize_with_scale(&mut self, phys_w: u32, phys_h: u32, scale: f64) {
        if phys_w > 0 && phys_h > 0 {
            self.physical_size = (phys_w, phys_h);
            self.config.width = phys_w;
            self.config.height = phys_h;
            self.surface.configure(&self.device, &self.config);
        }
        let log_w = (phys_w as f64 / scale).max(1.0) as u32;
        let log_h = (phys_h as f64 / scale).max(1.0) as u32;
        self.logical_size = (log_w, log_h);
        eprintln!("[diag] resize: physical={}x{}, logical={}x{}, scale={}", phys_w, phys_h, log_w, log_h, scale);
    }

    // Phase 1: 裁剪栈操作
    pub fn push_clip(&mut self, rect: dom::Rect<f32>) {
        self.clip_stack.push(rect);
    }

    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    // Phase 1: 透明度栈操作
    pub fn push_opacity(&mut self, alpha: f32) {
        self.opacity_stack.push(alpha);
    }

    pub fn pop_opacity(&mut self) {
        self.opacity_stack.pop();
    }

    // Phase 2+: 创建 border/shadow/image 着色器管线
    // fn create_border_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline;
    // fn create_shadow_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline;
    // fn create_image_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline;
}

impl RenderBackend for WgpuBackend {
    /// 渲染一帧 — 收集所有矩形/文本命令，通过实例化 draw call 渲染
    fn render(&mut self, display_list: &DisplayList) {
        // 1. 收集实例数据
        let mut rect_instances: Vec<f32> = Vec::new();
        let mut border_instances: Vec<f32> = Vec::new();
        let mut shadow_instances: Vec<f32> = Vec::new();
        let mut text_commands: Vec<&PaintCommand> = Vec::new();
        Self::collect_rects_and_text(
            display_list.commands(),
            &mut rect_instances,
            &mut border_instances,
            &mut shadow_instances,
            &mut text_commands,
        );

        // 2. 准备文字实例数据
        let text_instances = self
            .text_renderer
            .prepare_text(&text_commands, &self.queue);

        let rect_count = rect_instances.len() / 8;
        let border_count = border_instances.len() / 16;
        let shadow_count = shadow_instances.len() / 16;
        let text_count = text_instances.len() / 12;

        if rect_instances.is_empty() && border_instances.is_empty() && shadow_instances.is_empty() && text_instances.is_empty() {
            return;
        }

        // 3. 确保实例缓冲足够大
        if rect_count > 0 {
            self.ensure_instance_capacity(rect_count);
        }
        if border_count > 0 {
            self.ensure_border_instance_capacity(border_count);
        }
        if shadow_count > 0 {
            self.ensure_shadow_instance_capacity(shadow_count);
        }
        if text_count > 0 {
            self.ensure_text_instance_capacity(text_count);
        }

        // 4. P1-9: 增量写入实例数据
        // 使用 display_list.generation() 检测 DL 是否变更
        let dl_gen = display_list.generation();
        let gen_changed = dl_gen != self.last_dl_generation;
        let write_rect = rect_count > 0 && (gen_changed || rect_count != self.last_rect_count);
        if write_rect {
            self.queue.write_buffer(
                &self.instance_buf,
                0,
                bytemuck::cast_slice(&rect_instances),
            );
            self.last_rect_count = rect_count;
        }
        let write_border = border_count > 0 && (gen_changed || border_count != self.last_border_count);
        if write_border {
            self.queue.write_buffer(
                &self.border_instance_buf,
                0,
                bytemuck::cast_slice(&border_instances),
            );
            self.last_border_count = border_count;
        }
        let write_shadow = shadow_count > 0 && (gen_changed || shadow_count != self.last_shadow_count);
        if write_shadow {
            self.queue.write_buffer(
                &self.shadow_instance_buf,
                0,
                bytemuck::cast_slice(&shadow_instances),
            );
            self.last_shadow_count = shadow_count;
        }
        let write_text = text_count > 0 && (gen_changed || text_count != self.last_text_count);
        if write_text {
            self.queue.write_buffer(
                &self.text_instance_buf,
                0,
                bytemuck::cast_slice(&text_instances),
            );
            self.last_text_count = text_count;
        }
        self.last_dl_generation = dl_gen;
        // P1-9: 仅当屏幕尺寸变化时更新 uniform
        let screen_size = self.logical_size;
        if screen_size != self.last_screen_size {
            let screen_w = screen_size.0 as f32;
            let screen_h = screen_size.1 as f32;
            self.queue.write_buffer(
                &self.screen_uniform_buf,
                0,
                bytemuck::cast_slice(&[screen_w, screen_h]),
            );
            self.last_screen_size = screen_size;
        }

        // 5. 获取 surface 纹理
        let output = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(e) => {
                eprintln!("wgpu surface error: {e:?}");
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Frame Encoder"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // 6. 绘制阴影（BoxShadow → shadow_pipeline，最先渲染）
            if shadow_count > 0 {
                pass.set_pipeline(&self.shadow_pipeline);
                pass.set_bind_group(0, Some(&self.screen_bind_group), &[]);
                pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                pass.set_vertex_buffer(1, self.shadow_instance_buf.slice(..));
                pass.draw(0..6, 0..shadow_count as u32);
            }

            // 7. 绘制圆角矩形（FillRect → border_pipeline SDF）
            if border_count > 0 {
                pass.set_pipeline(&self.border_pipeline);
                pass.set_bind_group(0, Some(&self.screen_bind_group), &[]);
                pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                pass.set_vertex_buffer(1, self.border_instance_buf.slice(..));
                pass.draw(0..6, 0..border_count as u32);
            }

            // 8. 绘制矩形（Border → rect_pipeline）
            if rect_count > 0 {
                pass.set_pipeline(&self.rect_pipeline);
                pass.set_bind_group(0, Some(&self.screen_bind_group), &[]);
                pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                pass.set_vertex_buffer(1, self.instance_buf.slice(..));
                pass.draw(0..6, 0..rect_count as u32);
            }

            // 9. 绘制文字
            if text_count > 0 {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, Some(&self.text_bind_group), &[]);
                pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                pass.set_vertex_buffer(1, self.text_instance_buf.slice(..));
                pass.draw(0..6, 0..text_count as u32);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.physical_size = (width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn present(&mut self) {}

    fn size(&self) -> (u32, u32) {
        self.logical_size
    }
}

impl WgpuBackend {
    /// 确保实例缓冲至少能容纳 `n` 个矩形
    fn ensure_instance_capacity(&mut self, n: usize) {
        if n <= self.instance_capacity {
            return;
        }
        let new_cap = (n + 255) & !255; // 对齐到 256
        self.instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Data Buffer"),
            size: (new_cap * 8 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_capacity = new_cap;
    }

    /// 确保 border 实例缓冲至少能容纳 `n` 个实例
    fn ensure_border_instance_capacity(&mut self, n: usize) {
        if n <= self.border_instance_capacity {
            return;
        }
        let new_cap = (n + 255) & !255;
        self.border_instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Border Instance Buffer"),
            size: (new_cap * 16 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.border_instance_capacity = new_cap;
    }

    /// 确保 shadow 实例缓冲至少能容纳 `n` 个实例
    fn ensure_shadow_instance_capacity(&mut self, n: usize) {
        if n <= self.shadow_instance_capacity {
            return;
        }
        let new_cap = (n + 255) & !255;
        self.shadow_instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shadow Instance Buffer"),
            size: (new_cap * 16 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.shadow_instance_capacity = new_cap;
    }

    /// 确保文本实例缓冲至少能容纳 `n` 个字形
    fn ensure_text_instance_capacity(&mut self, n: usize) {
        if n <= self.text_instance_capacity {
            return;
        }
        let new_cap = (n + 255) & !255;
        self.text_instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Instance Buffer"),
            size: (new_cap * 12 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.text_instance_capacity = new_cap;
    }

    /// 递归收集 DisplayList 命令：FillRect→border, BoxShadow→shadow, Border→rects, 文本→单独列表
    fn collect_rects_and_text<'a>(
        commands: &'a [PaintCommand],
        out_rects: &mut Vec<f32>,
        out_border: &mut Vec<f32>,
        out_shadow: &mut Vec<f32>,
        out_text: &mut Vec<&'a PaintCommand>,
    ) {
        for cmd in commands {
            match cmd {
                PaintCommand::FillRect { rect, color, radius } => {
                    let r = (*radius).max(0.0);
                    let max_r = rect.width.min(rect.height) / 2.0;
                    let r = r.min(max_r);
                    Self::push_border_instance(
                        out_border,
                        rect.x, rect.y, rect.width, rect.height, r,
                        *color,
                        0.0,
                        Color::TRANSPARENT,
                    );
                }
                PaintCommand::Text { .. } => {
                    out_text.push(cmd);
                }
                PaintCommand::Border { rect, widths, colors, radius, .. } => {
                    let r = (*radius).max(0.0);
                    if r > 0.0 {
                        // 圆角边框 → border_pipeline (SDF 渲染)
                        Self::push_border_instance(
                            out_border,
                            rect.x, rect.y, rect.width, rect.height, r,
                            Color::TRANSPARENT,
                            widths[0],
                            colors[0],
                        );
                    } else {
                        // 直角边框 → rect_pipeline (按 widths 逐边渲染)
                        let (x, y, w, h) = (rect.x, rect.y, rect.width, rect.height);
                        if widths[0] > 0.0 { Self::push_rect(out_rects, x, y, w, widths[0], colors[0]); }
                        if widths[2] > 0.0 { Self::push_rect(out_rects, x, y + h - widths[2], w, widths[2], colors[2]); }
                        if widths[3] > 0.0 { Self::push_rect(out_rects, x, y, widths[3], h, colors[3]); }
                        if widths[1] > 0.0 { Self::push_rect(out_rects, x + w - widths[1], y, widths[1], h, colors[1]); }
                    }
                }
                PaintCommand::BoxShadow { rect, offset_x, offset_y, blur_radius, spread_radius, color, radius, .. } => {
                    let spread = spread_radius.max(0.0);
                    let blur = blur_radius.max(0.0);
                    let r = (*radius + spread).max(0.0);
                    let sx = rect.x + offset_x - spread - blur;
                    let sy = rect.y + offset_y - spread - blur;
                    let sw = rect.width + 2.0 * (spread + blur);
                    let sh = rect.height + 2.0 * (spread + blur);
                    Self::push_shadow_instance(out_shadow, sx, sy, sw, sh, r, blur, *color);
                }
                PaintCommand::Image { .. } => {}
                PaintCommand::Clip { rect, commands } => {
                    Self::collect_rects_and_text_clipped(rect, commands, out_rects, out_border, out_shadow, out_text);
                }
                PaintCommand::Opacity { commands, .. } => {
                    Self::collect_rects_and_text(commands, out_rects, out_border, out_shadow, out_text);
                }
            }
        }
    }

    /// 带裁剪区收集命令：将子命令的几何体裁剪到 clip_rect 内
    fn collect_rects_and_text_clipped<'a>(
        clip_rect: &dom::Rect<f32>,
        commands: &'a [PaintCommand],
        out_rects: &mut Vec<f32>,
        out_border: &mut Vec<f32>,
        out_shadow: &mut Vec<f32>,
        out_text: &mut Vec<&'a PaintCommand>,
    ) {
        for cmd in commands {
            match cmd {
                PaintCommand::FillRect { rect, color, radius } => {
                    if let Some(clipped) = Self::clip_rect_intersection(rect, clip_rect) {
                        let r = (*radius).max(0.0).min(clipped.width.min(clipped.height) / 2.0);
                        Self::push_border_instance(
                            out_border,
                            clipped.x, clipped.y, clipped.width, clipped.height, r,
                            *color, 0.0, Color::TRANSPARENT,
                        );
                    }
                }
                PaintCommand::Text { .. } => {
                    // 文本不支持 GPU 裁剪，跳过
                    // Phase 2+: GPU scissor 裁剪所有类型
                }
                PaintCommand::Border { rect, widths, colors, radius, .. } => {
                    if let Some(clipped) = Self::clip_rect_intersection(rect, clip_rect) {
                        let r = (*radius).max(0.0).min(clipped.width.min(clipped.height) / 2.0);
                        Self::push_border_instance(
                            out_border,
                            clipped.x, clipped.y, clipped.width, clipped.height, r,
                            Color::TRANSPARENT, widths[0], colors[0],
                        );
                    }
                }
                PaintCommand::BoxShadow { .. } => {
                    // 阴影暂不裁剪
                }
                PaintCommand::Image { .. } => {}
                PaintCommand::Clip { rect, commands } => {
                    // 嵌套 Clip：取交集
                    let inner = Self::clip_rect_intersection(rect, clip_rect);
                    if let Some(ref inner_clip) = inner {
                        Self::collect_rects_and_text_clipped(
                            inner_clip, commands, out_rects, out_border, out_shadow, out_text,
                        );
                    }
                }
                PaintCommand::Opacity { commands, .. } => {
                    Self::collect_rects_and_text_clipped(
                        clip_rect, commands, out_rects, out_border, out_shadow, out_text,
                    );
                }
            }
        }
    }

    /// 计算两个 rect 的交集，无交集返回 None
    fn clip_rect_intersection(a: &dom::Rect<f32>, b: &dom::Rect<f32>) -> Option<dom::Rect<f32>> {
        let x = a.x.max(b.x);
        let y = a.y.max(b.y);
        let right = (a.x + a.width).min(b.x + b.width);
        let bottom = (a.y + a.height).min(b.y + b.height);
        if right > x && bottom > y {
            Some(dom::Rect::new(x, y, right - x, bottom - y))
        } else {
            None
        }
    }

    fn push_rect(out: &mut Vec<f32>, x: f32, y: f32, w: f32, h: f32, color: Color) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        out.extend_from_slice(&[
            x, y, w, h,
            crate::srgb_to_linear(color.r as f32 / 255.0),
            crate::srgb_to_linear(color.g as f32 / 255.0),
            crate::srgb_to_linear(color.b as f32 / 255.0),
            color.a as f32 / 255.0,
        ]);
    }

    /// 推送一个圆角矩形实例到 border pipeline（每实例 16 f32 = 4 vec4）
    fn push_border_instance(
        out: &mut Vec<f32>,
        x: f32, y: f32, w: f32, h: f32,
        radius: f32,
        fill_color: Color,
        border_width: f32,
        border_color: Color,
    ) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let fr = crate::srgb_to_linear(fill_color.r as f32 / 255.0);
        let fg = crate::srgb_to_linear(fill_color.g as f32 / 255.0);
        let fb = crate::srgb_to_linear(fill_color.b as f32 / 255.0);
        let fa = fill_color.a as f32 / 255.0;
        let ba = border_color.a as f32 / 255.0;
        let br = crate::srgb_to_linear(border_color.r as f32 / 255.0);
        let bg = crate::srgb_to_linear(border_color.g as f32 / 255.0);
        out.extend_from_slice(&[
            // data0: rect position + size
            x, y, w, h,
            // data1: 4 corner radii
            radius, radius, radius, radius,
            // data2: fill color RGBA
            fr, fg, fb, fa,
            // data3: border_w, border_r, border_g, border_a
            border_width, br, bg, ba,
        ]);
    }

    /// 推送一个阴影实例到 shadow pipeline（每实例 16 f32 = 4 vec4）
    fn push_shadow_instance(
        out: &mut Vec<f32>,
        x: f32, y: f32, w: f32, h: f32,
        radius: f32,
        blur: f32,
        color: Color,
    ) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let r = crate::srgb_to_linear(color.r as f32 / 255.0);
        let g = crate::srgb_to_linear(color.g as f32 / 255.0);
        let b = crate::srgb_to_linear(color.b as f32 / 255.0);
        let a = color.a as f32 / 255.0;
        out.extend_from_slice(&[
            // data0: rect position + size (full extent incl blur)
            x, y, w, h,
            // data1: 4 corner radii
            radius, radius, radius, radius,
            // data2: shadow color
            r, g, b, a,
            // data3: blur_radius, (pad), (pad), (pad)
            blur, 0.0, 0.0, 0.0,
        ]);
    }
}

// Phase 1+: border_pipeline, shadow_pipeline, image_pipeline

// ============================================================
//  Phase 3: 实例化渲染 & 纹理图集
// ============================================================

/// 实例化矩形顶点着色器 —— Phase 3
///
/// 将多个 FillRect 命令合并为一次 instanced draw call。
/// 每个实例有 8 个 f32: x, y, w, h, r, g, b, a
const INSTANCED_RECT_SHADER: &str = r#"
struct InstanceData {
    rect_x: f32,
    rect_y: f32,
    rect_w: f32,
    rect_h: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
};

@group(0) @binding(0) var<uniform> screen: vec2<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) idx: u32,
    @location(0) inst: InstanceData,
) -> VertexOutput {
    let verts = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    let pos = verts[idx];
    let px = inst.rect_x + pos.x * inst.rect_w;
    let py = inst.rect_y + pos.y * inst.rect_h;
    let ndc_x = (px / screen.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / screen.y) * 2.0;

    return VertexOutput(
        vec4<f32>(ndc_x, ndc_y, 0.0, 1.0),
        vec4<f32>(inst.r, inst.g, inst.b, inst.a),
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// 纹理图集 —— Phase 3
///
/// 将多个小纹理打包到一个大纹理中，减少 GPU 状态切换。
pub struct TextureAtlas {
    /// 图集纹理 (Phase 3: 占位，尚未创建实际 GPU 纹理)
    pub width: u32,
    pub height: u32,
    /// 已分配的矩形区域 (x, y, w, h)
    pub regions: Vec<(u32, u32, u32, u32)>,
}

impl TextureAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            regions: Vec::new(),
        }
    }

    /// 分配一个矩形区域（简单行扫描分配器）
    pub fn allocate(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        // Phase 3: 简单的行扫描分配器
        // 返回 (x, y) 坐标
        let mut cursor_x = 0u32;
        let mut cursor_y = 0u32;
        let mut row_height = 0u32;

        for &(rx, ry, rw, rh) in &self.regions {
            if ry > cursor_y {
                cursor_y = ry;
                cursor_x = 0;
                row_height = 0;
            }
        }

        if cursor_x + w <= self.width && cursor_y + h <= self.height {
            let result = (cursor_x, cursor_y);
            self.regions.push((cursor_x, cursor_y, w, h));
            cursor_x += w;
            if h > row_height {
                row_height = h;
            }
            Some(result)
        } else if cursor_y + row_height + h <= self.height {
            // 换行
            cursor_x = 0;
            cursor_y += row_height;
            if cursor_x + w <= self.width && cursor_y + h <= self.height {
                let result = (cursor_x, cursor_y);
                self.regions.push((cursor_x, cursor_y, w, h));
                Some(result)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.regions.clear();
    }
}
