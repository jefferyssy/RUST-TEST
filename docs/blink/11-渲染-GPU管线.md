# 11 — 渲染 GPU 管线：wgpu + WGSL vs Skia + Viz 多进程

## 问题/目标

浏览器引擎的最末端是将绘制命令转换为屏幕像素。这涉及：
- 跨平台图形 API 抽象（DirectX / Vulkan / Metal / OpenGL）
- 着色器编写与编译
- 纹理管理与字形渲染
- 帧缓冲管理与呈现（vsync、triple buffering）
- 多进程安全隔离（渲染进程崩溃不影响浏览器主进程）

## 我们的实现

### 技术栈：wgpu + winit + WGSL

来源：[crates/renderer/src/wgpu_backend.rs](../../crates/renderer/src/wgpu_backend.rs)

```
winit (窗口 + 事件) → wgpu (GPU 抽象) → DX12 / Vulkan / Metal / WebGPU
                                                   ↓
                                            GPU 驱动 → 屏幕
```

**跨平台覆盖**：
- Windows → DX12（默认）或 Vulkan
- macOS → Metal
- Linux → Vulkan 或 OpenGL
- Web → WebGPU（编译为 wasm）

### 初始化流程

```rust
impl WgpuBackend {
    pub async fn new(window: &winit::window::Window) -> Self {
        // 1. 创建 wgpu 实例
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        // 2. 创建 surface（从 winit 窗口获取原生窗口句柄）
        let surface = instance.create_surface(window).unwrap();

        // 3. 请求 GPU 适配器（高性能优先）
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
        }).await.unwrap();

        // 4. 请求设备 + 队列
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::PUSH_CONSTANTS,
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            }, None
        ).await.unwrap();

        // 5. 配置 surface（格式、呈现模式、alpha 模式）
        let config = wgpu::SurfaceConfiguration {
            format: caps.formats[0],
            present_mode: wgpu::PresentMode::Fifo,     // ★ vsync
            alpha_mode: caps.alpha_modes[0],
            ...
        };
        surface.configure(&device, &config);

        // 6. 编译 4 个着色器 → 创建 4 个渲染管线
        let rect_pipeline = create_pipeline(&device, RECT_VERTEX_SHADER, ...);
        let border_pipeline = create_pipeline(&device, BORDER_VERTEX_SHADER, ...);
        let shadow_pipeline = create_pipeline(&device, SHADOW_VERTEX_SHADER, ...);
        let text_pipeline = create_pipeline(&device, TEXT_VERTEX_SHADER, ...);
        // ...
    }
}
```

### 渲染一帧

```rust
fn render(&mut self, display_list: &DisplayList) {
    // 1. 收集实例数据
    let (rect_instances, border_instances, shadow_instances, text_instances)
        = collect_from_display_list(display_list);

    // 2. 写入 GPU 缓冲
    queue.write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(&rect_instances));
    // ... 其他缓冲

    // 3. 获取 surface 纹理
    let output = self.surface.get_current_texture()?;
    let view = output.texture.create_view(&TextureViewDescriptor::default());

    // 4. 单 render pass，4 个 instanced draw call
    let mut encoder = device.create_command_encoder(...);
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &view,
            ops: Operations { load: LoadOp::Clear(WHITE), store: StoreOp::Store },
        })],
    });
    pass.set_pipeline(&self.shadow_pipeline);
    pass.draw(0..6, 0..shadow_count);   // 所有阴影
    pass.set_pipeline(&self.border_pipeline);
    pass.draw(0..6, 0..border_count);   // 所有圆角矩形
    pass.set_pipeline(&self.rect_pipeline);
    pass.draw(0..6, 0..rect_count);     // 所有边框
    pass.set_pipeline(&self.text_pipeline);
    pass.draw(0..6, 0..text_count);     // 所有文本
    drop(pass);

    self.queue.submit(Some(encoder.finish()));
    output.present();  // ★ 显示帧
}
```

### 着色器架构（WGSL）

| Shader | 顶点输入 | 实例输入 | 片段输出 |
|--------|---------|---------|---------|
| RECT | `vec2 pos` | `vec4 rect + vec4 color` (8 f32) | 纯色填充 |
| BORDER (SDF) | `vec2 pos` | `vec4 rect + vec4 radii + vec4 fill + vec4 border` (16 f32) | SDF 圆角矩形 + 边框 |
| SHADOW (SDF) | `vec2 pos` | `vec4 rect + vec4 radii + vec4 color + vec4 blur` (16 f32) | SDF + 高斯模糊衰减 |
| TEXT | `vec2 pos` | `vec4 dst + vec4 src_uv + vec4 color` (12 f32) | 字形图集纹理采样 |

**SDF 圆角矩形着色器核心**：

```wgpu
fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
    let cr = clamp(r, 0.0, min(half_size.x, half_size.y));
    let q = abs(p) - half_size + cr;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - cr;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let d_outer = rounded_rect_sdf(p, in.half_size, r_outer);
    if border_w > 0.0 {
        let d_inner = rounded_rect_sdf(p, border_half, r_inner);
        // border_t = outer内 ∩ inner外; fill_t = inner内
        // 最终 = fill * fill_alpha + border * border_alpha
    } else {
        // 仅填充：alpha = smoothstep(-aa, 0, d)
    }
}
```

