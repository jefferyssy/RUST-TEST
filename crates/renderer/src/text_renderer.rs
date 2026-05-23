//! 文本渲染器 —— 将文本转为 GPU 纹理并绘制
//!
//! 工作原理：
//! 1. fontdb 查找系统字体
//! 2. rustybuzz 对文本做 shaping → 获取字形 ID 和位置
//! 3. ab_glyph 将字形光栅化为位图
//! 4. 字形位图打包到纹理图集
//! 5. 生成实例数据（每字形一个四边形）供 GPU 管线渲染

use std::collections::HashMap;
use std::sync::Arc;

use ab_glyph::Font;
use dom::Color;
use render_tree::PaintCommand;

use crate::srgb_to_linear;

/// 字形纹理图集中的缓存条目
#[derive(Clone)]
struct GlyphEntry {
    /// 图集中的像素位置
    atlas_x: u32,
    atlas_y: u32,
    /// 字形位图尺寸（像素）
    width: u32,
    height: u32,
    /// 字形的 bearing（用于定位基线）
    bearing_x: f32,
    bearing_y: f32,
    /// 水平步进量
    advance: f32,
}

/// 字形缓存键
#[derive(Hash, Eq, PartialEq)]
struct GlyphKey {
    /// 字体数据哈希（区分不同字体）
    font_hash: u64,
    /// 字形 ID（rustybuzz 返回的 glyph_index）
    glyph_id: u32,
    /// 字号（定点数，精度 1/16 px）
    size_bits: u32,
}

/// 已加载的字体缓存
struct LoadedFont {
    data: Arc<Vec<u8>>,
    face_index: u32,
    hash: u64,
}

/// 文本渲染器
///
/// 负责：
/// - 加载系统字体
/// - 文本塑形（rustybuzz）
/// - 字形光栅化（ab_glyph）
/// - 字形纹理图集管理
/// - 生成 GPU 实例数据
pub struct TextRenderer {
    /// 字体数据库
    font_db: fontdb::Database,
    /// 已加载的字体（family → 字体数据）
    loaded_fonts: HashMap<String, LoadedFont>,
    /// 字形纹理图集
    atlas_texture: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    atlas_sampler: wgpu::Sampler,
    atlas_width: u32,
    atlas_height: u32,
    /// 图集填充状态（简单 shelf 分配器）
    atlas_cursor_x: u32,
    atlas_cursor_y: u32,
    atlas_row_height: u32,
    /// 字形缓存
    glyph_cache: HashMap<GlyphKey, GlyphEntry>,
    /// 暂存纹理数据的缓冲区（CPU 端）
    atlas_buffer: Vec<u8>,
    atlas_dirty: bool,
}

impl TextRenderer {
    pub fn new(device: &wgpu::Device, _format: wgpu::TextureFormat) -> Self {
        let mut font_db = fontdb::Database::new();
        font_db.load_system_fonts();

        let atlas_width = 1024u32;
        let atlas_height = 1024u32;

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Glyph Atlas Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let atlas_buffer = vec![0u8; (atlas_width * atlas_height) as usize];

        Self {
            font_db,
            loaded_fonts: HashMap::new(),
            atlas_texture,
            atlas_view,
            atlas_sampler,
            atlas_width,
            atlas_height,
            atlas_cursor_x: 0,
            atlas_cursor_y: 0,
            atlas_row_height: 0,
            glyph_cache: HashMap::new(),
            atlas_buffer,
            atlas_dirty: false,
        }
    }

    /// 获取图集纹理视图（用于创建文本管线的 bind group）
    pub fn atlas_texture_view(&self) -> &wgpu::TextureView {
        &self.atlas_view
    }

    /// 获取图集采样器（用于创建文本管线的 bind group）
    pub fn atlas_sampler(&self) -> &wgpu::Sampler {
        &self.atlas_sampler
    }

    /// 准备文本命令：塑形 + 光栅化，返回每个字形的实例数据
    ///
    /// 实例数据格式（每字形 12 f32 = 48 bytes）：
    ///   data0: dst_x, dst_y, dst_w, dst_h   (屏幕空间目标矩形)
    ///   data1: src_u, src_v, src_w, src_h   (图集 UV 坐标，归一化 0..1)
    ///   data2: r, g, b, a                   (文字颜色)
    pub fn prepare_text(
        &mut self,
        commands: &[&PaintCommand],
        queue: &wgpu::Queue,
    ) -> Vec<f32> {
        let mut instances: Vec<f32> = Vec::new();

        for cmd in commands {
            if let PaintCommand::Text {
                text,
                font_size,
                font_family,
                font_weight,
                x,
                y,
                color,
                ..
            } = cmd
            {
                if text.is_empty() {
                    continue;
                }
                self.shape_and_collect(
                    text,
                    *font_size,
                    font_family,
                    *font_weight,
                    *x,
                    *y,
                    color,
                    &mut instances,
                );
            }
        }

        // 如果图集有更新，上传脏数据
        if self.atlas_dirty {
            self.upload_atlas(queue);
        }

        instances
    }

