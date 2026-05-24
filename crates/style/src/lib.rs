//! # CSS crate — W3C CSS 引擎
//!
//! 提供 CSS 解析、选择器匹配、级联计算功能。
//! Phase 0 支持 ~30 个核心属性，Phase 3 扩展至 218+ API。

pub mod stylesheet;
pub mod selector;
pub mod cascade;
pub mod values;
pub mod properties;
pub mod animations;
pub mod transitions;
pub mod media;
pub mod custom_props;

pub use stylesheet::{
    parse_stylesheet, parse_inline_style, parse_media_query, parse_keyframes, parse_font_face,
    StyleSheet, Rule, Declaration,
    MediaType, MediaFeature, MediaCondition, MediaQuery,
    KeyframeSelector, Keyframe, KeyframesRule, FontFaceRule,
    ImportRule, ImportError,
};
pub use selector::{
    match_selectors, match_selectors_full, element_matches_selector,
    element_matches_selector_with_node,
    MatchedDeclaration, SelectorEngine,
    SelectorPart, PseudoClass, parse_selector_parts, compute_specificity,
    // Phase 3 新增
    Combinator, SelectorSegment, ParsedSelector,
    AttributeOp, PseudoElement,
    parse_selector, parse_selector_list,
    compute_specificity_parsed,
    matches_complex_selector, create_pseudo_element_node,
};
pub use cascade::{compute_element_style, compute_element_style_with_node, ComputedStyle};
pub use values::{
    CSSValue, CSSUnit, parse_css_value, parse_color, parse_length,
    parse_transform, parse_calc_expression, parse_css_function,
    parse_radial_gradient, parse_backdrop_filter,
    is_current_color,
    CalcValue, CalcNode, Transform, Gradient, GradientType, GradientDirection,
    TopOrBottom, LeftOrRight, ColorStop,
    // Phase 3 新增
    ClipPathShape, TimingFunction, StepDirection,
};
pub use properties::{
    parse_aspect_ratio, parse_contain, parse_content_visibility,
    parse_outline_offset, parse_font_variant, parse_font_stretch,
    parse_word_break, parse_overflow, parse_overflow_wrap,
    parse_transform_style, parse_perspective, parse_perspective_origin,
    parse_backface_visibility, parse_touch_action, parse_will_change,
    parse_phase3_property,
};
pub use animations::{
    AnimationEngine, AnimationState, AnimationPlayState,
    AnimationFillMode, AnimationDirection,
};
pub use transitions::{TransitionEngine, TransitionConfig};
pub use media::{MediaEvaluator, ViewportInfo};
pub use custom_props::CustomPropertyRegistry;
