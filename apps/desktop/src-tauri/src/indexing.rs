//! Fila de indexação do desktop: um job por vez (consciente de recursos), pedidos do usuário à
//! frente dos automáticos, pausa/cancelamento e eventos de progresso para a interface.

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use nexus_core::index::{
    IndexJob, IndexMode, IndexObserver, IndexTrigger, IndexingQueue, JobControl, JobPhase,
    JobStatus,
};
use nexus_core::{Core, ErrorCode, ErrorInfo};

pub const EVENT_PROGRESS: &str = "nexus://job-progress";
pub const EVENT_FINISHED: &str = "nexus://job-finished";
pub const EVENT_QUEUE: &str = "nexus://queue-changed";

/// Intervalo mínimo entre eventos de progresso (mudanças de status/fase sempre passam).
const PROGRESS_EVERY: Duration = Duration::from_millis(150);

/// Destino dos eventos (o `AppHandle` em produção; um gravador nos testes).
pub trait EventSink: Send + Sync + 'static {
    fn job(&self, event: &str, job: &IndexJob);
    fn queue(&self, queue: &IndexingQueue);
}

impl<R: tauri::Runtime> EventSink for tauri::AppHandle<R> {
    fn job(&self, event: &str, job: &IndexJob) {
        use tauri::Emitter;
        if let Err(e) = self.emit(event, job) {
            tracing::warn!(error = %e, event, "falha ao emitir evento");
        }
    }

    fn queue(&self, queue: &IndexingQueue) {
        use tauri::Emitter;
        if let Err(e) = self.emit(EVENT_QUEUE, queue) {
            tracing::warn!(error = %e, "falha ao emitir evento da fila");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub project_id: i64,
    pub mode: IndexMode,
    pub trigger: IndexTrigger,
}

struct Running {
    project_id: i64,
    control: Arc<JobControl>,
}

#[derive(Default)]
struct State {
    queue: VecDeque<Request>,
    running: Option<Running>,
    /// Mudanças chegaram durante o job deste projeto: reindexar ao terminar.
    rerun: HashSet<i64>,
    shutdown: bool,
}

struct Inner {
    core: Arc<Core>,
    sink: Box<dyn EventSink>,
    state: Mutex<State>,
    wake: Condvar,
}

impl Inner {
    fn lock(&self) -> MutexGuard<'_, State> {
        // Um panic em outro ponto não deve travar a fila para sempre.
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn snapshot(state: &State) -> IndexingQueue {
        IndexingQueue {
            running: state.running.as_ref().map(|r| r.project_id),
            paused: state
                .running
                .as_ref()
                .is_some_and(|r| r.control.is_paused()),
            queued: state.queue.iter().map(|r| r.project_id).collect(),
        }
    }

    fn publish(&self) {
        let snapshot = Self::snapshot(&self.lock());
        self.sink.queue(&snapshot);
    }
}

#[derive(Clone)]
pub struct JobManager {
    inner: Arc<Inner>,
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
}

fn is_user(trigger: IndexTrigger) -> bool {
    matches!(trigger, IndexTrigger::User | IndexTrigger::Cli)
}

impl JobManager {
    pub fn start(core: Arc<Core>, sink: impl EventSink) -> Self {
        let inner = Arc::new(Inner {
            core,
            sink: Box::new(sink),
            state: Mutex::new(State::default()),
            wake: Condvar::new(),
        });
        let worker_inner = Arc::clone(&inner);
        let handle = std::thread::Builder::new()
            .name("nexus-jobs".into())
            .spawn(move || worker(&worker_inner))
            .map_err(
                |e| tracing::error!(error = %e, "não foi possível iniciar a fila de indexação"),
            )
            .ok();
        Self {
            inner,
            worker: Arc::new(Mutex::new(handle)),
        }
    }

    /// Enfileira um pedido. Pedidos repetidos para o mesmo projeto são combinados.
    pub fn enqueue(&self, request: Request) -> Result<(), ErrorInfo> {
        {
            let mut state = self.inner.lock();
            if state.shutdown {
                return Ok(());
            }
            if state
                .running
                .as_ref()
                .is_some_and(|r| r.project_id == request.project_id)
            {
                if is_user(request.trigger) {
                    return Err(ErrorInfo {
                        code: ErrorCode::Conflict,
                        message: "Este projeto já está sendo indexado.".into(),
                        op_id: None,
                    });
                }
                state.rerun.insert(request.project_id);
                return Ok(());
            }
            if let Some(pos) = state
                .queue
                .iter()
                .position(|r| r.project_id == request.project_id)
            {
                let mut merged = state.queue[pos];
                if request.mode == IndexMode::Full {
                    merged.mode = IndexMode::Full;
                }
                if is_user(request.trigger) && !is_user(merged.trigger) {
                    merged.trigger = request.trigger;
                    state.queue.remove(pos);
                    state.queue.push_front(merged);
                } else {
                    state.queue[pos] = merged;
                }
            } else if is_user(request.trigger) {
                // Interação do usuário tem prioridade sobre reconciliações automáticas.
                let first_auto = state
                    .queue
                    .iter()
                    .position(|r| !is_user(r.trigger))
                    .unwrap_or(state.queue.len());
                state.queue.insert(first_auto, request);
            } else {
                state.queue.push_back(request);
            }
        }
        self.inner.wake.notify_all();
        self.inner.publish();
        Ok(())
    }

    fn with_running(&self, project_id: i64, f: impl FnOnce(&JobControl)) -> bool {
        let state = self.inner.lock();
        match &state.running {
            Some(r) if r.project_id == project_id => {
                f(&r.control);
                true
            }
            _ => false,
        }
    }

    pub fn pause(&self, project_id: i64) -> bool {
        let done = self.with_running(project_id, JobControl::pause);
        self.inner.publish();
        done
    }

    pub fn resume(&self, project_id: i64) -> bool {
        let done = self.with_running(project_id, JobControl::resume);
        self.inner.publish();
        done
    }

    /// Cancela o job em execução e/ou remove o projeto da fila.
    pub fn cancel(&self, project_id: i64) -> bool {
        let removed = {
            let mut state = self.inner.lock();
            state.rerun.remove(&project_id);
            let before = state.queue.len();
            state.queue.retain(|r| r.project_id != project_id);
            before != state.queue.len()
        };
        let cancelled = self.with_running(project_id, |c| {
            c.resume();
            c.cancel();
        });
        self.inner.publish();
        removed || cancelled
    }

    pub fn queue(&self) -> IndexingQueue {
        Inner::snapshot(&self.inner.lock())
    }

    /// Encerra: cancela o job atual (fica registrado como cancelado, não interrompido).
    pub fn shutdown(&self, wait: Duration) {
        {
            let mut state = self.inner.lock();
            state.shutdown = true;
            state.queue.clear();
            if let Some(r) = &state.running {
                r.control.resume();
                r.control.cancel();
            }
        }
        self.inner.wake.notify_all();
        let handle = self.worker.lock().ok().and_then(|mut h| h.take());
        if let Some(handle) = handle {
            let deadline = Instant::now() + wait;
            while !handle.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            if handle.is_finished() {
                let _ = handle.join();
            }
        }
    }
}

struct EmitObserver<'a> {
    sink: &'a dyn EventSink,
    last: Mutex<(Option<Instant>, JobStatus, JobPhase)>,
}

impl IndexObserver for EmitObserver<'_> {
    fn on_update(&self, job: &IndexJob) {
        let Ok(mut last) = self.last.lock() else {
            return;
        };
        let changed = last.1 != job.status || last.2 != job.phase;
        let due = last.0.is_none_or(|t| t.elapsed() >= PROGRESS_EVERY);
        if changed || due {
            *last = (Some(Instant::now()), job.status, job.phase);
            drop(last);
            self.sink.job(EVENT_PROGRESS, job);
        }
    }
}

