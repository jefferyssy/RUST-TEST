//! CSS 过渡引擎 —— Phase 2
//!
//! 对应 W3C CSS Transitions Level 1 规范。
//! 当 CSS 属性值变更时，在旧值和新值之间平滑过渡。

use std::collections::HashMap;

use crate::values::CSSValue;

/// 过渡属性配置
#[derive(Debug, Clone)]
pub struct TransitionConfig {
    /// 过渡的属性名（"all" 表示所有可过渡属性）
    pub property: String,
    /// 持续时间（秒）
    pub duration: f32,
    /// 延迟时间（秒）
    pub delay: f32,
    /// 缓动函数
    pub timing_function: String,
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            property: "all".to_string(),
            duration: 0.0,
            delay: 0.0,
            timing_function: "ease".to_string(),
        }
    }
}

/// 过渡属性运行时状态
#[derive(Debug, Clone)]
struct TransitionPropertyState {
    /// 属性名
    property: String,
    /// 起始值
    from: CSSValue,
    /// 目标值
    to: CSSValue,
    /// 已运行时间
    elapsed: f32,
    /// 持续时间
    duration: f32,
    /// 延迟
    delay: f32,
}

/// CSS 过渡引擎
pub struct TransitionEngine {
    /// 按元素分组的过渡状态
    transitions: HashMap<String, Vec<TransitionPropertyState>>,
}

impl TransitionEngine {
    pub fn new() -> Self {
        Self {
            transitions: HashMap::new(),
        }
    }

    /// 获取可过渡的属性列表
    pub fn animatable_properties() -> &'static [&'static str] {
        &[
            "width", "height", "min-width", "min-height", "max-width", "max-height",
            "left", "right", "top", "bottom",
            "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
            "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
            "border-width", "border-top-width", "border-right-width",
            "border-bottom-width", "border-left-width",
            "font-size", "line-height",
            "color", "background-color", "border-color",
            "opacity", "transform",
            "border-radius",
            "box-shadow",
            "letter-spacing", "word-spacing",
            "text-indent",
            "z-index",
            "flex-grow", "flex-shrink", "flex-basis",
        ]
    }

    /// 当元素 CSS 属性变更时调用，触发过渡
    pub fn property_changed(
        &mut self,
        element_id: &str,
        property: &str,
        old_value: &CSSValue,
        new_value: &CSSValue,
        config: &TransitionConfig,
    ) {
        // 检查是否匹配过渡属性
        if config.property != "all" && config.property != property {
            return;
        }

        if config.duration <= 0.0 || old_value == new_value {
            return;
        }

        let state = TransitionPropertyState {
            property: property.to_string(),
            from: old_value.clone(),
            to: new_value.clone(),
            elapsed: 0.0,
            duration: config.duration,
            delay: config.delay,
        };

        self.transitions
            .entry(element_id.to_string())
            .or_default()
            .push(state);
    }

    /// 推进所有过渡 delta_time 秒，返回 (元素id → 当前属性值)
    pub fn tick(&mut self, delta_time: f32) -> HashMap<String, HashMap<String, CSSValue>> {
        let mut result = HashMap::new();
        let mut completed = Vec::new();

        for (elem_id, states) in &mut self.transitions {
            let mut elem_props: HashMap<String, CSSValue> = HashMap::new();

            for (i, state) in states.iter_mut().enumerate() {
                state.elapsed += delta_time;

                if state.elapsed < state.delay {
                    // 仍在延迟期，使用旧值
                    elem_props.insert(state.property.clone(), state.from.clone());
                    continue;
                }

                let active_time = state.elapsed - state.delay;

                if active_time >= state.duration {
                    // 过渡完成，使用目标值
                    elem_props.insert(state.property.clone(), state.to.clone());
                    completed.push((elem_id.clone(), i));
                    continue;
                }

                let progress = (active_time / state.duration).clamp(0.0, 1.0);
                let eased_progress = Self::apply_easing(progress, &state.property);

                if let Some(interp) =
                    interpolate_css_value(&state.from, &state.to, eased_progress)
                {
                    elem_props.insert(state.property.clone(), interp);
                }
            }

            result.insert(elem_id.clone(), elem_props);
        }

        // 清理已完成的过渡
        for (elem_id, idx) in completed.iter().rev() {
            if let Some(states) = self.transitions.get_mut(elem_id) {
                if *idx < states.len() {
                    states.remove(*idx);
                }
            }
        }
        // 移除空条目
        self.transitions.retain(|_, v| !v.is_empty());

        result
    }

    /// 应用缓动函数
    fn apply_easing(t: f32, _property: &str) -> f32 {
        // Phase 2: 基础 ease 实现（三次贝塞尔 0.25, 0.1, 0.25, 1.0）
        // Phase 3+: cubic-bezier, steps, etc.
        ease(t)
    }
}

