use super::*;

#[test]
fn test_measure_empty() {
    let mut m = TextMeasurer::new();
    let size = m.measure("", 16.0, "sans-serif", 400);
    assert_eq!(size.width, 0.0);
    assert_eq!(size.height, 0.0);
}

#[test]
fn test_measure_estimates_width() {
    let mut m = TextMeasurer::new();
    // 使用 rustybuzz 精确测量，"Hello" 宽度依赖系统字体，但应 > 0
    let size = m.measure("Hello", 16.0, "sans-serif", 400);
    assert!(size.width > 0.0, "width should be positive, got {}", size.width);
    assert!(size.width < 100.0, "width should be reasonable, got {}", size.width);
    assert_eq!(size.height, 19.2);
}

#[test]
fn test_measure_different_font_size() {
    let mut m = TextMeasurer::new();
    let small = m.measure("Hi", 12.0, "sans-serif", 400);
    let large = m.measure("Hi", 24.0, "sans-serif", 400);
    assert!(large.width > small.width);
    assert!(large.height > small.height);
}

#[test]
fn test_measure_lines_zero_width() {
    let mut m = TextMeasurer::new();
    let lines = m.measure_lines("hello", 16.0, 0.0);
    assert!(lines.is_empty());
}

#[test]
fn test_measure_lines_empty_text() {
    let mut m = TextMeasurer::new();
    let lines = m.measure_lines("", 16.0, 100.0);
    assert!(lines.is_empty());
}

#[test]
fn test_measure_lines_single_line() {
    let mut m = TextMeasurer::new();
    let lines = m.measure_lines("hello", 16.0, 1000.0);
    assert_eq!(lines.len(), 1);
}
