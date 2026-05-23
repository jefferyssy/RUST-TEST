//! 事件循环与动画帧调度
//!
//! Phase 1: AnimationFrameScheduler 支持 requestAnimationFrame / cancelAnimationFrame。

use std::collections::HashMap;

/// 动画帧回调类型
pub type FrameCallback = Box<dyn FnOnce(f64)>;

/// 动画帧调度器 —— 管理 requestAnimationFrame 回调
///
/// 在每次 about_to_wait 时触发所有已注册的回调。
pub struct AnimationFrameScheduler {
    callbacks: HashMap<u32, FrameCallback>,
    next_frame_id: u32,
    current_time: f64,
}

impl AnimationFrameScheduler {
    pub fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
            next_frame_id: 1,
            current_time: 0.0,
        }
    }

    /// 注册动画帧回调，返回 frame_id
    pub fn request_animation_frame(&mut self, callback: FrameCallback) -> u32 {
        let id = self.next_frame_id;
        self.next_frame_id += 1;
        self.callbacks.insert(id, callback);
        id
    }

    /// 取消动画帧回调
    pub fn cancel_animation_frame(&mut self, frame_id: u32) {
        self.callbacks.remove(&frame_id);
    }

    /// 触发所有待执行的回调（每帧调用一次）
    pub fn tick(&mut self, timestamp: f64) {
        self.current_time = timestamp;
        let callbacks = std::mem::take(&mut self.callbacks);
        for (_, cb) in callbacks {
            cb(self.current_time);
        }
    }

    /// 获取当前帧时间戳
    pub fn now(&self) -> f64 {
        self.current_time
    }

    /// 待处理的回调数量
    pub fn pending_count(&self) -> usize {
        self.callbacks.len()
    }
}

// Phase 2+: microtask queue, setTimeout/setInterval 调度

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_scheduler() {
        let sched = AnimationFrameScheduler::new();
        assert_eq!(sched.now(), 0.0);
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn test_request_returns_unique_ids() {
        let mut sched = AnimationFrameScheduler::new();
        let id1 = sched.request_animation_frame(Box::new(|_| {}));
        let id2 = sched.request_animation_frame(Box::new(|_| {}));
        assert_ne!(id1, id2);
        assert_eq!(sched.pending_count(), 2);
    }

    #[test]
    fn test_cancel_animation_frame() {
        let mut sched = AnimationFrameScheduler::new();
        let id = sched.request_animation_frame(Box::new(|_| {}));
        assert_eq!(sched.pending_count(), 1);
        sched.cancel_animation_frame(id);
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn test_cancel_nonexistent_id_does_not_panic() {
        let mut sched = AnimationFrameScheduler::new();
        sched.cancel_animation_frame(999);
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn test_tick_executes_callbacks() {
        let mut sched = AnimationFrameScheduler::new();
        let mut called = false;
        let called_ptr = &mut called as *mut bool;
        sched.request_animation_frame(Box::new(move |ts| {
            assert!(ts > 0.0);
            unsafe { *called_ptr = true; }
        }));
        sched.tick(16.67);
        assert!(called);
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn test_tick_updates_now() {
        let mut sched = AnimationFrameScheduler::new();
        sched.tick(100.0);
        assert_eq!(sched.now(), 100.0);
    }

    #[test]
    fn test_tick_with_no_callbacks() {
        let mut sched = AnimationFrameScheduler::new();
        sched.tick(50.0); // should not panic
        assert_eq!(sched.now(), 50.0);
    }

    #[test]
    fn test_multiple_callbacks_in_one_tick() {
        let mut sched = AnimationFrameScheduler::new();
        let mut count = 0;
        let count_ptr = &mut count as *mut i32;
        sched.request_animation_frame(Box::new(move |_| {
            unsafe { *count_ptr += 1; }
        }));
        sched.request_animation_frame(Box::new(move |_| {
            unsafe { *count_ptr += 1; }
        }));
        sched.tick(33.33);
        assert_eq!(count, 2);
    }
}