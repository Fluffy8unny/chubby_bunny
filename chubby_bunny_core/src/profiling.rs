#[derive(Clone, serde::Serialize, Default)]
pub struct ProfileNode {
    pub name: String,
    pub call_count: u32,
    pub total_time_us: f64,
    pub min_time_us: f64,
    pub max_time_us: f64,
    pub avg_time_us: f64,
    pub children: Vec<ProfileNode>,
}

#[cfg(feature = "profiling")]
mod runtime {
    use super::ProfileNode;
    use std::cell::RefCell;

    /// A scope that is currently open on the profiler stack.
    /// `name` is always a string literal passed to `profile_scope!`, so no
    /// allocation is needed while the scope is active.
    struct ActiveNode {
        name: &'static str,
        start_time_us: f64,
        children: Vec<ProfileNode>,
    }

    #[derive(Default)]
    struct ProfilerState {
        frame_active: bool,
        frame_name: String,
        frame_start_us: f64,
        stack: Vec<ActiveNode>,
        root_children: Vec<ProfileNode>,
        last_frame: Option<ProfileNode>,
    }

    thread_local! {
        static PROFILER_STATE: RefCell<ProfilerState> = RefCell::new(ProfilerState::default());
    }

    fn now_us() -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            return web_sys::window()
                .and_then(|window| window.performance())
                .map(|performance| performance.now() * 1000.0)
                .unwrap_or(0.0);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
            let start = START.get_or_init(std::time::Instant::now);
            return start.elapsed().as_secs_f64() * 1_000_000.0;
        }
    }

    /// Merges a finished scope into `children`, aggregating by name.
    /// Only allocates a `String` the first time a given `name` is seen within
    /// `children`; every subsequent call for the same name is allocation-free.
    fn merge_active_node(
        children: &mut Vec<ProfileNode>,
        name: &'static str,
        elapsed_us: f64,
        sub_children: Vec<ProfileNode>,
    ) {
        if let Some(existing) = children.iter_mut().find(|existing| existing.name == name) {
            existing.call_count += 1;
            existing.total_time_us += elapsed_us;
            existing.min_time_us = existing.min_time_us.min(elapsed_us);
            existing.max_time_us = existing.max_time_us.max(elapsed_us);
            merge_children(&mut existing.children, sub_children);
            return;
        }

        children.push(ProfileNode {
            name: name.to_string(),
            call_count: 1,
            total_time_us: elapsed_us,
            min_time_us: elapsed_us,
            max_time_us: elapsed_us,
            avg_time_us: elapsed_us,
            children: sub_children,
        });
    }

    /// Folds an already-built subtree (from a finished child scope) into `children`.
    fn merge_children(children: &mut Vec<ProfileNode>, mut incoming: Vec<ProfileNode>) {
        for node in incoming.drain(..) {
            if let Some(existing) = children.iter_mut().find(|existing| existing.name == node.name) {
                existing.call_count += node.call_count;
                existing.total_time_us += node.total_time_us;
                existing.min_time_us = existing.min_time_us.min(node.min_time_us);
                existing.max_time_us = existing.max_time_us.max(node.max_time_us);
                merge_children(&mut existing.children, node.children);
            } else {
                children.push(node);
            }
        }
    }

    fn finalize_averages(node: &mut ProfileNode) {
        if node.call_count > 0 {
            node.avg_time_us = node.total_time_us / f64::from(node.call_count);
        }
        for child in &mut node.children {
            finalize_averages(child);
        }
    }

    /// RAII scope guard. Pushes onto the thread-local profiler stack on
    /// creation and merges its accumulated timing into its parent on drop.
    /// No heap allocation happens while the guard is alive.
    pub struct ProfileGuard {
        active: bool,
    }

    impl ProfileGuard {
        pub fn new(name: &'static str) -> Self {
            let active = PROFILER_STATE.with(|state| {
                let mut state = state.borrow_mut();
                if !state.frame_active {
                    return false;
                }
                state.stack.push(ActiveNode {
                    name,
                    start_time_us: now_us(),
                    children: Vec::new(),
                });
                true
            });
            Self { active }
        }
    }

    impl Drop for ProfileGuard {
        fn drop(&mut self) {
            if !self.active {
                return;
            }

            PROFILER_STATE.with(|state| {
                let mut state = state.borrow_mut();
                let Some(active_node) = state.stack.pop() else {
                    return;
                };

                let elapsed_us = (now_us() - active_node.start_time_us).max(0.0);
                let target = match state.stack.last_mut() {
                    Some(parent) => &mut parent.children,
                    None => &mut state.root_children,
                };
                merge_active_node(target, active_node.name, elapsed_us, active_node.children);
            });
        }
    }

    pub fn begin_frame(name: &str) {
        PROFILER_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.frame_active = true;
            state.frame_name = name.to_string();
            state.frame_start_us = now_us();
            state.stack.clear();
            state.root_children.clear();
            state.last_frame = None;
        });
    }

    pub fn end_frame() {
        PROFILER_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.frame_active = false;
            state.stack.clear();

            let frame_name = std::mem::take(&mut state.frame_name);
            if frame_name.is_empty() {
                state.last_frame = None;
                return;
            }

            let elapsed_us = (now_us() - state.frame_start_us).max(0.0);
            let mut frame = ProfileNode {
                name: frame_name,
                call_count: 1,
                total_time_us: elapsed_us,
                min_time_us: elapsed_us,
                max_time_us: elapsed_us,
                avg_time_us: elapsed_us,
                children: std::mem::take(&mut state.root_children),
            };
            finalize_averages(&mut frame);
            state.last_frame = Some(frame);
            state.frame_start_us = 0.0;
        });
    }

    pub fn take_last_frame() -> Option<ProfileNode> {
        PROFILER_STATE.with(|state| state.borrow_mut().last_frame.take())
    }
}

#[cfg(feature = "profiling")]
pub use runtime::{begin_frame, end_frame, take_last_frame, ProfileGuard};

#[cfg(not(feature = "profiling"))]
pub struct ProfileGuard;

#[cfg(not(feature = "profiling"))]
impl ProfileGuard {
    pub fn new(name: &'static str) -> Self {
        let _ = name;
        Self
    }
}

#[cfg(not(feature = "profiling"))]
pub fn begin_frame(_name: &str) {}

#[cfg(not(feature = "profiling"))]
pub fn end_frame() {}

#[cfg(not(feature = "profiling"))]
pub fn take_last_frame() -> Option<ProfileNode> {
    None
}