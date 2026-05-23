use super::*;
use dom::{Color, Rect};
use render_tree::{DisplayList, PaintCommand};

/// Mock 渲染后端 —— 记录渲染调用供测试验证
struct MockBackend {
    render_count: u32,
    current_size: (u32, u32),
    last_command_count: usize,
}

impl MockBackend {
    fn new() -> Self {
        Self { render_count: 0, current_size: (800, 600), last_command_count: 0 }
    }
}

impl RenderBackend for MockBackend {
    fn render(&mut self, display_list: &DisplayList) {
        self.render_count += 1;
        self.last_command_count = display_list.len();
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.current_size = (width, height);
    }

    fn present(&mut self) {}

    fn size(&self) -> (u32, u32) {
        self.current_size
    }
}

#[test]
fn test_mock_backend_initial_state() {
    let mock = MockBackend::new();
    assert_eq!(mock.render_count, 0);
    assert_eq!(mock.size(), (800, 600));
}

#[test]
fn test_mock_render_tracks_count() {
    let mut mock = MockBackend::new();
    let mut dl = DisplayList::new();
    dl.push(PaintCommand::FillRect {
        rect: Rect::new(0.0, 0.0, 100.0, 100.0),
        color: Color::rgb(255, 0, 0),
    });
    mock.render(&dl);
    assert_eq!(mock.render_count, 1);
    mock.render(&DisplayList::new());
    assert_eq!(mock.render_count, 2);
}

#[test]
fn test_mock_resize() {
    let mut mock = MockBackend::new();
    mock.resize(1024, 768);
    assert_eq!(mock.size(), (1024, 768));
}

#[test]
fn test_mock_render_tracks_command_count() {
    let mut mock = MockBackend::new();
    let mut dl = DisplayList::new();
    dl.push(PaintCommand::FillRect {
        rect: Rect::new(10.0, 20.0, 50.0, 30.0),
        color: Color::BLACK,
    });
    mock.render(&dl);
    assert_eq!(mock.last_command_count, 1);

    mock.render(&DisplayList::new());
    assert_eq!(mock.last_command_count, 0);
}
