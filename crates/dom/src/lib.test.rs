use super::*;

#[test]
fn test_color_rgb() {
    let c = Color::rgb(255, 0, 0);
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 255);
}

#[test]
fn test_color_rgba() {
    let c = Color::rgba(0, 255, 0, 128);
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 128);
}

#[test]
fn test_color_constants() {
    assert_eq!(Color::BLACK, Color::rgb(0, 0, 0));
    assert_eq!(Color::WHITE, Color::rgb(255, 255, 255));
    assert_eq!(Color::TRANSPARENT, Color::rgba(0, 0, 0, 0));
}

#[test]
fn test_rect() {
    let r = Rect::new(10.0, 20.0, 100.0, 200.0);
    assert_eq!(r.x, 10.0);
    assert_eq!(r.y, 20.0);
    assert_eq!(r.width, 100.0);
    assert_eq!(r.height, 200.0);
}

#[test]
fn test_size() {
    let s = Size::new(800, 600);
    assert_eq!(s.width, 800);
    assert_eq!(s.height, 600);
}
