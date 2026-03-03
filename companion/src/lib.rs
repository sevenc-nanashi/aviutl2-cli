use aviutl2::{anyhow::Context, tracing};

#[aviutl2::plugin(GenericPlugin)]
struct CompanionAux2 {
    restart_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    _process_lock: std::fs::File,
    thread_handle: Option<std::thread::JoinHandle<()>>,
    graceful_stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle = aviutl2::generic::GlobalEditHandle::new();

static DATA_ROOT: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| {
    process_path::get_executable_path()
        .unwrap()
        .with_file_name("aviutl2-cli-companion")
});
static FLAG_PATH: std::sync::LazyLock<std::path::PathBuf> =
    std::sync::LazyLock::new(|| DATA_ROOT.join("restart_flag.txt"));

impl aviutl2::generic::GenericPlugin for CompanionAux2 {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();

        std::fs::create_dir_all(&*DATA_ROOT)?;
        std::fs::remove_file(&*FLAG_PATH).ok();

        let lock_file_path = DATA_ROOT.join("process.lock");
        let process_lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&lock_file_path)
            .with_context(|| format!("Failed to create lock file: {}", lock_file_path.display()))?;
        process_lock.try_lock()?;

        let graceful_stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let restart_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        Ok(Self {
            thread_handle: Some(std::thread::spawn({
                let graceful_stop_flag = graceful_stop_flag.clone();
                let restart_flag = restart_flag.clone();
                move || {
                    watch_flag_file(graceful_stop_flag, restart_flag);
                }
            })),
            _process_lock: process_lock,
            restart_flag,
            graceful_stop_flag,
        })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "aviutl2-cli companion".into(),
            information: "A companion plugin for aviutl2-cli".into(),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        EDIT_HANDLE.init(registry.create_edit_handle());
    }
}
impl Drop for CompanionAux2 {
    fn drop(&mut self) {
        if self.restart_flag.load(std::sync::atomic::Ordering::SeqCst) {
            tracing::info!("Shutting down host application...");
            std::process::exit(0);
        } else {
            tracing::info!("Cleaning up plugin resources...");
            self.graceful_stop_flag
                .store(true, std::sync::atomic::Ordering::SeqCst);
            if let Err(e) = self.thread_handle.take().unwrap().join() {
                tracing::error!("Failed to join thread: {:?}", e);
            }
        }
    }
}

fn watch_flag_file(
    graceful_stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    restart_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let path = FLAG_PATH.clone();
    tracing::info!("Watching for flag file at: {:?}", path);
    std::thread::spawn(move || {
        while !graceful_stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
            if EDIT_HANDLE.is_ready() && path.exists() {
                tracing::info!("Restart flag detected. Restarting host application...");
                restart_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                EDIT_HANDLE.restart_host_app();
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}

aviutl2::register_generic_plugin!(CompanionAux2);
