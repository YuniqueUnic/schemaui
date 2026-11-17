use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::ui_ast::RichContent;

use super::raster::{RasterInput, RasterOutput, Rasterizer};

pub(super) const IMAGE_FRAME_INTERVAL: Duration = Duration::from_millis(33);

pub(super) enum ImageWork {
    Prepare(Arc<RichContent>),
    Render(RasterInput),
}

pub(super) enum ImageResult {
    Prepared(f32, f32),
    Rendered(RasterOutput),
}

struct Request {
    id: u64,
    input: ImageWork,
}

#[derive(Default)]
struct WorkState {
    generation: u64,
    pending: Option<Request>,
    completed: Option<(u64, Result<ImageResult, String>)>,
    stopping: bool,
    finished: bool,
}

#[derive(Default)]
struct Shared {
    state: Mutex<WorkState>,
    wake: Condvar,
}

#[derive(Clone)]
pub(super) struct ImageWorkerClient(Arc<Shared>);

impl ImageWorkerClient {
    pub fn submit(&self, input: ImageWork) -> u64 {
        let mut state = self.0.state.lock().expect("image work state poisoned");
        state.generation = state.generation.wrapping_add(1);
        let id = state.generation;
        state.pending = Some(Request { id, input });
        state.completed = None;
        self.0.wake.notify_one();
        id
    }

    pub fn take(&self, id: u64) -> Option<Result<ImageResult, String>> {
        let mut state = self.0.state.lock().expect("image work state poisoned");
        if state.generation != id {
            return None;
        }
        if let Some((completed, _)) = &state.completed
            && *completed == id
        {
            return state.completed.take().map(|(_, result)| result);
        }
        state
            .finished
            .then(|| Err("Image preview worker stopped before completing the image".into()))
    }

    pub fn cancel(&self, id: u64) {
        let mut state = self.0.state.lock().expect("image work state poisoned");
        if state.generation == id {
            state.generation = state.generation.wrapping_add(1);
            state.pending = None;
            state.completed = None;
        }
    }
}

pub(super) struct ImageWorker {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl ImageWorker {
    pub fn new() -> Result<Self, String> {
        let shared = Arc::new(Shared::default());
        let worker = shared.clone();
        let thread = thread::Builder::new()
            .name("schemaui-images".into())
            .spawn(move || run(worker))
            .map_err(|error| format!("cannot start image preview worker: {error}"))?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    pub fn client(&self) -> ImageWorkerClient {
        ImageWorkerClient(self.shared.clone())
    }
}

impl Drop for ImageWorker {
    fn drop(&mut self) {
        {
            let mut state = self.shared.state.lock().expect("image work state poisoned");
            state.stopping = true;
            state.generation = state.generation.wrapping_add(1);
            state.pending = None;
            state.completed = None;
            self.shared.wake.notify_one();
        }
        // Library raster/encode calls are not interruptible. Only the current
        // call can remain; there is never a queue of obsolete frames to finish.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct WorkerExit(Arc<Shared>);

impl Drop for WorkerExit {
    fn drop(&mut self) {
        self.0
            .state
            .lock()
            .expect("image work state poisoned")
            .finished = true;
    }
}

fn run(shared: Arc<Shared>) {
    let _exit = WorkerExit(shared.clone());
    let mut rasterizer = Rasterizer::default();
    let mut next_frame = Instant::now();
    loop {
        let request = {
            let mut state = shared.state.lock().expect("image work state poisoned");
            loop {
                if state.stopping {
                    break;
                }
                if state.pending.is_none() {
                    state = shared.wake.wait(state).expect("image work state poisoned");
                } else if let Some(delay) = next_frame.checked_duration_since(Instant::now()) {
                    state = shared
                        .wake
                        .wait_timeout(state, delay)
                        .expect("image work state poisoned")
                        .0;
                } else {
                    break;
                }
            }
            if state.stopping {
                break;
            }
            state.pending.take().expect("image request was present")
        };
        let current = || {
            let state = shared.state.lock().expect("image work state poisoned");
            !state.stopping && state.generation == request.id
        };
        next_frame = Instant::now() + IMAGE_FRAME_INTERVAL;
        let result = match request.input {
            ImageWork::Prepare(content) => rasterizer
                .prepare(content, &current)
                .map(|size| size.map(|(width, height)| ImageResult::Prepared(width, height))),
            ImageWork::Render(input) => rasterizer
                .render(input, &current)
                .map(|output| output.map(ImageResult::Rendered)),
        };
        let mut state = shared.state.lock().expect("image work state poisoned");
        if !state.stopping && state.generation == request.id {
            match result {
                Ok(Some(output)) => state.completed = Some((request.id, Ok(output))),
                Err(error) => state.completed = Some((request.id, Err(error))),
                Ok(None) => {}
            }
        }
    }
}