### 字形渲染

来源：[crates/renderer/src/text_renderer.rs](../../crates/renderer/src/text_renderer.rs)

```rust
// TextRenderer 使用 ab_glyph 将字形光栅化到纹理图集
pub struct TextRenderer {
    atlas_texture: wgpu::Texture,    // 字形图集（所有字形打包在一个纹理中）
    atlas_sampler: wgpu::Sampler,
    glyph_cache: HashMap<GlyphKey, GlyphInfo>,
}

impl TextRenderer {
    pub fn prepare_text(&mut self, commands: &[&PaintCommand], queue: &Queue) -> Vec<f32> {
        // 1. 对每个 Text 命令：
        //    a. 解析 font family + size + weight → ab_glyph::Font
        //    b. 对每个字符：查找/渲染字形到图集纹理
        //    c. 生成实例数据：(dst_x, dst_y, dst_w, dst_h, src_u, src_v, src_w, src_h, r, g, b, a)
        // 2. 返回打包的 12 f32/字形的实例数据
    }
}
```

## Blink 的实现

### 技术栈：Skia + cc + Viz + ANGLE

```
Skia (2D 图形库) → 光栅化到 tile
  ↓
cc (Chromium Compositor) → 合成器帧管理
  ↓
Viz (Visuals) → 多进程合成 + 显示
  ↓
ANGLE / Metal / Vulkan → GPU 驱动 → 屏幕
```

### Skia：跨平台 2D 图形

```cpp
// Skia 是 Google 自研的 2D 图形库，Blink 的所有绘制通过 Skia Canvas
class GraphicsContext {
    SkCanvas* Canvas() { return canvas_; }

    void FillRect(const gfx::Rect& rect, const Color& color, SkBlendMode mode) {
        SkPaint paint;
        paint.setColor(SkColor4f(color));
        paint.setBlendMode(mode);
        canvas_->drawRect(ToSkRect(rect), paint);
    }

    void DrawText(const String& text, const gfx::PointF& point,
                  const Font& font, const Color& color) {
        SkFont sk_font = font.PrimaryFont()->ToSkFont();
        canvas_->drawString(text, point.x(), point.y(), sk_font, paint);
    }

    void DrawRectWithBorderRadius(const gfx::Rect& rect, const Color& color,
                                   const FloatRoundedRect& radii) {
        SkRRect rrect = ToSkRRect(radii);
        canvas_->drawRRect(rrect, paint);
    }
};
```

### DisplayItem → SkPicture 录制

```cpp
// Blink 的 DisplayItem 可以录制为 SkPicture（可缓存的绘制命令序列）
class DrawingDisplayItem : public DisplayItem {
    PaintRecord paint_record_;  // ★ SkPicture 录制的内容

    // 从 DisplayItem 生成 SkPicture
    void RasterToSkPicture(GraphicsContext& context) {
        SkPictureRecorder recorder;
        SkCanvas* canvas = recorder.beginRecording(visual_rect_);
        context.Canvas() = canvas;
        Draw(context);  // 虚函数：各子类实现自己的绘制
        paint_record_ = recorder.finishRecordingAsPicture();
    }
};

// 缓存：SkPicture 可跨帧复用
// 如果 DisplayItem 未变更 → 直接从 SkPicture 播放，跳过重新录制
void PaintController::PlaybackCachedItem(const DisplayItem& item) {
    SkPicture* cached = item.GetPicture();  // ← 从缓存取出
    canvas_->drawPicture(cached);           // ← 播放（极快）
}
```

### cc::Layer 树 + Tile 管理

```cpp
// cc 层树：每个 Layer 持有 DisplayItemList
class PictureLayer : public Layer {
    scoped_refptr<DisplayItemList> display_list_;

    // 在合成器线程调用
    void Update() override {
        // 1. 计算脏 rect
        gfx::Rect dirty = layer_tree_impl()->UpdateRect();

        // 2. 更新 tile 优先级
        tile_manager_->UpdateTilePriorities();

        // 3. 异步光栅化脏 tile
        for (Tile* tile : tiles_) {
            if (tile->NeedsRaster()) {
                tile_worker_pool_->PostTask([tile, display_list_]() {
                    // 为每个 tile 创建 SkCanvas → 播放 DisplayItemList
                    SkBitmap bitmap;
                    bitmap.allocPixels(tile->size());
                    SkCanvas canvas(bitmap);
                    display_list_->Raster(&canvas, tile->content_rect());
                    tile->SetBitmap(bitmap);
                });
            }
        }
    }
};
```

### Viz：多进程合成

```cpp
// Viz 进程独立于 Renderer 进程
// 职责：接收多个渲染进程的 CompositorFrame → 合成 → 显示

class FrameSinkManager {
    // 聚合多个 CompositorFrame
    void AggregateSurfaces(SurfaceId root_surface) {
        CompositorFrame aggregate;
        for (const auto& [id, surface] : active_surfaces_) {
            aggregate.Merge(surface->GetCompositorFrame());
        }
        // 提交到显示
        display_->DrawAndSwap(aggregate);
    }
};

// 多进程安全隔离：
// - 渲染进程崩溃 → Viz 显示崩溃页面占位符
// - iframe 跨域隔离 → 独立渲染进程 + Viz 合成
// - GPU 崩溃 → Viz 可软件光栅化降级
```

