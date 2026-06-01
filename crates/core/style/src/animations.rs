//! CSS 动画引擎 —— Phase 2
//!
//! 对应 W3C CSS Animations Level 1 规范。
//! 支持 @keyframes 规则解析、关键帧插值、动画时间线管理。

use std::collections::HashMap;

use crate::values::CSSValue;
use crate::stylesheet::KeyframesRule;

/// 动画播放状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationPlayState {
    Running,
    Paused,
}

/// 动画填充模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationFillMode {
    None,
    Forwards,
    Backwards,
    Both,
}

/// 动画方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

/// 单个动画实例的运行时状态
#[derive(Debug, Clone)]
pub struct AnimationState {
    /// 动画名称（对应 @keyframes 名）
    pub name: String,
    /// 持续时间（秒）
    pub duration: f32,
    /// 延迟时间（秒）
    pub delay: f32,
    /// 迭代次数（f32::INFINITY = 无限）
    pub iteration_count: f32,
    /// 当前已运行时间（秒）
    pub current_time: f32,
    /// 播放状态
    pub play_state: AnimationPlayState,
    /// 填充模式
    pub fill_mode: AnimationFillMode,
    /// 方向
    pub direction: AnimationDirection,
    /// 缓动函数名
    pub timing_function: String,
    /// 当前迭代次数
    pub current_iteration: u32,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            name: String::new(),
            duration: 0.0,
            delay: 0.0,
            iteration_count: 1.0,
            current_time: 0.0,
            play_state: AnimationPlayState::Running,
            fill_mode: AnimationFillMode::None,
            direction: AnimationDirection::Normal,
            timing_function: "ease".to_string(),
            current_iteration: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::CSSUnit;