    /// 塑形文本并生成字形实例数据
    fn shape_and_collect(
        &mut self,
        text: &str,
        font_size: f32,
        font_family: &str,
        font_weight: u16,
        x: f32,
        y: f32,
        color: &Color,
        instances: &mut Vec<f32>,
    ) {
        let atlas_w = self.atlas_width as f32;
        let atlas_h = self.atlas_height as f32;
        let scale = ab_glyph::PxScale::from(font_size);

        // 加载字体并提取所需数据（在块内释放 self 上的借用）
        let (font_data, font_hash, face_index) = {
            let key = format!("{}-{}", font_family, font_weight);
            self.ensure_font(font_family, font_weight);
            let font = self
                .loaded_fonts
                .get(&key)
                .or_else(|| self.loaded_fonts.get("__fallback"));
            match font {
                Some(f) => (f.data.clone(), f.hash, f.face_index),
                None => return,
            }
        };

        // rustybuzz 塑形
        let rb_face = match rustybuzz::Face::from_slice(&font_data, face_index) {
            Some(face) => face,
            None => return,
        };
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        let shaped = rustybuzz::shape(&rb_face, &[], buffer);
        let glyph_infos = shaped.glyph_infos();
        let glyph_positions = shaped.glyph_positions();

        // ab_glyph 字体引用
        let ag_font = match ab_glyph::FontRef::try_from_slice(&font_data) {
            Ok(f) => f,
            Err(_) => return,
        };

        let scale_factor = font_size / ag_font.height_unscaled();
        // y 是 em square 顶部（已在 builder 中加过 half-leading）
        // 基线 = em square 顶部 + font ascent
        let baseline_y = y + ag_font.ascent_unscaled() * scale_factor;

        let mut cursor_x = x;
        let r = srgb_to_linear(color.r as f32 / 255.0);
        let g = srgb_to_linear(color.g as f32 / 255.0);
        let b = srgb_to_linear(color.b as f32 / 255.0);
        let a = color.a as f32 / 255.0;

        for (info, pos) in glyph_infos.iter().zip(glyph_positions.iter()) {
            let glyph_id = info.glyph_id;
            let size_bits = (font_size * 16.0) as u32;
            let key = GlyphKey {
                font_hash,
                glyph_id,
                size_bits,
            };

            let glyph = self.ensure_glyph(&ag_font, &key, glyph_id, scale);

            // baseline_y 是基线位置，bearing_y = 基线到字形顶部的距离
            let gx = cursor_x
                + pos.x_offset as f32 * scale_factor
                + glyph.bearing_x;
            let gy = baseline_y + pos.y_offset as f32 * scale_factor
                - glyph.bearing_y;

            if glyph.width > 0 && glyph.height > 0 {
                instances.extend_from_slice(&[
                    gx,
                    gy,
                    glyph.width as f32,
                    glyph.height as f32,
                    glyph.atlas_x as f32 / atlas_w,
                    glyph.atlas_y as f32 / atlas_h,
                    glyph.width as f32 / atlas_w,
                    glyph.height as f32 / atlas_h,
                    r,
                    g,
                    b,
                    a,
                ]);
            }

            cursor_x += pos.x_advance as f32 * scale_factor;
        }
    }

    /// 确保字形在缓存中，如果缺失则光栅化并添加到图集
    fn ensure_glyph(
        &mut self,
        ag_font: &ab_glyph::FontRef,
        key: &GlyphKey,
        glyph_id: u32,
        scale: ab_glyph::PxScale,
    ) -> GlyphEntry {
        if let Some(entry) = self.glyph_cache.get(key) {
            return entry.clone();
        }

        let ab_glyph_id = ab_glyph::GlyphId(glyph_id as u16);
        let glyph = ab_glyph_id.with_scale(scale);

        let entry = match ag_font.outline_glyph(glyph) {
            Some(outlined) => {
                let bounds = outlined.px_bounds();
                let gw = bounds.width().ceil() as u32;
                let gh = bounds.height().ceil() as u32;
                let bearing_x = bounds.min.x;
                let bearing_y = -bounds.min.y; // ab_glyph Y-down: 基线到字形顶部 = -min.y
                let advance = ag_font.h_advance_unscaled(ab_glyph_id) * scale.x
                    / ag_font.height_unscaled();

                if gw == 0 || gh == 0 {
                    GlyphEntry {
                        atlas_x: 0,
                        atlas_y: 0,
                        width: 0,
                        height: 0,
                        bearing_x,
                        bearing_y,
                        advance,
                    }
                } else {
                    let (ax, ay) = self.allocate_atlas_region(gw, gh);
                    // 光栅化到 CPU 缓冲区
                    outlined.draw(|px, py, coverage| {
                        let bx = ax as usize + px as usize;
                        let by = ay as usize + py as usize;
                        if bx < self.atlas_width as usize
                            && by < self.atlas_height as usize
                        {
                            let idx = by * self.atlas_width as usize + bx;
                            self.atlas_buffer[idx] =
                                (coverage * 255.0).round() as u8;
                        }
                    });
                    self.atlas_dirty = true;
                    GlyphEntry {
                        atlas_x: ax,
                        atlas_y: ay,
                        width: gw,
                        height: gh,
                        bearing_x,
                        bearing_y,
                        advance,
                    }
                }
            }
            None => GlyphEntry {
                atlas_x: 0,
                atlas_y: 0,
                width: 0,
                height: 0,
                bearing_x: 0.0,
                bearing_y: 0.0,
                advance: ag_font.h_advance_unscaled(ab_glyph_id) * scale.x
                    / ag_font.height_unscaled(),
            },
        };

        self.glyph_cache.insert(
            GlyphKey {
                font_hash: key.font_hash,
                glyph_id: key.glyph_id,
                size_bits: key.size_bits,
            },
            entry.clone(),
        );

        entry
    }

