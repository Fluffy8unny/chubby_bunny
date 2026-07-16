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
    use std::sync::Once;
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;

    struct ActiveNode {
        name: String,
        start_time_us: f64,
        children: Vec<ProfileNode>,
    }

    #[derive(Default)]
    struct SpanScopeNameVisitor {
        scope_name: Option<String>,
    }

    impl Visit for SpanScopeNameVisitor {
        fn record_str(&mut self, field: &Field, value: &str) {
            if field.name() == "scope_name" {
                self.scope_name = Some(value.to_string());
            }
        }

        fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
    }

    #[derive(Clone)]
    struct SpanScopeName(String);

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

    struct ProfilingLayer;

    impl<S> Layer<S> for ProfilingLayer
    where
        S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
    {
        fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
            let mut visitor = SpanScopeNameVisitor::default();
            attrs.record(&mut visitor);
            let Some(span) = ctx.span(id) else {
                return;
            };
            let scope_name = visitor
                .scope_name
                .unwrap_or_else(|| span.metadata().name().to_string());
            span.extensions_mut().insert(SpanScopeName(scope_name));
        }

        fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
            let Some(span) = ctx.span(id) else {
                return;
            };

            let scope_name = span
                .extensions()
                .get::<SpanScopeName>()
                .map(|x| x.0.clone())
                .unwrap_or_else(|| span.metadata().name().to_string());

            PROFILER_STATE.with(|state| {
                let mut state = state.borrow_mut();
                if !state.frame_active {
                    return;
                }

                state.stack.push(ActiveNode {
                    name: scope_name,
                    start_time_us: now_us(),
                    children: Vec::new(),
                });
            });
        }

        fn on_exit(&self, _id: &Id, _ctx: Context<'_, S>) {
            PROFILER_STATE.with(|state| {
                let mut state = state.borrow_mut();
                if !state.frame_active {
                    return;
                }

                let Some(active_node) = state.stack.pop() else {
                    return;
                };

                let elapsed_us = (now_us() - active_node.start_time_us).max(0.0);
                let finished_node = ProfileNode {
                    name: active_node.name,
                    call_count: 1,
                    total_time_us: elapsed_us,
                    min_time_us: elapsed_us,
                    max_time_us: elapsed_us,
                    avg_time_us: elapsed_us,
                    children: active_node.children,
                };

                if let Some(parent) = state.stack.last_mut() {
                    merge_profile_node(&mut parent.children, finished_node);
                } else {
                    merge_profile_node(&mut state.root_children, finished_node);
                }
            });
        }
    }

    fn ensure_subscriber_installed() {
        static INSTALL: Once = Once::new();
        INSTALL.call_once(|| {
            let subscriber = tracing_subscriber::registry().with(ProfilingLayer);
            let _ = tracing::subscriber::set_global_default(subscriber);
        });
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

    fn merge_profile_node(children: &mut Vec<ProfileNode>, mut node: ProfileNode) {
        if let Some(existing) = children.iter_mut().find(|existing| existing.name == node.name) {
            existing.call_count += node.call_count;
            existing.total_time_us += node.total_time_us;
            existing.min_time_us = existing.min_time_us.min(node.min_time_us);
            existing.max_time_us = existing.max_time_us.max(node.max_time_us);
            for child in node.children.drain(..) {
                merge_profile_node(&mut existing.children, child);
            }
            return;
        }

        children.push(node);
    }

    fn finalize_averages(node: &mut ProfileNode) {
        if node.call_count > 0 {
            node.avg_time_us = node.total_time_us / f64::from(node.call_count);
        }
        for child in &mut node.children {
            finalize_averages(child);
        }
    }

    pub struct ProfileGuard {
        _entered: tracing::span::EnteredSpan,
    }

    impl ProfileGuard {
        pub fn new(name: &str) -> Self {
            ensure_subscriber_installed();
            Self {
                _entered: tracing::info_span!("profile_scope", scope_name = name).entered(),
            }
        }
    }

    pub fn begin_frame(name: &str) {
        ensure_subscriber_installed();
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
pub use runtime::ProfileGuard;

#[cfg(not(feature = "profiling"))]
pub struct ProfileGuard;

#[cfg(not(feature = "profiling"))]
impl ProfileGuard {
    pub fn new(name: &str) -> Self {
        let _ = name;
        Self
    }
}

pub fn begin_frame(name: &str) {
    #[cfg(feature = "profiling")]
    {
        runtime::begin_frame(name);
    }

    #[cfg(not(feature = "profiling"))]
    {
        let _ = name;
    }
}

pub fn end_frame() {
    #[cfg(feature = "profiling")]
    {
        runtime::end_frame();
    }
}

pub fn take_last_frame() -> Option<ProfileNode> {
    #[cfg(feature = "profiling")]
    {
        return runtime::take_last_frame();
    }

    #[cfg(not(feature = "profiling"))]
    {
        None
    }
}