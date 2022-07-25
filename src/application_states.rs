use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

pub trait ApplicationStates {
    fn is_initialized(&self) -> bool;
    fn is_shutting_down(&self) -> bool;
}

pub enum AppStateCreated {
    Initialized,
    NotInitliazed(AtomicBool),
}

pub struct AppStates {
    pub initialized: AppStateCreated,
    pub shutting_down: Arc<AtomicBool>,
}

impl AppStates {
    pub fn create_un_initialized() -> Self {
        Self {
            initialized: AppStateCreated::NotInitliazed(AtomicBool::new(false)),
            shutting_down: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn create_initialized() -> Self {
        Self {
            initialized: AppStateCreated::Initialized,
            shutting_down: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn wait_until_shutdown(&self) {
        signal_hook::flag::register(signal_hook::consts::SIGTERM, self.shutting_down.clone())
            .unwrap();

        signal_hook::flag::register(signal_hook::consts::SIGINT, self.shutting_down.clone())
            .unwrap();

        while !self.is_shutting_down() {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    pub fn set_initialized(&self) {
        match &self.initialized {
            AppStateCreated::NotInitliazed(state) => {
                state.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            AppStateCreated::Initialized => {}
        }
    }

    pub fn set_shutting_down(&self) {
        self.shutting_down
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn is_shutting_down(&self) -> bool {
        self.shutting_down
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl ApplicationStates for AppStates {
    fn is_initialized(&self) -> bool {
        match self.initialized {
            AppStateCreated::Initialized => true,
            AppStateCreated::NotInitliazed(ref initialized) => {
                initialized.load(std::sync::atomic::Ordering::Relaxed)
            }
        }
    }

    fn is_shutting_down(&self) -> bool {
        self.shutting_down
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
