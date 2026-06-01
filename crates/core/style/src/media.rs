//! 媒体查询求值引擎 —— Phase 2
//!
//! 对应 W3C Media Queries Level 4 规范。
//! 根据视口/设备特征评估 @media 规则条件。

use crate::stylesheet::{MediaQuery, MediaType, MediaFeature};

/// 视口信息（用于媒体查询求值）
#[derive(Debug, Clone)]
pub struct ViewportInfo {
    /// 视口宽度（像素）
    pub width: f32,
    /// 视口高度（像素）
    pub height: f32,
    /// 设备像素比
    pub device_pixel_ratio: f32,
    /// 是否彩色屏幕
    pub color: bool,
    /// 颜色位数
    pub color_bits: u32,
    /// 屏幕方向："portrait" | "landscape"
    pub orientation: String,
    /// 是否支持指针（鼠标/触控）
    pub pointer: String,
    /// 是否支持 hover
    pub hover: String,
    /// 首选配色方案："light" | "dark"
    pub prefers_color_scheme: String,
    /// 是否减少动画
    pub prefers_reduced_motion: bool,
}

impl Default for ViewportInfo {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            device_pixel_ratio: 1.0,
            color: true,
            color_bits: 24,
            orientation: "landscape".to_string(),
            pointer: "fine".to_string(),
            hover: "hover".to_string(),
            prefers_color_scheme: "light".to_string(),
            prefers_reduced_motion: false,
        }
    }
}

/// 媒体查询求值器
pub struct MediaEvaluator {
    viewport: ViewportInfo,
}

impl MediaEvaluator {
    pub fn new(viewport: ViewportInfo) -> Self {
        Self { viewport }
    }

    /// 更新视口信息
    pub fn update_viewport(&mut self, info: ViewportInfo) {
        self.viewport = info;
    }

    /// 评估媒体查询列表（逗号分隔 = OR 逻辑），任一匹配返回 true
    pub fn evaluate(&self, queries: &[MediaQuery]) -> bool {
        if queries.is_empty() {
            return true; // 无媒体查询 = 始终匹配
        }
        queries.iter().any(|q| self.evaluate_query(q))
    }

    /// 评估单个媒体查询
    fn evaluate_query(&self, query: &MediaQuery) -> bool {
        // 检查媒体类型
        if !self.evaluate_media_type(&query.media_type) {
            return false;
        }

        // 检查媒体特性（AND 逻辑）
        for feature in &query.features {
            if !self.evaluate_feature(feature) {
                return false;
            }
        }

        true
    }

    fn evaluate_media_type(&self, media_type: &MediaType) -> bool {
        match media_type {
            MediaType::All => true,
            MediaType::Screen => true, // 始终是 screen
            MediaType::Print => false,  // Phase 2: 不支持打印
            MediaType::Custom(_) => false,
        }
    }

    fn evaluate_feature(&self, feature: &MediaFeature) -> bool {
        match feature {
            MediaFeature::Width { min, max } => {
                if let Some(min) = min {
                    if self.viewport.width < *min {
                        return false;
                    }
                }
                if let Some(max) = max {
                    if self.viewport.width > *max {
                        return false;
                    }
                }
                true
            }
            MediaFeature::Height { min, max } => {
                if let Some(min) = min {
                    if self.viewport.height < *min {
                        return false;
                    }
                }
                if let Some(max) = max {
                    if self.viewport.height > *max {
                        return false;
                    }
                }
                true
            }
            MediaFeature::Orientation(ref orient) => {
                self.viewport.orientation.eq_ignore_ascii_case(orient)
            }
            MediaFeature::PrefersColorScheme(ref scheme) => {
                self.viewport.prefers_color_scheme.eq_ignore_ascii_case(scheme)
            }
            MediaFeature::PrefersReducedMotion => {
                self.viewport.prefers_reduced_motion
            }
            MediaFeature::AspectRatio { min, max } => {
                let ratio = if self.viewport.height > 0.0 {
                    self.viewport.width / self.viewport.height
                } else {
                    0.0
                };
                if let Some(min) = min {
                    if ratio < *min { return false; }
                }
                if let Some(max) = max {
                    if ratio > *max { return false; }
                }
                true
            }
            MediaFeature::Custom(ref name, ref value) => {
                // Phase 2: 对未知特性返回 true（宽容模式）
                // Phase 3+: 完整特性解析
                let _ = name;
                let _ = value;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_default() {
        let vp = ViewportInfo::default();
        assert_eq!(vp.width, 800.0);
        assert_eq!(vp.height, 600.0);
    }

    #[test]
    fn test_media_all_matches() {
        let eval = MediaEvaluator::new(ViewportInfo::default());
        let query = MediaQuery {
            media_type: MediaType::All,
            features: vec![],
            rules: vec![],
        };
        assert!(eval.evaluate_query(&query));
    }

    #[test]
    fn test_media_width_matches() {
        let eval = MediaEvaluator::new(ViewportInfo::default());
        let query = MediaQuery {
            media_type: MediaType::All,
            features: vec![MediaFeature::Width {
                min: Some(600.0),
                max: Some(1200.0),
            }],
            rules: vec![],
        };
        assert!(eval.evaluate_query(&query));
    }

    #[test]
    fn test_media_width_no_match() {
        let eval = MediaEvaluator::new(ViewportInfo::default());
        let query = MediaQuery {
            media_type: MediaType::All,
            features: vec![MediaFeature::Width {
                min: Some(1000.0),
                max: None,
            }],
            rules: vec![],
        };
        assert!(!eval.evaluate_query(&query));
    }
}
