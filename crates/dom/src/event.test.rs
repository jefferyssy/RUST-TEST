use super::*;

#[test]
fn test_event_new() {
    let e = Event::new("click");
    assert_eq!(e.event_type, "click");
    assert!(e.bubbles);
    assert!(e.cancelable);
    assert!(!e.default_prevented());
    assert!(!e.propagation_stopped());
    assert!(e.time_stamp > 0.0);
}

#[test]
fn test_prevent_default() {
    let e = Event::new("submit");
    assert!(!e.default_prevented());
    e.prevent_default();
    assert!(e.default_prevented());
}

#[test]
fn test_stop_propagation() {
    let e = Event::new("click");
    assert!(!e.propagation_stopped());
    e.stop_propagation();
    assert!(e.propagation_stopped());
}

#[test]
fn test_mouse_event() {
    let me = MouseEvent::new("click", 100.0, 200.0, 0);
    assert_eq!(me.event.event_type, "click");
    assert_eq!(me.client_x, 100.0);
    assert_eq!(me.client_y, 200.0);
    assert_eq!(me.button, 0);
    assert!(me.event.bubbles);
    assert!(!me.alt_key);
}

#[test]
fn test_listener_id_increment() {
    let id1 = next_listener_id();
    let id2 = next_listener_id();
    assert!(id2 > id1);
}