fn worker(inner: &Inner) {
    loop {
        let (request, control) = {
            let mut state = inner.lock();
            loop {
                if state.shutdown {
                    return;
                }
                // Um job por vez; `running` só é ocupado aqui.
                if state.running.is_none()
                    && let Some(request) = state.queue.pop_front()
                {
                    let control = Arc::new(JobControl::new());
                    state.running = Some(Running {
                        project_id: request.project_id,
                        control: Arc::clone(&control),
                    });
                    break (request, control);
                }
                state = inner
                    .wake
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };
        inner.publish();

        let observer = EmitObserver {
            sink: inner.sink.as_ref(),
            last: Mutex::new((None, JobStatus::Running, JobPhase::Scanning)),
        };
        let result = inner
            .core
            .index_options(request.mode, request.trigger)
            .and_then(|opts| {
                inner
                    .core
                    .index_project(request.project_id, &opts, &control, &observer)
            });
        match result {
            Ok(job) => inner.sink.job(EVENT_FINISHED, &job),
            Err(e) => {
                tracing::warn!(project_id = request.project_id, error = %e, "indexação falhou");
                // O job (se chegou a ser criado) foi gravado como falho: publica o estado final.
                if let Ok(Some(job)) = inner
                    .core
                    .list_jobs(request.project_id, 1)
                    .map(|mut jobs| jobs.pop())
                {
                    inner.sink.job(EVENT_FINISHED, &job);
                }
            }
        }

        {
            let mut state = inner.lock();
            state.running = None;
            if state.rerun.remove(&request.project_id)
                && !state.shutdown
                && !state
                    .queue
                    .iter()
                    .any(|r| r.project_id == request.project_id)
            {
                state.queue.push_back(Request {
                    project_id: request.project_id,
                    mode: IndexMode::Incremental,
                    trigger: IndexTrigger::Watcher,
                });
            }
        }
        inner.publish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        jobs: Mutex<Vec<(String, IndexJob)>>,
        queues: Mutex<Vec<IndexingQueue>>,
    }

    impl EventSink for Arc<Recorder> {
        fn job(&self, event: &str, job: &IndexJob) {
            self.jobs
                .lock()
                .unwrap()
                .push((event.to_owned(), job.clone()));
        }
        fn queue(&self, queue: &IndexingQueue) {
            self.queues.lock().unwrap().push(queue.clone());
        }
    }

    fn setup(files: usize) -> (tempfile::TempDir, Arc<Core>, i64) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        for i in 0..files {
            std::fs::write(repo.join(format!("f{i}.txt")), format!("arquivo {i}\n")).unwrap();
        }
        let core =
            Arc::new(Core::open(nexus_core::DataPaths::at(tmp.path().join("data"))).unwrap());
        let id = core.add_project(&repo).unwrap().id;
        (tmp, core, id)
    }

    fn wait_for(cond: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !cond() {
            assert!(Instant::now() < deadline, "tempo esgotado");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn runs_queued_job_and_emits_progress_and_finished() {
        let (_tmp, core, id) = setup(10);
        let recorder = Arc::new(Recorder::default());
        let manager = JobManager::start(Arc::clone(&core), Arc::clone(&recorder));
        manager
            .enqueue(Request {
                project_id: id,
                mode: IndexMode::Incremental,
                trigger: IndexTrigger::User,
            })
            .unwrap();
        wait_for(|| {
            recorder
                .jobs
                .lock()
                .unwrap()
                .iter()
                .any(|(e, _)| e == EVENT_FINISHED)
        });
        let jobs = recorder.jobs.lock().unwrap();
        assert!(jobs.iter().any(|(e, _)| e == EVENT_PROGRESS));
        let (_, finished) = jobs.iter().find(|(e, _)| e == EVENT_FINISHED).unwrap();
        assert_eq!(finished.status, JobStatus::Succeeded);
        assert_eq!(finished.discovered, 10);
        drop(jobs);
        wait_for(|| manager.queue().running.is_none());
        manager.shutdown(Duration::from_secs(5));
    }

    #[test]
    fn duplicate_requests_are_merged_and_user_requests_jump_the_queue() {
        let (_tmp, core, id) = setup(1);
        let recorder = Arc::new(Recorder::default());
        let manager = JobManager::start(core, Arc::clone(&recorder));
        // Mantém o worker ocupado com um "projeto" falso? Usamos a fila diretamente sem worker livre:
        {
            let mut state = manager.inner.lock();
            state.running = Some(Running {
                project_id: 999,
                control: Arc::new(JobControl::new()),
            });
        }
        let auto = |pid| Request {
            project_id: pid,
            mode: IndexMode::Incremental,
            trigger: IndexTrigger::Reconcile,
        };
        manager.enqueue(auto(1)).unwrap();
        manager.enqueue(auto(2)).unwrap();
        manager
            .enqueue(Request {
                project_id: 3,
                mode: IndexMode::Incremental,
                trigger: IndexTrigger::User,
            })
            .unwrap();
        manager
            .enqueue(Request {
                project_id: 2,
                mode: IndexMode::Full,
                trigger: IndexTrigger::User,
            })
            .unwrap();
        let q = manager.queue();
        assert_eq!(q.running, Some(999));
        assert_eq!(q.queued, vec![2, 3, 1]);
        assert_eq!(manager.inner.lock().queue[0].mode, IndexMode::Full);
        // Pedido automático para o projeto em execução vira "reindexar depois".
        manager.enqueue(auto(999)).unwrap();
        assert!(manager.inner.lock().rerun.contains(&999));
        // Pedido do usuário para o projeto em execução é recusado com mensagem clara.
        let err = manager
            .enqueue(Request {
                project_id: 999,
                mode: IndexMode::Incremental,
                trigger: IndexTrigger::User,
            })
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Conflict);
        assert!(manager.cancel(1));
        assert_eq!(manager.queue().queued, vec![2, 3]);
        manager.inner.lock().running = None;
        let _ = id;
        manager.shutdown(Duration::from_secs(5));
    }

    #[test]
    fn shutdown_cancels_the_running_job_and_clears_the_queue() {
        let (_tmp, core, _id) = setup(1);
        let manager = JobManager::start(core, Arc::new(Recorder::default()));
        let control = Arc::new(JobControl::new());
        control.pause();
        {
            let mut state = manager.inner.lock();
            state.running = Some(Running {
                project_id: 7,
                control: Arc::clone(&control),
            });
            state.queue.push_back(Request {
                project_id: 8,
                mode: IndexMode::Incremental,
                trigger: IndexTrigger::Watcher,
            });
        }
        manager.shutdown(Duration::from_secs(5));
        assert!(control.is_cancelled());
        assert!(
            !control.is_paused(),
            "um job pausado precisa acordar para terminar"
        );
        assert!(manager.queue().queued.is_empty());
    }
}