    /// 在图集中分配矩形区域（简单 shelf 算法）
    fn allocate_atlas_region(&mut self, w: u32, h: u32) -> (u32, u32) {
        // 检查当前行是否有空间
        let space_right = self.atlas_width.saturating_sub(self.atlas_cursor_x);
        if w > space_right {
            self.atlas_cursor_x = 0;
            self.atlas_cursor_y += self.atlas_row_height;
            self.atlas_row_height = 0;
        }

        // 如果超出底部，回绕并清空图集
        if self.atlas_cursor_y + h > self.atlas_height {
            self.atlas_cursor_x = 0;
            self.atlas_cursor_y = 0;
            self.atlas_row_height = 0;
            self.glyph_cache.clear();
            self.atlas_buffer.fill(0);
            self.atlas_dirty = true;
        }

        let result = (self.atlas_cursor_x, self.atlas_cursor_y);
        self.atlas_cursor_x += w;
        if h > self.atlas_row_height {
            self.atlas_row_height = h;
        }
        result
    }

    /// 将脏图集数据上传到 GPU
    fn upload_atlas(&mut self, queue: &wgpu::Queue) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.atlas_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.atlas_width),
                rows_per_image: Some(self.atlas_height),
            },
            wgpu::Extent3d {
                width: self.atlas_width,
                height: self.atlas_height,
                depth_or_array_layers: 1,
            },
        );
        self.atlas_dirty = false;
    }

    /// 确保字体已加载（按 family + weight）
    fn ensure_font(&mut self, family: &str, weight: u16) {
        let key = format!("{}-{}", family, weight);
        if self.loaded_fonts.contains_key(&key) {
            return;
        }

        // 将 CSS 泛型字体族名映射到 fontdb::Family 枚举
        let fontdb_family = match family {
            "serif" => fontdb::Family::Serif,
            "sans-serif" => fontdb::Family::SansSerif,
            "monospace" => fontdb::Family::Monospace,
            "cursive" => fontdb::Family::Cursive,
            "fantasy" => fontdb::Family::Fantasy,
            _ => fontdb::Family::Name(family),
        };

        let query = self.font_db.query(&fontdb::Query {
            families: &[fontdb_family],
            weight: fontdb::Weight(weight),
            ..Default::default()
        });

        if let Some(face_id) = query {
            if let Some(face_info) = self.font_db.face(face_id) {
                let path: Option<std::path::PathBuf> = match &face_info.source {
                    fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => {
                        Some(p.clone())
                    }
                    _ => None,
                };
                if let Some(ref path) = path {
                    self.load_font_from_path(&key, path, face_info.index);
                }
            }
        }

        // 回退：使用任意系统字体
        if !self.loaded_fonts.contains_key("__fallback") {
            let fallback = self.font_db.query(&fontdb::Query::default());
            if let Some(face_id) = fallback {
                if let Some(face_info) = self.font_db.face(face_id) {
                    let path: Option<std::path::PathBuf> = match &face_info.source {
                        fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => {
                            Some(p.clone())
                        }
                        _ => None,
                    };
                    if let Some(ref path) = path {
                        self.load_font_from_path("__fallback", path, face_info.index);
                    }
                }
            }
        }
    }

    fn load_font_from_path(
        &mut self,
        key: &str,
        path: &std::path::Path,
        index: u32,
    ) {
        let data = match std::fs::read(path) {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };
        let hash = Self::hash_bytes(&data);

        self.loaded_fonts.insert(
            key.to_string(),
            LoadedFont {
                data,
                face_index: index,
                hash,
            },
        );
    }

    /// 简单字节哈希
    fn hash_bytes(data: &[u8]) -> u64 {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash_slice(data, &mut hasher);
        hasher.finish()
    }
}