### ANGLE：跨平台 OpenGL ES → Native API

```cpp
// ANGLE 将 OpenGL ES 调用翻译为原生图形 API
// Blink 的 WebGL 实现通过 ANGLE 完成
// 路径：WebGL API → OpenGL ES → ANGLE → DX11/Vulkan/Metal

// ANGLE 的使用方式对 Blink 透明
// 但为 WebGL 内容提供硬件加速
```

## 优劣势分析

| 维度 | 我们 (wgpu + WGSL) | Blink (Skia + cc + Viz) |
|------|--------------------|------------------------|
| **图形 API** | wgpu（自动映射到 DX12/Vulkan/Metal） | Skia（抽象层）+ ANGLE（GL→Native） |
| **着色器语言** | WGSL（WebGPU 标准） | SkSL（Skia Shader Language）→ GLSL/HLSL/MSL |
| **光栅化** | 单帧全量（实例化 draw） | 分 tile 异步光栅化 + SkPicture 缓存 |
| **进程模型** | 单进程，同步渲染 | 多进程（Renderer → Viz → GPU） |
| **帧缓存** | `surface.get_current_texture()` + `present()` | Triple buffering + CompositorFrame |
| **vsync** | `PresentMode::Fifo`（由 wgpu 处理） | `BeginFrameArgs` + `DisplayScheduler` |
| **字形渲染** | ab_glyph → 字形图集纹理 + WGSL 采样 | HarfBuzz + FreeType → SkFont → Skia 位图 |
| **2D API 等价** | PaintCommand 枚举 + 自研 shader | SkCanvas API（完整 Skia 2D 功能） |
| **崩溃隔离** | 无（进程崩溃 = 应用关闭） | GPU 崩溃降级 → 软件光栅化 |

### 我们的优势

1. **wgpu 跨平台自动**：一次代码，DX12/Vulkan/Metal/WebGPU 全覆盖
2. **WGSL 编译时检查**：着色器语法在 wgpu 初始化时验证，不会运行时崩溃
3. **单进程简化**：无 IPC 开销，无多进程协调逻辑
4. **SDF 圆角 + 阴影内嵌 shader**：无需离屏渲染或多 pass，高效
5. **实例化渲染 draw call 极少**：固定 4 个 instanced draw，GPU 效率高
6. **~1000 行着色器 + ~600 行渲染逻辑**：代码量少，逻辑清晰

### 我们的劣势

1. **无 tile 光栅化**：大页面（10000+ 元素）时单帧全量渲染压力大
2. **无 Skia 生态**：缺少 Skia 的颜色管理、抗锯齿、路径渲染、渐变等成熟实现
3. **字形图集每次重建**：prepare_text 在每次 render 时重建所有字形实例数据
4. **单进程无隔离**：GPU 崩溃 = 应用崩溃
5. **无 WebGL/Canvas 支持基础**：自研 shader 体系无法直接支持 `<canvas>` / WebGL
6. **PresentMode 仅 Fifo**：无 Immediate/Mailbox 支持，帧率严格绑定 vsync

### Blink 的优势

1. **Skia 生态完整**：2D 图形功能极全面（路径、渐变、图片解码、色彩空间...）
2. **tile 光栅化**：大页面只渲染可见 tile，内存和 GPU 压力可控
3. **SkPicture 缓存**：未变更内容完全跳过光栅化
4. **多进程安全**：渲染进程崩溃 ≠ 浏览器崩溃
5. **WebGL/Canvas 2D 原生支持**：Skia + ANGLE 完整支持 Web 图形 API

### Blink 的劣势

1. **代码量巨大**：Skia 100 万+ 行，cc 50 万+ 行，viz 20 万+ 行
2. **GPU 兼容性负担**：需处理数百种 GPU/驱动组合的 bug
3. **着色器编译运行时**：GLSL → ANGLE → Native shader 的转译链路
4. **进程 IPC 开销**：每次 Commit 都经过序列化/反序列化

## 改进方向

### 短期

1. **字形缓存持久化**：`prepare_text` 仅处理变更的 Text 命令，复用未变更的字形实例数据
2. **帧缓冲管理优化**：在 resize 时智能保留纹理内容或提前丢弃

### 中期

3. **简单 tile 化**：将视口分为 4×4 tile，对每 tile 内的命令做视口剔除
4. **PresentMode 选项**：支持 Immediate 模式用于无撕裂需求的场景（如动画）

### 长期

5. **Skia 集成**：用 `skia_safe` crate（Rust 绑定）替换自研 shader，获得完整 2D 图形能力
6. **COSMIC 合成器**：参考 COSMIC compositor 的 smithay 实现多进程合成
7. **WebGPU Compute Shader**：利用 wgpu compute shader 加速布局计算（GPU 布局引擎）
