//! Controle de um job em execução: cancelamento e pausa, compartilhados entre threads.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

#[derive(Debug, Default)]
pub struct JobControl {
    cancelled: AtomicBool,
    paused: Mutex<bool>,
    wake: Condvar,
}

impl JobControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.wake.notify_all();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn pause(&self) {
        if let Ok(mut p) = self.paused.lock() {
            *p = true;
        }
    }

    pub fn resume(&self) {
        if let Ok(mut p) = self.paused.lock() {
            *p = false;
        }
        self.wake.notify_all();
    }

    pub fn is_paused(&self) -> bool {
        self.paused.lock().map(|p| *p).unwrap_or(false)
    }

    /// Bloqueia enquanto pausado. `tick` é chamado periodicamente (heartbeat).
    /// Retorna `true` se o job foi cancelado durante a espera.
    pub fn wait_while_paused(&self, tick: &mut dyn FnMut(bool)) -> bool {
        let Ok(mut paused) = self.paused.lock() else {
            return self.is_cancelled();
        };
        let mut notified_pause = false;
        while *paused && !self.is_cancelled() {
            if !notified_pause {
                notified_pause = true;
                drop(paused);
                tick(true);
                paused = match self.paused.lock() {
                    Ok(p) => p,
                    Err(_) => return self.is_cancelled(),
                };
                continue;
            }
            paused = match self.wake.wait_timeout(paused, Duration::from_secs(5)) {
                Ok((p, _)) => p,
                Err(_) => return self.is_cancelled(),
            };
            if *paused {
                drop(paused);
                tick(true);
                paused = match self.paused.lock() {
                    Ok(p) => p,
                    Err(_) => return self.is_cancelled(),
                };
            }
        }
        let was_paused = notified_pause;
        drop(paused);
        if was_paused {
            tick(false);
        }
        self.is_cancelled()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn pause_blocks_until_resume() {
        let control = Arc::new(JobControl::new());
        control.pause();
        let c = Arc::clone(&control);
        let handle = std::thread::spawn(move || {
            let mut ticks = Vec::new();
            let cancelled = c.wait_while_paused(&mut |paused| ticks.push(paused));
            (cancelled, ticks)
        });
        std::thread::sleep(Duration::from_millis(50));
        control.resume();
        let (cancelled, ticks) = handle.join().unwrap();
        assert!(!cancelled);
        assert_eq!(ticks.first(), Some(&true));
        assert_eq!(ticks.last(), Some(&false));
    }

    #[test]
    fn cancel_wakes_paused_job() {
        let control = Arc::new(JobControl::new());
        control.pause();
        let c = Arc::clone(&control);
        let handle = std::thread::spawn(move || c.wait_while_paused(&mut |_| {}));
        std::thread::sleep(Duration::from_millis(50));
        control.cancel();
        assert!(handle.join().unwrap());
    }

    #[test]
    fn not_paused_returns_immediately() {
        let control = JobControl::new();
        let mut called = false;
        assert!(!control.wait_while_paused(&mut |_| called = true));
        assert!(!called);
    }
}