    fn make_keyframes(name: &str, percents: Vec<(f32, Vec<(&str, CSSValue)>)>) -> KeyframesRule {
        use crate::stylesheet::{Keyframe, KeyframeSelector};
        KeyframesRule {
            name: name.to_string(),
            keyframes: percents
                .into_iter()
                .map(|(pct, props)| Keyframe {
                    selector: KeyframeSelector::Percent(pct * 100.0),
                    declarations: vec![],
                    properties: props.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_animation_engine_new() {
        let engine = AnimationEngine::new();
        assert!(engine.get_animation_state("test").is_none());
    }

    #[test]
    fn test_register_keyframes() {
        let mut engine = AnimationEngine::new();
        let kf = make_keyframes("fade", vec![
            (0.0, vec![("opacity", CSSValue::Number(0.0))]),
            (1.0, vec![("opacity", CSSValue::Number(1.0))]),
        ]);
        engine.register_keyframes(kf);
        assert!(engine.keyframes.contains_key("fade"));
    }

    #[test]
    fn test_start_and_tick_animation() {
        let mut engine = AnimationEngine::new();
        let kf = make_keyframes("fade", vec![
            (0.0, vec![("opacity", CSSValue::Number(0.0))]),
            (1.0, vec![("opacity", CSSValue::Number(1.0))]),
        ]);
        engine.register_keyframes(kf);

        let anim = AnimationState {
            name: "fade".to_string(),
            duration: 1.0,
            delay: 0.0,
            iteration_count: 1.0,
            ..Default::default()
        };
        engine.start_animation("elem1", anim);

        let result = engine.tick(0.5);
        let props = result.get("elem1").unwrap();
        assert!(props.contains_key("opacity"));
        if let CSSValue::Number(v) = props["opacity"] {
            assert!((v - 0.5).abs() < 0.01, "Expected ~0.5, got {}", v);
        } else {
            panic!("Expected Number value");
        }
    }

    #[test]
    fn test_interpolate_length_values() {
        let result = AnimationEngine::interpolate_value(
            &CSSValue::Length(0.0, CSSUnit::Px),
            &CSSValue::Length(100.0, CSSUnit::Px),
            0.5,
        );
        assert!(matches!(result, Some(CSSValue::Length(v, _)) if (v - 50.0).abs() < 0.01));
    }

    #[test]
    fn test_interpolate_percentage_values() {
        let result = AnimationEngine::interpolate_value(
            &CSSValue::Percentage(0.0),
            &CSSValue::Percentage(100.0),
            0.3,
        );
        assert!(matches!(result, Some(CSSValue::Percentage(v)) if (v - 30.0).abs() < 0.01));
    }

    #[test]
    fn test_interpolate_number_values() {
        let result = AnimationEngine::interpolate_value(
            &CSSValue::Number(10.0),
            &CSSValue::Number(20.0),
            0.25,
        );
        assert!(matches!(result, Some(CSSValue::Number(v)) if (v - 12.5).abs() < 0.01));
    }

    #[test]
    fn test_cancel_animation() {
        let mut engine = AnimationEngine::new();
        let anim = AnimationState {
            name: "test".to_string(),
            ..Default::default()
        };
        engine.start_animation("e", anim);
        engine.cancel_animation("e", "test");
        assert_eq!(engine.get_animation_state("e").unwrap().len(), 0);
    }

    #[test]
    fn test_animation_iteration_count_limit() {
        let mut engine = AnimationEngine::new();
        let kf = make_keyframes("blink", vec![
            (0.0, vec![("opacity", CSSValue::Number(1.0))]),
            (1.0, vec![("opacity", CSSValue::Number(0.0))]),
        ]);
        engine.register_keyframes(kf);

        let anim = AnimationState {
            name: "blink".to_string(),
            duration: 0.5,
            delay: 0.0,
            iteration_count: 2.0,
            ..Default::default()
        };
        engine.start_animation("e", anim);

        engine.tick(0.5); // first iteration
        engine.tick(0.5); // second iteration — should pause
        let state = engine.get_animation_state("e").unwrap();
        assert_eq!(state[0].play_state, AnimationPlayState::Paused);
    }

    #[test]
    fn test_single_keyframe_uses_direct_value() {
        let kf = make_keyframes("solid", vec![
            (0.0, vec![("color", CSSValue::Keyword("red".into()))]),
        ]);
        let result = AnimationEngine::interpolate_keyframes(&kf, 0.5);
        assert_eq!(result.get("color"), Some(&CSSValue::Keyword("red".into())));
    }
}

/// CSS 动画引擎
pub struct AnimationEngine {
    /// 注册的 @keyframes 规则（名称 → 规则）
    keyframes: HashMap<String, KeyframesRule>,
    /// 活跃的动画实例（元素标识 → 动画列表）
    active_animations: HashMap<String, Vec<AnimationState>>,
}

impl AnimationEngine {
    pub fn new() -> Self {
        Self {
            keyframes: HashMap::new(),
            active_animations: HashMap::new(),
        }
    }

    /// 注册 @keyframes 规则
    pub fn register_keyframes(&mut self, rule: KeyframesRule) {
        self.keyframes.insert(rule.name.clone(), rule);
    }

    /// 为元素启动动画
    pub fn start_animation(&mut self, element_id: &str, anim: AnimationState) {
        self.active_animations
            .entry(element_id.to_string())
            .or_default()
            .push(anim);
    }

    /// 推进所有动画 delta_time 秒，返回 (元素id → 动画属性值)
    pub fn tick(&mut self, delta_time: f32) -> HashMap<String, HashMap<String, CSSValue>> {
        let mut result = HashMap::new();

        for (elem_id, animations) in &mut self.active_animations {
            let mut elem_props: HashMap<String, CSSValue> = HashMap::new();

            for anim in animations.iter_mut() {
                if anim.play_state != AnimationPlayState::Running {
                    continue;
                }

                anim.current_time += delta_time;

                // 检查迭代
                if anim.current_time >= anim.duration + anim.delay {
                    anim.current_iteration += 1;
                    if anim.current_iteration as f32 >= anim.iteration_count
                        && anim.iteration_count != f32::INFINITY
                    {
                        anim.play_state = AnimationPlayState::Paused;
                        anim.current_time = anim.duration + anim.delay;
                    } else {
                        anim.current_time -= anim.duration;
                    }
                }

                // 插值计算当前属性值
                if anim.current_time >= anim.delay {
                    let progress = if anim.duration > 0.0 {
                        ((anim.current_time - anim.delay) / anim.duration).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };

                    if let Some(kf) = self.keyframes.get(&anim.name) {
                        let interpolated = Self::interpolate_keyframes(kf, progress);
                        elem_props.extend(interpolated);
                    }
                }
            }

            result.insert(elem_id.clone(), elem_props);
        }

        result
    }

    /// 在关键帧之间插值
    fn interpolate_keyframes(
        keyframes: &KeyframesRule,
        progress: f32,
    ) -> HashMap<String, CSSValue> {
        let mut props = HashMap::new();
        if keyframes.keyframes.is_empty() {
            return props;
        }

        // 对每对关键帧进行线性插值
        let kfs = &keyframes.keyframes;
        if kfs.len() == 1 {
            // 单关键帧：直接使用
            for (prop, val) in &kfs[0].properties {
                props.insert(prop.clone(), val.clone());
            }
            return props;
        }

        // 找到 progress 所在的关键帧区间
        let mut lower_idx = 0;
        let mut upper_idx = kfs.len() - 1;

        for i in 1..kfs.len() {
            if progress <= kfs[i].percent() {
                lower_idx = i - 1;
                upper_idx = i;
                break;
            }
        }

        let lower = &kfs[lower_idx];
        let upper = &kfs[upper_idx];
        let range = upper.percent() - lower.percent();
        let local_progress = if range > 0.0 {
            (progress - lower.percent()) / range
        } else {
            1.0
        };

        // 收集所有属性名
        let mut all_props: Vec<String> = Vec::new();
        for (prop, _) in &lower.properties {
            if !all_props.contains(prop) {
                all_props.push(prop.clone());
            }
        }
        for (prop, _) in &upper.properties {
            if !all_props.contains(prop) {
                all_props.push(prop.clone());
            }
        }

        for prop in &all_props {
            let from_val = lower.properties.get(prop);
            let to_val = upper.properties.get(prop);

            match (from_val, to_val) {
                (Some(from), Some(to)) => {
                    if let Some(interp) =
                        Self::interpolate_value(from, to, local_progress)
                    {
                        props.insert(prop.clone(), interp);
                    }
                }
                (Some(v), None) | (None, Some(v)) => {
                    props.insert(prop.clone(), v.clone());
                }
                (None, None) => {}
            }
        }

        props
    }

    /// 在两个 CSS 值之间插值
    fn interpolate_value(from: &CSSValue, to: &CSSValue, t: f32) -> Option<CSSValue> {
        use CSSValue::*;
        match (from, to) {
            (Length(a, ua), Length(b, _ub)) => Some(Length(a + (b - a) * t, *ua)),
            (Percentage(a), Percentage(b)) => Some(Percentage(a + (b - a) * t)),
            (Number(a), Number(b)) => Some(Number(a + (b - a) * t)),
            _ => None,
        }
    }

    /// 移除元素的指定动画
    pub fn cancel_animation(&mut self, element_id: &str, animation_name: &str) {
        if let Some(anims) = self.active_animations.get_mut(element_id) {
            anims.retain(|a| a.name != animation_name);
        }
    }

    /// 获取动画状态
    pub fn get_animation_state(&self, element_id: &str) -> Option<&[AnimationState]> {
        self.active_animations.get(element_id).map(|v| v.as_slice())
    }
}