/// CSS ease 缓动函数（cubic-bezier(0.25, 0.1, 0.25, 1.0) 的近似）
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    // 使用标准 ease 曲线的三次多项式近似
    -2.0 * t * t * t + 3.0 * t * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::CSSUnit;

    #[test]
    fn test_ease_start() {
        assert!((ease(0.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_ease_end() {
        assert!((ease(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_ease_mid() {
        let mid = ease(0.5);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn test_ease_clamped() {
        assert!((ease(-0.5) - 0.0).abs() < 0.001);
        assert!((ease(1.5) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_animatable_properties_list() {
        let props = TransitionEngine::animatable_properties();
        assert!(props.contains(&"opacity"));
        assert!(props.contains(&"width"));
        assert!(props.contains(&"color"));
        assert!(props.contains(&"transform"));
    }

    #[test]
    fn test_property_changed_creates_transition() {
        let mut engine = TransitionEngine::new();
        let config = TransitionConfig {
            property: "opacity".to_string(),
            duration: 1.0,
            delay: 0.0,
            timing_function: "ease".to_string(),
        };
        engine.property_changed(
            "elem1",
            "opacity",
            &CSSValue::Number(0.0),
            &CSSValue::Number(1.0),
            &config,
        );
        // tick(0.0) still produces a result at progress=0 (the from value)
        let result = engine.tick(0.0);
        assert!(result.contains_key("elem1"));
        assert_eq!(result["elem1"]["opacity"], CSSValue::Number(0.0));
    }

    #[test]
    fn test_transition_tick_interpolation() {
        let mut engine = TransitionEngine::new();
        let config = TransitionConfig {
            property: "width".to_string(),
            duration: 1.0,
            delay: 0.0,
            timing_function: "ease".to_string(),
        };
        engine.property_changed(
            "box1",
            "width",
            &CSSValue::Length(0.0, CSSUnit::Px),
            &CSSValue::Length(100.0, CSSUnit::Px),
            &config,
        );
        let result = engine.tick(0.5);
        let props = result.get("box1").unwrap();
        assert!(props.contains_key("width"));
        if let CSSValue::Length(v, _) = props["width"] {
            assert!(v > 0.0 && v < 100.0, "Expected interpolated value, got {}", v);
        } else {
            panic!("Expected Length value");
        }
    }

    #[test]
    fn test_transition_completion() {
        let mut engine = TransitionEngine::new();
        let config = TransitionConfig {
            property: "opacity".to_string(),
            duration: 0.5,
            delay: 0.0,
            timing_function: "ease".to_string(),
        };
        engine.property_changed(
            "elem",
            "opacity",
            &CSSValue::Number(1.0),
            &CSSValue::Number(0.0),
            &config,
        );
        let result = engine.tick(1.0); // past duration
        let props = result.get("elem").unwrap();
        assert_eq!(props["opacity"], CSSValue::Number(0.0)); // should reach target
    }

    #[test]
    fn test_transition_delay_period() {
        let mut engine = TransitionEngine::new();
        let config = TransitionConfig {
            property: "opacity".to_string(),
            duration: 1.0,
            delay: 0.5,
            timing_function: "ease".to_string(),
        };
        engine.property_changed(
            "elem",
            "opacity",
            &CSSValue::Number(0.0),
            &CSSValue::Number(1.0),
            &config,
        );
        // During delay, return from value
        let result = engine.tick(0.3);
        let props = result.get("elem").unwrap();
        assert_eq!(props["opacity"], CSSValue::Number(0.0));
    }

    #[test]
    fn test_property_not_in_config_ignored() {
        let mut engine = TransitionEngine::new();
        let config = TransitionConfig {
            property: "opacity".to_string(),
            duration: 1.0,
            delay: 0.0,
            timing_function: "ease".to_string(),
        };
        engine.property_changed(
            "elem",
            "width",
            &CSSValue::Length(0.0, CSSUnit::Px),
            &CSSValue::Length(100.0, CSSUnit::Px),
            &config,
        );
        let result = engine.tick(0.5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_all_property_matches_any() {
        let mut engine = TransitionEngine::new();
        let config = TransitionConfig {
            property: "all".to_string(),
            duration: 1.0,
            delay: 0.0,
            timing_function: "ease".to_string(),
        };
        engine.property_changed(
            "elem",
            "margin-top",
            &CSSValue::Length(10.0, CSSUnit::Px),
            &CSSValue::Length(20.0, CSSUnit::Px),
            &config,
        );
        let result = engine.tick(0.5);
        assert!(result.contains_key("elem"));
    }

    #[test]
    fn test_zero_duration_no_transition() {
        let mut engine = TransitionEngine::new();
        let config = TransitionConfig {
            property: "opacity".to_string(),
            duration: 0.0,
            delay: 0.0,
            timing_function: "ease".to_string(),
        };
        engine.property_changed(
            "elem",
            "opacity",
            &CSSValue::Number(0.0),
            &CSSValue::Number(1.0),
            &config,
        );
        let result = engine.tick(0.5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_same_value_no_transition() {
        let mut engine = TransitionEngine::new();
        let config = TransitionConfig {
            property: "opacity".to_string(),
            duration: 1.0,
            delay: 0.0,
            timing_function: "ease".to_string(),
        };
        engine.property_changed(
            "elem",
            "opacity",
            &CSSValue::Number(0.5),
            &CSSValue::Number(0.5),
            &config,
        );
        let result = engine.tick(0.5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_interpolate_percentage_values() {
        let result = interpolate_css_value(
            &CSSValue::Percentage(0.0),
            &CSSValue::Percentage(100.0),
            0.3,
        );
        assert!(matches!(result, Some(CSSValue::Percentage(v)) if (v - 30.0).abs() < 0.01));
    }

    #[test]
    fn test_interpolate_mismatched_types_returns_none() {
        let result = interpolate_css_value(
            &CSSValue::Number(1.0),
            &CSSValue::Keyword("red".to_string()),
            0.5,
        );
        assert!(result.is_none());
    }
}

/// 在两个 CSS 值之间线性插值
fn interpolate_css_value(from: &CSSValue, to: &CSSValue, t: f32) -> Option<CSSValue> {
    use CSSValue::*;
    match (from, to) {
        (Length(a, ua), Length(b, _)) => Some(Length(a + (b - a) * t, *ua)),
        (Percentage(a), Percentage(b)) => Some(Percentage(a + (b - a) * t)),
        (Number(a), Number(b)) => Some(Number(a + (b - a) * t)),
        _ => None,
    }
}
