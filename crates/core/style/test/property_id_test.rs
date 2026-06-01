//! property_id 模块测试

use super::*;

#[test]
fn test_from_str_common_properties() {
    assert_eq!(PropertyId::from_str("width"), Some(PropertyId::Width));
    assert_eq!(PropertyId::from_str("height"), Some(PropertyId::Height));
    assert_eq!(PropertyId::from_str("display"), Some(PropertyId::Display));
    assert_eq!(PropertyId::from_str("color"), Some(PropertyId::Color));
    assert_eq!(PropertyId::from_str("font-size"), Some(PropertyId::FontSize));
    assert_eq!(PropertyId::from_str("background-color"), Some(PropertyId::BackgroundColor));
    assert_eq!(PropertyId::from_str("border-radius"), Some(PropertyId::BorderRadius));
    assert_eq!(PropertyId::from_str("z-index"), Some(PropertyId::ZIndex));
}

#[test]
fn test_from_str_box_model_properties() {
    assert_eq!(PropertyId::from_str("margin"), Some(PropertyId::Margin));
    assert_eq!(PropertyId::from_str("margin-top"), Some(PropertyId::MarginTop));
    assert_eq!(PropertyId::from_str("padding"), Some(PropertyId::Padding));
    assert_eq!(PropertyId::from_str("padding-left"), Some(PropertyId::PaddingLeft));
    assert_eq!(PropertyId::from_str("border"), Some(PropertyId::Border));
}

#[test]
fn test_from_str_flex_properties() {
    assert_eq!(PropertyId::from_str("flex-direction"), Some(PropertyId::FlexDirection));
    assert_eq!(PropertyId::from_str("flex-wrap"), Some(PropertyId::FlexWrap));
    assert_eq!(PropertyId::from_str("justify-content"), Some(PropertyId::JustifyContent));
    assert_eq!(PropertyId::from_str("align-items"), Some(PropertyId::AlignItems));
    assert_eq!(PropertyId::from_str("flex-grow"), Some(PropertyId::FlexGrow));
    assert_eq!(PropertyId::from_str("flex"), Some(PropertyId::Flex));
    assert_eq!(PropertyId::from_str("gap"), Some(PropertyId::Gap));
}

#[test]
fn test_from_str_unknown() {
    assert_eq!(PropertyId::from_str("unknown-property"), None);
    assert_eq!(PropertyId::from_str("not-a-css-prop"), None);
    assert_eq!(PropertyId::from_str(""), None);
    assert_eq!(PropertyId::from_str("Width"), None); // 大小写敏感
}

#[test]
fn test_name_roundtrip() {
    let ids = [
        PropertyId::Width,
        PropertyId::Height,
        PropertyId::MarginTop,
        PropertyId::PaddingLeft,
        PropertyId::Display,
        PropertyId::FontSize,
        PropertyId::BackgroundColor,
        PropertyId::BoxShadow,
        PropertyId::FlexDirection,
        PropertyId::GridTemplateColumns,
        PropertyId::ZIndex,
        PropertyId::Transform,
    ];
    for id in &ids {
        let name = id.name();
        let parsed = PropertyId::from_str(name);
        assert_eq!(parsed, Some(*id), "Roundtrip failed for {:?}", id);
    }
}

#[test]
fn test_display_trait() {
    assert_eq!(format!("{}", PropertyId::Width), "width");
    assert_eq!(format!("{}", PropertyId::FontSize), "font-size");
    assert_eq!(format!("{}", PropertyId::BackgroundColor), "background-color");
    assert_eq!(format!("{}", PropertyId::JustifyContent), "justify-content");
}

#[test]
fn test_property_id_copy() {
    let id = PropertyId::Width;
    let copied = id;
    assert_eq!(id, copied);
}

#[test]
fn test_property_id_eq() {
    assert_eq!(PropertyId::Width, PropertyId::Width);
    assert_ne!(PropertyId::Width, PropertyId::Height);
}
