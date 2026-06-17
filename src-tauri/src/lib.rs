use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use enigo::{Direction, Enigo, Key as EnigoKey, Keyboard, Settings as EnigoSettings};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use rdev::{listen, Event, EventType, Key};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, io::Cursor};
use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WindowEvent,
};

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    last_text: Arc<Mutex<String>>,
    hotkey_config: Arc<Mutex<HotkeyConfig>>,
    inline_state: Arc<Mutex<InlineState>>,
    main_pointer_operation_until: Arc<Mutex<Option<Instant>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MainHotkey {
    Alt,
    Ctrl,
}

#[derive(Clone, Copy, Debug)]
struct HotkeyConfig {
    enabled: bool,
    main_hotkey: MainHotkey,
}

#[derive(Clone, Debug)]
struct InlineState {
    enabled: bool,
    active: bool,
    trigger: String,
    query: String,
    selected_index: usize,
    item_ids: Vec<i64>,
    last_slash: Option<Instant>,
    target_hwnd: isize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FloatingPayload {
    query: String,
    trigger: String,
    selected_index: usize,
}

#[derive(Debug, Serialize)]
struct ClipboardItem {
    id: i64,
    kind: String,
    title: String,
    content: String,
    preview: String,
    meta: String,
    source_app: Option<String>,
    created_at: String,
    used_at: Option<String>,
}

struct ClipboardSnapshot {
    kind: String,
    title: String,
    content: String,
    preview: String,
    hash_input: String,
}

struct HtmlAttrRange {
    full_start: usize,
    full_end: usize,
    value_start: Option<usize>,
    value_end: Option<usize>,
}

#[derive(Debug, Serialize)]
struct QuickInput {
    id: i64,
    title: String,
    content: String,
    tags: Vec<String>,
    prefix: String,
    sort_order: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct SaveQuickInputPayload {
    id: Option<i64>,
    content: String,
    tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppSettings {
    history_limit: i64,
    main_hotkey: String,
    main_hotkey_enabled: bool,
    inline_trigger: String,
    inline_trigger_enabled: bool,
    launch_at_startup: bool,
    launch_as_admin: bool,
    auto_hide_to_tray: bool,
    confirm_close_to_tray: bool,
    enter_paste_to_active: bool,
    hide_on_blur: bool,
    confirm_exit: bool,
    move_activated_to_top: bool,
    close_after_activation: bool,
    focus_previous_after_activation: bool,
    paste_after_activation: bool,
    onboarding_completed: bool,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Clipboard(#[from] arboard::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

type AppResult<T> = Result<T, AppError>;

enum SingleInstanceState {
    Primary(SingleInstanceGuard),
    Secondary,
}

struct SingleInstanceGuard {
    #[cfg(target_os = "windows")]
    mutex: winapi::shared::ntdef::HANDLE,
}

impl SingleInstanceGuard {
    fn acquire(name: &str) -> SingleInstanceState {
        #[cfg(target_os = "windows")]
        {
            use std::ptr;
            use winapi::um::{
                errhandlingapi::GetLastError,
                synchapi::CreateMutexW,
            };
            use winapi::shared::winerror::ERROR_ALREADY_EXISTS;

            let mutex_name = format!("Local\\{name}SingleInstance");
            let mutex = unsafe { CreateMutexW(ptr::null_mut(), 1, wide(&mutex_name).as_ptr()) };
            if mutex.is_null() {
                return SingleInstanceState::Primary(SingleInstanceGuard { mutex });
            }
            let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            if already_exists {
                unsafe {
                    winapi::um::handleapi::CloseHandle(mutex);
                }
                SingleInstanceState::Secondary
            } else {
                SingleInstanceState::Primary(SingleInstanceGuard { mutex })
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = name;
            SingleInstanceState::Primary(SingleInstanceGuard {})
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            if !self.mutex.is_null() {
                winapi::um::handleapi::CloseHandle(self.mutex);
            }
        }
    }
}

pub fn run() {
    let single_instance = match SingleInstanceGuard::acquire("SheepClip") {
        SingleInstanceState::Primary(guard) => guard,
        SingleInstanceState::Secondary => {
            notify_primary_instance("SheepClip");
            return;
        }
    };

    tauri::Builder::default()
        .setup(|app| {
            let db_path = prepare_database(app.handle())?;
            let initial_settings = {
                let conn = Connection::open(&db_path)?;
                read_settings(&conn)?
            };
            let hotkey_config = Arc::new(Mutex::new(HotkeyConfig {
                enabled: initial_settings.main_hotkey_enabled,
                main_hotkey: MainHotkey::from_setting(&initial_settings.main_hotkey),
            }));
            let inline_state = Arc::new(Mutex::new(InlineState {
                enabled: initial_settings.inline_trigger_enabled,
                active: false,
                trigger: initial_settings.inline_trigger.clone(),
                query: String::new(),
                selected_index: 0,
                item_ids: Vec::new(),
                last_slash: None,
                target_hwnd: 0,
            }));
            let main_pointer_operation_until = Arc::new(Mutex::new(None));
            let state = AppState {
                db_path: db_path.clone(),
                last_text: Arc::new(Mutex::new(String::new())),
                hotkey_config: hotkey_config.clone(),
                inline_state: inline_state.clone(),
                main_pointer_operation_until: main_pointer_operation_until.clone(),
            };

            setup_main_window(app.handle(), main_pointer_operation_until);
            setup_floating_window(app.handle());
            apply_window_icons(app.handle());
            setup_single_instance_listener(app.handle().clone(), "SheepClip");
            setup_tray(app.handle())?;
            apply_window_visibility_mode_by_path(app.handle(), &db_path)?;
            apply_initial_window_state(app.handle());
            start_clipboard_watcher(
                app.handle().clone(),
                db_path.clone(),
                state.last_text.clone(),
            );
            start_global_keyboard_listener(
                app.handle().clone(),
                db_path,
                hotkey_config,
                inline_state,
            );

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_clipboard_items,
            list_quick_inputs,
            list_tags,
            add_tag,
            delete_tag,
            save_quick_input,
            delete_quick_input,
            reorder_quick_inputs,
            get_settings,
            save_settings,
            reset_settings,
            mark_main_pointer_operation,
            open_external_url,
            copy_item_to_clipboard,
            delete_clipboard_item,
            capture_current_clipboard,
            clear_clipboard_history,
            add_clipboard_item_to_quick_input,
            show_main_window,
            hide_main_window,
            minimize_main_window,
            set_main_window_always_on_top,
            exit_app,
            paste_item_from_main_window,
            paste_quick_input_from_floating,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run SheepClip");
    drop(single_instance);
}

fn setup_main_window(app: &AppHandle, pointer_operation_until: Arc<Mutex<Option<Instant>>>) {
    if let Some(window) = app.get_webview_window("main") {
        let app_handle = app.clone();
        let pointer_guard = pointer_operation_until.clone();
        window.on_window_event(move |event| match event {
            WindowEvent::Focused(false) => {
                if main_pointer_operation_active(&pointer_guard) {
                    return;
                }
                let app_handle = app_handle.clone();
                let pointer_guard = pointer_guard.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(140));
                    if main_pointer_operation_active(&pointer_guard) {
                        return;
                    }
                    let Some(window) = app_handle.get_webview_window("main") else {
                        return;
                    };
                    if window.is_focused().unwrap_or(false)
                        || window.is_always_on_top().unwrap_or(false)
                        || !should_auto_hide(&app_handle).unwrap_or(true)
                    {
                        return;
                    }
                    if should_hide_to_tray(&app_handle).unwrap_or(true) {
                        let _ = window.hide();
                    } else {
                        let _ = window.minimize();
                    }
                });
            }
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                if let Ok(mut until) = pointer_guard.lock() {
                    *until = Some(Instant::now() + Duration::from_millis(500));
                }
            }
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if should_confirm_close(&app_handle).unwrap_or(true) {
                    let _ = app_handle.emit_to("main", "main-close-requested", ());
                } else if let Some(window) = app_handle.get_webview_window("main") {
                    if should_hide_to_tray(&app_handle).unwrap_or(true) {
                        let _ = window.hide();
                    } else {
                        let _ = window.minimize();
                    }
                }
            }
            _ => {}
        });
    }
}

fn setup_floating_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("floating") {
        window.on_window_event(move |event| {
            if matches!(event, WindowEvent::Focused(false)) {
                // Floating hide is handled by the frontend with a short delay so native
                // drag/resize operations do not immediately close the window.
            }
        });
    }
}

fn apply_window_icons(app: &AppHandle) {
    let Some(icon) = app.default_window_icon().cloned() else {
        return;
    };
    for label in ["main", "floating"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_icon(icon.clone());
        }
    }
}

fn setup_single_instance_listener(app: AppHandle, name: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::ptr;
        use winapi::um::{
            handleapi::CloseHandle,
            synchapi::{CreateEventW, ResetEvent, WaitForSingleObject},
            winbase::INFINITE,
        };

        let event_name = format!("Local\\{name}ShowMainWindow");
        thread::spawn(move || {
            let event = unsafe { CreateEventW(ptr::null_mut(), 1, 0, wide(&event_name).as_ptr()) };
            if event.is_null() {
                return;
            }
            loop {
                let wait_result = unsafe { WaitForSingleObject(event, INFINITE) };
                if wait_result == winapi::um::winbase::WAIT_OBJECT_0 {
                    unsafe {
                        ResetEvent(event);
                    }
                    let _ = show_main_window(app.clone());
                } else {
                    break;
                }
            }
            unsafe {
                CloseHandle(event);
            }
        });
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, name);
    }
}

fn notify_primary_instance(name: &str) {
    #[cfg(target_os = "windows")]
    {
        use winapi::um::{
            handleapi::CloseHandle,
            synchapi::{OpenEventW, SetEvent},
            winnt::EVENT_MODIFY_STATE,
        };

        let event_name = format!("Local\\{name}ShowMainWindow");
        let event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, wide(&event_name).as_ptr()) };
        if !event.is_null() {
            unsafe {
                SetEvent(event);
                CloseHandle(event);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = name;
    }
}

fn main_pointer_operation_active(pointer_operation_until: &Arc<Mutex<Option<Instant>>>) -> bool {
    pointer_operation_until
        .lock()
        .ok()
        .and_then(|until| *until)
        .map(|until| until > Instant::now())
        .unwrap_or(false)
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示 SheepClip", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut builder = TrayIconBuilder::new().menu(&menu).tooltip("SheepClip");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                let _ = show_main_window(app.clone());
            }
            "quit" => {
                if should_confirm_close(app).unwrap_or(true) {
                    let _ = show_main_window(app.clone());
                    let _ = app.emit_to("main", "main-close-requested", ());
                } else {
                    app.exit(0);
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_main_window(tray.app_handle().clone());
            }
        })
        .build(app)?;

    Ok(())
}

fn should_auto_hide(app: &AppHandle) -> AppResult<bool> {
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Message(error.to_string()))?
        .join("sheepclip.db");
    let conn = Connection::open(db_path)?;
    read_bool(&conn, "hide_on_blur", true)
}

fn should_confirm_close(app: &AppHandle) -> AppResult<bool> {
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Message(error.to_string()))?
        .join("sheepclip.db");
    let conn = Connection::open(db_path)?;
    read_bool(&conn, "confirm_exit", true)
}

fn should_hide_to_tray(app: &AppHandle) -> AppResult<bool> {
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Message(error.to_string()))?
        .join("sheepclip.db");
    let conn = Connection::open(db_path)?;
    read_bool(&conn, "auto_hide_to_tray", true)
}

fn apply_window_visibility_mode_by_path(app: &AppHandle, db_path: &Path) -> AppResult<()> {
    let conn = Connection::open(db_path)?;
    let auto_hide = read_bool(&conn, "auto_hide_to_tray", true)?;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_skip_taskbar(auto_hide);
    }
    Ok(())
}

fn apply_initial_window_state(app: &AppHandle) {
    let is_startup = std::env::args().any(|arg| arg == "--startup");
    if is_startup {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    } else {
        let _ = show_main_window(app.clone());
    }
}

fn apply_startup_setting(app: &AppHandle, enabled: bool, as_admin: bool) -> AppResult<()> {
    let _ = app;
    #[cfg(target_os = "windows")]
    {
        let exe_path = std::env::current_exe()?;
        if as_admin {
            set_run_registry(false, &exe_path)?;
            set_startup_task(enabled, &exe_path)?;
        } else {
            set_startup_task(false, &exe_path)?;
            set_run_registry(enabled, &exe_path)?;
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, enabled, as_admin);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn wide(value: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

fn open_url_in_browser(url: &str) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        use winapi::um::{shellapi::ShellExecuteW, winuser::SW_SHOWNORMAL};

        let operation = wide("open");
        let file = wide(url);
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        } as isize;

        if result <= 32 {
            return Err(AppError::Message("打开浏览器失败".into()));
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = url;
        Err(AppError::Message("当前平台暂不支持打开外部链接".into()))
    }
}

#[cfg(target_os = "windows")]
fn windows_system_tool(name: &str) -> String {
    std::env::var("SystemRoot")
        .map(|root| format!(r"{root}\System32\{name}"))
        .unwrap_or_else(|_| name.to_string())
}

#[cfg(target_os = "windows")]
fn set_run_registry(enabled: bool, exe_path: &Path) -> AppResult<()> {
    let value = format!("\"{}\" --startup", exe_path.display());
    let status = if enabled {
        Command::new(windows_system_tool("reg.exe"))
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "SheepClip",
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    } else {
        Command::new(windows_system_tool("reg.exe"))
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "SheepClip",
                "/f",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    }
    .map_err(|error| AppError::Message(format!("配置开机自启失败: {error}")))?;

    if enabled && !status.success() {
        return Err(AppError::Message("配置开机自启失败，请检查系统权限".into()));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_startup_task(enabled: bool, exe_path: &Path) -> AppResult<()> {
    let task_name = "SheepClip";
    let _ = Command::new(windows_system_tool("schtasks.exe"))
        .args(["/Delete", "/TN", task_name, "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if !enabled {
        return Ok(());
    }

    let task_run = format!(r#""{}" --startup"#, exe_path.display());
    let task_name_arg = quote_windows_arg(task_name);
    let task_run_arg = quote_windows_arg(&task_run);
    let args = format!(
        "/Create /TN {} /SC ONLOGON /TR {} /RL HIGHEST /F",
        task_name_arg, task_run_arg
    );
    run_elevated(&windows_system_tool("schtasks.exe"), &args)
        .map_err(|error| AppError::Message(format!("配置管理员自启失败: {error}")))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn quote_windows_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

#[cfg(target_os = "windows")]
fn run_elevated(file: &str, args: &str) -> AppResult<()> {
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_HIDE;

    let operation = wide("runas");
    let file = wide(file);
    let args = wide(args);
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            args.as_ptr(),
            std::ptr::null(),
            SW_HIDE,
        )
    } as isize;

    if result <= 32 {
        return Err(AppError::Message("系统拒绝或取消了管理员授权".into()));
    }
    Ok(())
}

fn prepare_database(app: &AppHandle) -> AppResult<PathBuf> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Message(error.to_string()))?;
    fs::create_dir_all(&app_dir)?;

    let db_path = app_dir.join("sheepclip.db");
    let conn = Connection::open(&db_path)?;
    migrate_database(&conn)?;
    seed_defaults(&conn)?;
    Ok(db_path)
}

fn open_db(state: &AppState) -> AppResult<Connection> {
    let conn = Connection::open(&state.db_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conn)
}

fn migrate_database(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS clipboard_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL DEFAULT 'text',
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            preview TEXT NOT NULL,
            source_app TEXT,
            content_hash TEXT NOT NULL,
            created_at TEXT NOT NULL,
            used_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_clipboard_items_created_at
            ON clipboard_items(created_at DESC);

        CREATE UNIQUE INDEX IF NOT EXISTS idx_clipboard_items_hash
            ON clipboard_items(content_hash);

        CREATE TABLE IF NOT EXISTS quick_inputs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            tags TEXT NOT NULL DEFAULT '[]',
            prefix TEXT NOT NULL DEFAULT '',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_quick_inputs_prefix
            ON quick_inputs(prefix);

        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS quick_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS blacklist_apps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            process_name TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
        );
        "#,
    )?;
    ensure_column(
        conn,
        "clipboard_items",
        "kind",
        "TEXT NOT NULL DEFAULT 'text'",
    )?;
    ensure_column(
        conn,
        "clipboard_items",
        "preview",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(conn, "clipboard_items", "source_app", "TEXT")?;
    ensure_column(conn, "clipboard_items", "used_at", "TEXT")?;
    ensure_column(
        conn,
        "quick_inputs",
        "sort_order",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    conn.execute(
        "UPDATE clipboard_items SET preview = content WHERE preview = ''",
        [],
    )?;
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> AppResult<()> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|name| name == column);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

fn seed_defaults(conn: &Connection) -> AppResult<()> {
    let recommended = recommended_settings();
    let defaults = settings_entries(&recommended);

    for (key, value) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO app_settings(key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO app_settings(key, value) VALUES ('seed_content_version', '0')",
        [],
    )?;

    let tag_count: i64 = conn.query_row("SELECT COUNT(*) FROM quick_tags", [], |row| row.get(0))?;
    if tag_count == 0 {
        let timestamp = now();
        for tag in ["contact", "email", "address", "common"] {
            conn.execute(
                "INSERT OR IGNORE INTO quick_tags(name, created_at) VALUES (?1, ?2)",
                params![tag, timestamp],
            )?;
        }
    }

    let quick_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM quick_inputs", [], |row| row.get(0))?;
    if quick_count == 0 {
        let now = now();
        let quick_inputs = [
            ("13333333333", r#"["contact"]"#, "133"),
            ("xxxxxxxx@qq.com", r#"["email"]"#, "qq"),
            ("北京市朝阳区xxxxxxxx102", r#"["address"]"#, "北京"),
        ];
        for (index, (content, tags, prefix)) in quick_inputs.iter().enumerate() {
            conn.execute(
                "INSERT INTO quick_inputs(title, content, tags, prefix, sort_order, created_at, updated_at)
                 VALUES (?1, ?1, ?2, ?3, ?4, ?5, ?5)",
                params![content, tags, prefix, (index as i64) + 1, now],
            )?;
        }
    }

    conn.execute(
        "UPDATE quick_inputs SET sort_order = id WHERE sort_order = 0",
        [],
    )?;
    seed_initial_clipboard_items(conn)?;

    Ok(())
}

fn seed_initial_clipboard_items(conn: &Connection) -> AppResult<()> {
    let seed_version = read_i64(conn, "seed_content_version", 0)?;
    if seed_version >= 1 {
        return Ok(());
    }

    let clipboard_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))?;
    if clipboard_count == 0 {
        let timestamp = now();
        let source_app = "SheepClip 引导";
        let samples = [
            "这里会自动记录你复制过的文本、图片、文件和富文本。双击条目或按 Enter 可以快速复制。",
            "在任意输入框里快速输入 //，会弹出快捷输入窗口；输入关键词或标签可以筛选短语。",
            "双击 Alt 可以唤起主窗口，Tab 可以在剪贴板历史和快捷输入之间切换。",
        ];

        for content in samples {
            conn.execute(
                "INSERT OR IGNORE INTO clipboard_items(kind, title, content, preview, source_app, content_hash, created_at)
                 VALUES ('text', ?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    make_preview(content, 42),
                    content,
                    make_preview(content, 64),
                    source_app,
                    content_hash(&format!("seed:text:{content}")),
                    timestamp
                ],
            )?;
        }

        let logo_data = format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD.encode(include_bytes!("../icons/clipboard.png"))
        );
        conn.execute(
            "INSERT OR IGNORE INTO clipboard_items(kind, title, content, preview, source_app, content_hash, created_at)
             VALUES ('image', ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "SheepClip 图标示例",
                logo_data,
                "软件图标示例，图片条目会显示缩略图",
                source_app,
                image_clipboard_hash(&logo_data),
                timestamp
            ],
        )?;
    }

    write_setting(conn, "seed_content_version", "1")?;
    Ok(())
}

#[tauri::command]
fn list_clipboard_items(
    state: State<AppState>,
    query: Option<String>,
    limit: Option<i64>,
) -> AppResult<Vec<ClipboardItem>> {
    let conn = open_db(&state)?;
    let limit = limit.unwrap_or(100).clamp(1, 10000);
    let query = query.unwrap_or_default();
    let like = format!("%{}%", query);

    let mut statement = conn.prepare(
        r#"
        SELECT id, kind, title, content, preview, source_app, created_at, used_at
        FROM clipboard_items
        WHERE ?1 = '' OR title LIKE ?2 OR content LIKE ?2 OR preview LIKE ?2
        ORDER BY datetime(created_at) DESC
        LIMIT ?3
        "#,
    )?;

    let items = statement
        .query_map(params![query, like, limit], |row| {
            let kind: String = row.get(1)?;
            let content: String = row.get(3)?;
            let preview: String = row.get(4)?;
            let meta = clipboard_item_meta(&kind, &content, &preview);
            Ok(ClipboardItem {
                id: row.get(0)?,
                kind,
                title: row.get(2)?,
                content,
                preview,
                meta,
                source_app: row.get(5)?,
                created_at: row.get(6)?,
                used_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

#[tauri::command]
fn list_quick_inputs(
    state: State<AppState>,
    query: Option<String>,
    limit: Option<i64>,
) -> AppResult<Vec<QuickInput>> {
    let conn = open_db(&state)?;
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let query = query
        .unwrap_or_default()
        .trim()
        .trim_start_matches("//")
        .trim_start_matches('#')
        .to_string();
    let like = format!("%{}%", query);
    let tag_like = format!("%\"{}\"%", query);

    let mut statement = conn.prepare(
        r#"
        SELECT id, title, content, tags, prefix, sort_order, created_at, updated_at
        FROM quick_inputs
        WHERE ?1 = '' OR title LIKE ?2 OR content LIKE ?2 OR tags LIKE ?2 OR tags LIKE ?3 OR prefix LIKE ?2
        ORDER BY sort_order ASC, id ASC
        LIMIT ?4
        "#,
    )?;

    let items = statement
        .query_map(params![query, like, tag_like, limit], |row| {
            let tags_json: String = row.get(3)?;
            let tags = serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default();
            Ok(QuickInput {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tags,
                prefix: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

#[tauri::command]
fn list_tags(state: State<AppState>) -> AppResult<Vec<String>> {
    let conn = open_db(&state)?;
    let mut statement = conn.prepare(
        "SELECT name FROM quick_tags ORDER BY CASE name
            WHEN 'dev' THEN 1
            WHEN 'contact' THEN 2
            WHEN 'reply' THEN 3
            WHEN 'common' THEN 4
            ELSE 9
        END, name ASC",
    )?;
    let tags = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tags)
}

#[tauri::command]
fn add_tag(state: State<AppState>, name: String) -> AppResult<String> {
    let conn = open_db(&state)?;
    let normalized = normalize_tag(&name)?;
    conn.execute(
        "INSERT OR IGNORE INTO quick_tags(name, created_at) VALUES (?1, ?2)",
        params![normalized, now()],
    )?;
    Ok(normalized)
}

#[tauri::command]
fn delete_tag(state: State<AppState>, name: String) -> AppResult<()> {
    let conn = open_db(&state)?;
    let normalized = normalize_tag(&name)?;
    conn.execute(
        "DELETE FROM quick_tags WHERE name = ?1",
        params![normalized],
    )?;

    let mut statement = conn.prepare("SELECT id, tags FROM quick_inputs")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    for (id, tags_json) in rows {
        let mut tags = serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default();
        let original_len = tags.len();
        tags.retain(|tag| tag != &normalized);
        if tags.len() != original_len {
            let updated = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into());
            conn.execute(
                "UPDATE quick_inputs SET tags = ?1, updated_at = ?2 WHERE id = ?3",
                params![updated, now(), id],
            )?;
        }
    }

    Ok(())
}

#[tauri::command]
fn save_quick_input(
    app: AppHandle,
    state: State<AppState>,
    payload: SaveQuickInputPayload,
) -> AppResult<QuickInput> {
    let conn = open_db(&state)?;
    let content = payload.content.trim();
    if content.is_empty() {
        return Err(AppError::Message("内容不能为空".into()));
    }

    let tags = payload
        .tags
        .iter()
        .filter_map(|tag| normalize_tag(tag).ok())
        .collect::<Vec<_>>();
    for tag in &tags {
        conn.execute(
            "INSERT OR IGNORE INTO quick_tags(name, created_at) VALUES (?1, ?2)",
            params![tag, now()],
        )?;
    }
    let tags = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into());
    let title = make_title(content);
    let prefix = derive_prefix(content, 3);
    let timestamp = now();

    let id = if let Some(id) = payload.id {
        conn.execute(
            "UPDATE quick_inputs
             SET title = ?1, content = ?2, tags = ?3, prefix = ?4, updated_at = ?5
             WHERE id = ?6",
            params![title, content, tags, prefix, timestamp, id],
        )?;
        id
    } else {
        let sort_order = next_quick_sort_order(&conn)?;
        conn.execute(
            "INSERT INTO quick_inputs(title, content, tags, prefix, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![title, content, tags, prefix, sort_order, timestamp],
        )?;
        conn.last_insert_rowid()
    };

    let saved = get_quick_input(&conn, id)?;
    let _ = app.emit("quick-inputs-updated", ());
    Ok(saved)
}

#[tauri::command]
fn delete_quick_input(app: AppHandle, state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = open_db(&state)?;
    conn.execute("DELETE FROM quick_inputs WHERE id = ?1", params![id])?;
    let _ = app.emit("quick-inputs-updated", ());
    Ok(())
}

#[tauri::command]
fn reorder_quick_inputs(app: AppHandle, state: State<AppState>, ids: Vec<i64>) -> AppResult<()> {
    let mut conn = open_db(&state)?;
    let tx = conn.transaction()?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE quick_inputs SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![(index as i64) + 1, now(), id],
        )?;
    }
    tx.commit()?;
    let _ = app.emit("quick-inputs-updated", ());
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> AppResult<AppSettings> {
    let conn = open_db(&state)?;
    Ok(read_settings(&conn)?)
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: AppSettings,
) -> AppResult<AppSettings> {
    let conn = open_db(&state)?;
    let previous = read_settings(&conn)?;
    let sanitized = sanitize_settings(settings);

    validate_shortcut_settings(&sanitized)?;

    if previous.launch_at_startup != sanitized.launch_at_startup
        || previous.launch_as_admin != sanitized.launch_as_admin
    {
        apply_startup_setting(&app, sanitized.launch_at_startup, sanitized.launch_as_admin)?;
    }

    write_settings(&conn, &sanitized)?;
    trim_history(&conn, sanitized.history_limit)?;
    apply_runtime_settings(&app, &state, &sanitized);

    Ok(sanitized)
}

#[tauri::command]
fn reset_settings(app: AppHandle, state: State<AppState>) -> AppResult<AppSettings> {
    let conn = open_db(&state)?;
    let previous = read_settings(&conn)?;
    let recommended = recommended_settings();

    if previous.launch_at_startup != recommended.launch_at_startup
        || previous.launch_as_admin != recommended.launch_as_admin
    {
        apply_startup_setting(
            &app,
            recommended.launch_at_startup,
            recommended.launch_as_admin,
        )?;
    }

    write_settings(&conn, &recommended)?;
    trim_history(&conn, recommended.history_limit)?;
    apply_runtime_settings(&app, &state, &recommended);
    Ok(recommended)
}

#[tauri::command]
fn mark_main_pointer_operation(state: State<AppState>) -> AppResult<()> {
    if let Ok(mut until) = state.main_pointer_operation_until.lock() {
        *until = Some(Instant::now() + Duration::from_millis(1200));
    }
    Ok(())
}

#[tauri::command]
fn open_external_url(url: String) -> AppResult<()> {
    if !url.starts_with("https://github.com/passheep/SheepClip") {
        return Err(AppError::Message("不支持打开该链接".into()));
    }
    open_url_in_browser(&url)
}

#[tauri::command]
fn delete_clipboard_item(app: AppHandle, state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = open_db(&state)?;
    conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
    let _ = app.emit("clipboard-history-updated", ());
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
fn copy_item_to_clipboard(
    app: AppHandle,
    state: State<AppState>,
    source: String,
    id: i64,
    paste: Option<bool>,
) -> AppResult<()> {
    let conn = open_db(&state)?;
    let (kind, content) = if source == "quick" {
        let content = conn
            .query_row(
                "SELECT content FROM quick_inputs WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::Message("未找到要复制的内容".into()))?;
        ("text".to_string(), content)
    } else {
        conn.query_row(
            "SELECT kind, content FROM clipboard_items WHERE id = ?1",
            params![id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::Message("未找到要复制的内容".into()))?
    };

    let content = normalize_content_for_clipboard(&kind, &content);
    let mut clipboard = Clipboard::new()?;
    set_clipboard_by_kind(&mut clipboard, &kind, &content)?;
    if let Ok(mut last) = state.last_text.lock() {
        *last = clipboard_skip_marker(&kind, &content);
    }

    let timestamp = now();
    if source == "quick" {
        conn.execute(
            "UPDATE quick_inputs SET updated_at = ?1 WHERE id = ?2",
            params![timestamp, id],
        )?;
    } else {
        conn.execute(
            "UPDATE clipboard_items SET used_at = ?1 WHERE id = ?2",
            params![timestamp, id],
        )?;
    }
    let settings = read_settings(&conn)?;
    apply_activation_options(
        &app,
        &state,
        &conn,
        source.as_str(),
        id,
        &settings,
        paste.unwrap_or(false),
        0,
    )?;

    Ok(())
}

#[tauri::command]
fn capture_current_clipboard(state: State<AppState>) -> AppResult<()> {
    let mut clipboard = Clipboard::new()?;
    if let Some(snapshot) = read_clipboard_snapshot(&mut clipboard) {
        insert_clipboard_snapshot(&state.db_path, snapshot)?;
    }
    Ok(())
}

#[tauri::command]
fn clear_clipboard_history(state: State<AppState>) -> AppResult<()> {
    let conn = open_db(&state)?;
    conn.execute("DELETE FROM clipboard_items", [])?;
    Ok(())
}

#[tauri::command]
fn add_clipboard_item_to_quick_input(
    app: AppHandle,
    state: State<AppState>,
    id: i64,
) -> AppResult<QuickInput> {
    let conn = open_db(&state)?;
    let content = conn
        .query_row(
            "SELECT content FROM clipboard_items WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| AppError::Message("未找到剪贴板内容".into()))?;
    let title = make_title(&content);
    let prefix = derive_prefix(&content, 3);
    let timestamp = now();

    conn.execute(
        "INSERT INTO quick_inputs(title, content, tags, prefix, sort_order, created_at, updated_at)
         VALUES (?1, ?2, '[]', ?3, ?4, ?5, ?5)",
        params![
            title,
            content,
            prefix,
            next_quick_sort_order(&conn)?,
            timestamp
        ],
    )?;

    let saved = get_quick_input(&conn, conn.last_insert_rowid())?;
    let _ = app.emit("quick-inputs-updated", ());
    Ok(saved)
}

#[tauri::command]
fn show_main_window(app: AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Message("主窗口不存在".into()))?;
    let _ = window.unminimize();
    window
        .show()
        .map_err(|error| AppError::Message(error.to_string()))?;
    let _ = window.unminimize();
    window
        .set_focus()
        .map_err(|error| AppError::Message(error.to_string()))?;
    let _ = app.emit("main-window-shown", ());
    Ok(())
}

#[tauri::command]
fn hide_main_window(app: AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Message("主窗口不存在".into()))?;
    window
        .hide()
        .map_err(|error| AppError::Message(error.to_string()))?;
    Ok(())
}

#[tauri::command]
fn minimize_main_window(app: AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Message("主窗口不存在".into()))?;
    window
        .minimize()
        .map_err(|error| AppError::Message(error.to_string()))?;
    Ok(())
}

#[tauri::command]
fn set_main_window_always_on_top(app: AppHandle, pinned: bool) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Message("主窗口不存在".into()))?;
    window
        .set_always_on_top(pinned)
        .map_err(|error| AppError::Message(error.to_string()))?;
    Ok(())
}

#[tauri::command]
fn exit_app(app: AppHandle) -> AppResult<()> {
    app.exit(0);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
fn paste_item_from_main_window(
    app: AppHandle,
    state: State<AppState>,
    source: String,
    id: i64,
) -> AppResult<()> {
    if id <= 0 {
        return Ok(());
    }

    let conn = open_db(&state)?;
    let (kind, content) = if source == "clipboard" {
        conn.query_row(
            "SELECT kind, content FROM clipboard_items WHERE id = ?1",
            params![id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::Message("未找到剪贴板内容".into()))?
    } else {
        let content = conn
            .query_row(
                "SELECT content FROM quick_inputs WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::Message("未找到快捷输入".into()))?;
        ("text".into(), content)
    };

    let content = normalize_content_for_clipboard(&kind, &content);
    let mut clipboard = Clipboard::new()?;
    set_clipboard_by_kind(&mut clipboard, &kind, &content)?;
    if let Ok(mut last) = state.last_text.lock() {
        *last = clipboard_skip_marker(&kind, &content);
    }

    if source == "clipboard" {
        conn.execute(
            "UPDATE clipboard_items SET used_at = ?1 WHERE id = ?2",
            params![now(), id],
        )?;
    } else {
        conn.execute(
            "UPDATE quick_inputs SET updated_at = ?1 WHERE id = ?2",
            params![now(), id],
        )?;
    }
    let settings = read_settings(&conn)?;
    apply_activation_options(&app, &state, &conn, source.as_str(), id, &settings, true, 0)?;

    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
fn paste_quick_input_from_floating(
    app: AppHandle,
    state: State<AppState>,
    id: i64,
    backspace_count: usize,
    source: Option<String>,
) -> AppResult<()> {
    if id <= 0 {
        hide_floating_window(&app);
        reset_inline_state(&state.inline_state);
        return Ok(());
    }

    let conn = open_db(&state)?;
    let source = source.unwrap_or_else(|| "quick".into());
    let (kind, content) = if source == "clipboard" {
        conn.query_row(
            "SELECT kind, content FROM clipboard_items WHERE id = ?1",
            params![id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::Message("未找到剪贴板内容".into()))?
    } else {
        let content = conn
            .query_row(
                "SELECT content FROM quick_inputs WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::Message("未找到快捷输入".into()))?;
        ("text".into(), content)
    };

    let content = normalize_content_for_clipboard(&kind, &content);
    let mut clipboard = Clipboard::new()?;
    set_clipboard_by_kind(&mut clipboard, &kind, &content)?;
    if let Ok(mut last) = state.last_text.lock() {
        *last = clipboard_skip_marker(&kind, &content);
    }

    if source == "clipboard" {
        conn.execute(
            "UPDATE clipboard_items SET used_at = ?1 WHERE id = ?2",
            params![now(), id],
        )?;
    } else {
        conn.execute(
            "UPDATE quick_inputs SET updated_at = ?1 WHERE id = ?2",
            params![now(), id],
        )?;
    }
    let settings = read_settings(&conn)?;
    hide_floating_window(&app);
    apply_activation_options(
        &app,
        &state,
        &conn,
        source.as_str(),
        id,
        &settings,
        true,
        backspace_count,
    )?;

    reset_inline_state(&state.inline_state);
    Ok(())
}

fn get_quick_input(conn: &Connection, id: i64) -> AppResult<QuickInput> {
    conn.query_row(
        "SELECT id, title, content, tags, prefix, sort_order, created_at, updated_at
         FROM quick_inputs WHERE id = ?1",
        params![id],
        |row| {
            let tags_json: String = row.get(3)?;
            let tags = serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default();
            Ok(QuickInput {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tags,
                prefix: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .map_err(AppError::from)
}

fn apply_activation_options(
    app: &AppHandle,
    state: &AppState,
    conn: &Connection,
    source: &str,
    id: i64,
    settings: &AppSettings,
    force_paste: bool,
    backspace_count: usize,
) -> AppResult<()> {
    apply_activation_order(conn, source, id, settings)?;

    let should_focus_previous = settings.focus_previous_after_activation
        || settings.paste_after_activation
        || force_paste;
    let target_hwnd = if should_focus_previous {
        inline_target_window(&state.inline_state)
    } else {
        0
    };

    if settings.close_after_activation {
        if let Some(window) = app.get_webview_window("main") {
            if !window.is_always_on_top().unwrap_or(false) {
                if settings.auto_hide_to_tray {
                    let _ = window.hide();
                } else {
                    let _ = window.minimize();
                }
            }
        }
    }

    if should_focus_previous && target_hwnd != 0 {
        thread::sleep(Duration::from_millis(80));
        focus_window(target_hwnd);
    }

    if settings.paste_after_activation || force_paste {
        thread::sleep(Duration::from_millis(80));
        paste_with_backspace(backspace_count)?;
    }

    Ok(())
}

fn apply_activation_order(
    conn: &Connection,
    source: &str,
    id: i64,
    settings: &AppSettings,
) -> AppResult<()> {
    if !settings.move_activated_to_top {
        return Ok(());
    }

    let timestamp = now();
    if source == "clipboard" {
        conn.execute(
            "UPDATE clipboard_items SET created_at = ?1, used_at = ?1 WHERE id = ?2",
            params![timestamp, id],
        )?;
    } else {
        conn.execute(
            "UPDATE quick_inputs
             SET sort_order = 0, updated_at = ?1
             WHERE id = ?2",
            params![timestamp, id],
        )?;
        normalize_quick_sort_order(conn)?;
    }

    Ok(())
}

fn normalize_quick_sort_order(conn: &Connection) -> AppResult<()> {
    let ids = conn
        .prepare("SELECT id FROM quick_inputs ORDER BY sort_order ASC, id ASC")?
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for (index, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE quick_inputs SET sort_order = ?1 WHERE id = ?2",
            params![(index as i64) + 1, id],
        )?;
    }
    Ok(())
}

fn read_settings(conn: &Connection) -> AppResult<AppSettings> {
    Ok(AppSettings {
        history_limit: read_i64(conn, "history_limit", 2000)?.clamp(50, 10000),
        main_hotkey: read_text(conn, "main_hotkey", "Alt")?,
        main_hotkey_enabled: read_bool(conn, "main_hotkey_enabled", true)?,
        inline_trigger: read_text(conn, "inline_trigger", "//")?,
        inline_trigger_enabled: read_bool(conn, "inline_trigger_enabled", true)?,
        launch_at_startup: read_bool(conn, "launch_at_startup", false)?,
        launch_as_admin: read_bool(conn, "launch_as_admin", false)?,
        auto_hide_to_tray: read_bool(conn, "auto_hide_to_tray", false)?,
        confirm_close_to_tray: read_bool(conn, "confirm_close_to_tray", true)?,
        enter_paste_to_active: read_bool(conn, "enter_paste_to_active", false)?,
        hide_on_blur: read_bool(conn, "hide_on_blur", false)?,
        confirm_exit: read_bool(conn, "confirm_exit", true)?,
        move_activated_to_top: read_bool(conn, "move_activated_to_top", true)?,
        close_after_activation: read_bool(conn, "close_after_activation", true)?,
        focus_previous_after_activation: read_bool(
            conn,
            "focus_previous_after_activation",
            true,
        )?,
        paste_after_activation: read_bool(conn, "paste_after_activation", false)?,
        onboarding_completed: read_bool(conn, "onboarding_completed", false)?,
    })
}

fn recommended_settings() -> AppSettings {
    AppSettings {
        history_limit: 2000,
        main_hotkey: "Alt".into(),
        main_hotkey_enabled: true,
        inline_trigger: "//".into(),
        inline_trigger_enabled: true,
        launch_at_startup: false,
        launch_as_admin: false,
        auto_hide_to_tray: false,
        confirm_close_to_tray: true,
        enter_paste_to_active: false,
        hide_on_blur: false,
        confirm_exit: true,
        move_activated_to_top: true,
        close_after_activation: true,
        focus_previous_after_activation: true,
        paste_after_activation: false,
        onboarding_completed: false,
    }
}

fn sanitize_settings(settings: AppSettings) -> AppSettings {
    AppSettings {
        history_limit: settings.history_limit.clamp(50, 10000),
        main_hotkey: if settings.main_hotkey == "Ctrl" {
            "Ctrl".into()
        } else {
            "Alt".into()
        },
        inline_trigger: if settings.inline_trigger.trim().is_empty() {
            "//".into()
        } else {
            settings.inline_trigger.trim().chars().take(6).collect()
        },
        main_hotkey_enabled: settings.main_hotkey_enabled,
        inline_trigger_enabled: settings.inline_trigger_enabled,
        launch_at_startup: settings.launch_at_startup,
        launch_as_admin: settings.launch_as_admin,
        auto_hide_to_tray: settings.auto_hide_to_tray,
        confirm_close_to_tray: settings.confirm_close_to_tray,
        enter_paste_to_active: settings.enter_paste_to_active,
        hide_on_blur: settings.hide_on_blur,
        confirm_exit: settings.confirm_exit,
        move_activated_to_top: settings.move_activated_to_top,
        close_after_activation: settings.close_after_activation,
        focus_previous_after_activation: settings.focus_previous_after_activation,
        paste_after_activation: settings.paste_after_activation,
        onboarding_completed: settings.onboarding_completed,
    }
}

fn settings_entries(settings: &AppSettings) -> Vec<(&'static str, String)> {
    vec![
        ("history_limit", settings.history_limit.to_string()),
        ("main_hotkey", settings.main_hotkey.clone()),
        (
            "main_hotkey_enabled",
            bool_text(settings.main_hotkey_enabled).into(),
        ),
        ("inline_trigger", settings.inline_trigger.clone()),
        (
            "inline_trigger_enabled",
            bool_text(settings.inline_trigger_enabled).into(),
        ),
        (
            "launch_at_startup",
            bool_text(settings.launch_at_startup).into(),
        ),
        ("launch_as_admin", bool_text(settings.launch_as_admin).into()),
        (
            "auto_hide_to_tray",
            bool_text(settings.auto_hide_to_tray).into(),
        ),
        (
            "confirm_close_to_tray",
            bool_text(settings.confirm_close_to_tray).into(),
        ),
        (
            "enter_paste_to_active",
            bool_text(settings.enter_paste_to_active).into(),
        ),
        ("hide_on_blur", bool_text(settings.hide_on_blur).into()),
        ("confirm_exit", bool_text(settings.confirm_exit).into()),
        (
            "move_activated_to_top",
            bool_text(settings.move_activated_to_top).into(),
        ),
        (
            "close_after_activation",
            bool_text(settings.close_after_activation).into(),
        ),
        (
            "focus_previous_after_activation",
            bool_text(settings.focus_previous_after_activation).into(),
        ),
        (
            "paste_after_activation",
            bool_text(settings.paste_after_activation).into(),
        ),
        (
            "onboarding_completed",
            bool_text(settings.onboarding_completed).into(),
        ),
    ]
}

fn write_settings(conn: &Connection, settings: &AppSettings) -> AppResult<()> {
    for (key, value) in settings_entries(settings) {
        write_setting(conn, key, &value)?;
    }
    Ok(())
}

fn apply_runtime_settings(app: &AppHandle, state: &AppState, settings: &AppSettings) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_skip_taskbar(settings.auto_hide_to_tray);
    }

    if let Ok(mut config) = state.hotkey_config.lock() {
        config.enabled = settings.main_hotkey_enabled;
        config.main_hotkey = MainHotkey::from_setting(&settings.main_hotkey);
    }
    if let Ok(mut inline) = state.inline_state.lock() {
        inline.enabled = settings.inline_trigger_enabled;
        inline.trigger = settings.inline_trigger.clone();
        inline.active = false;
        inline.query.clear();
        inline.selected_index = 0;
        inline.item_ids.clear();
        inline.last_slash = None;
        inline.target_hwnd = 0;
    }
}

fn validate_shortcut_settings(settings: &AppSettings) -> AppResult<()> {
    if settings.main_hotkey_enabled && settings.inline_trigger_enabled {
        let trigger = settings.inline_trigger.trim();
        if trigger.eq_ignore_ascii_case(&settings.main_hotkey) {
            return Err(AppError::Message(
                "快捷键冲突：主窗口双击键不能与行内触发符相同".into(),
            ));
        }
    }

    if settings.inline_trigger_enabled {
        let trigger = settings.inline_trigger.trim();
        if trigger.chars().count() < 2 {
            return Err(AppError::Message(
                "快捷键冲突：行内触发符至少需要 2 个字符，避免正常输入时误触发".into(),
            ));
        }
        if trigger.chars().any(|ch| ch.is_alphanumeric()) {
            return Err(AppError::Message(
                "快捷键冲突：行内触发符建议只使用符号，不要使用字母或数字".into(),
            ));
        }
        let high_risk = ["//", "##", ";;", "..", ",,", "\\\\"];
        if !high_risk.contains(&trigger)
            && trigger
                .chars()
                .collect::<Vec<_>>()
                .windows(2)
                .all(|pair| pair[0] != pair[1])
        {
            return Err(AppError::Message(
                "快捷键冲突：行内触发符建议使用连续重复符号，如 // 或 ##".into(),
            ));
        }
    }

    Ok(())
}

fn write_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn read_text(conn: &Connection, key: &str, fallback: &str) -> AppResult<String> {
    Ok(conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .unwrap_or_else(|| fallback.into()))
}

fn read_i64(conn: &Connection, key: &str, fallback: i64) -> AppResult<i64> {
    Ok(read_text(conn, key, &fallback.to_string())?
        .parse::<i64>()
        .unwrap_or(fallback))
}

fn read_bool(conn: &Connection, key: &str, fallback: bool) -> AppResult<bool> {
    Ok(match read_text(conn, key, bool_text(fallback))?.as_str() {
        "true" | "1" | "yes" => true,
        "false" | "0" | "no" => false,
        _ => fallback,
    })
}

fn start_clipboard_watcher(app: AppHandle, db_path: PathBuf, last_text: Arc<Mutex<String>>) {
    thread::spawn(move || loop {
        let mut sleep_ms = 450;
        if let Ok(mut clipboard) = Clipboard::new() {
            if let Some(snapshot) = read_clipboard_snapshot(&mut clipboard) {
                if !snapshot.content.trim().is_empty() {
                    let should_insert = last_text
                        .lock()
                        .map(|mut last| {
                            if *last == snapshot.hash_input {
                                false
                            } else {
                                *last = snapshot.hash_input.clone();
                                true
                            }
                        })
                        .unwrap_or(false);

                    if should_insert {
                        if insert_clipboard_snapshot(&db_path, snapshot).is_ok() {
                            let _ = app.emit("clipboard-history-updated", ());
                        }
                    }
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            sleep_ms = if clipboard_sequence_number().is_some() {
                250
            } else {
                sleep_ms
            };
        }
        thread::sleep(Duration::from_millis(sleep_ms));
    });
}

fn read_clipboard_snapshot(clipboard: &mut Clipboard) -> Option<ClipboardSnapshot> {
    let image_snapshot = clipboard.get_image().ok().and_then(|image| {
        let data_url = image_to_data_url(&image)?;
        let title = format!("图片 {}x{}", image.width, image.height);
        Some(ClipboardSnapshot {
            kind: "image".into(),
            title: title.clone(),
            preview: title,
            hash_input: image_clipboard_hash(&data_url),
            content: data_url,
        })
    });

    if let Ok(html) = clipboard.get().html() {
        let trimmed = html.trim();
        let content = enrich_rich_text_html(trimmed, image_snapshot.as_ref());
        if is_meaningful_rich_text(&content, clipboard) {
            return Some(ClipboardSnapshot {
                kind: "rich_text".into(),
                title: make_title(&strip_html_tags(&content)),
                content: content.clone(),
                preview: make_preview(&strip_html_tags(&content), 160),
                hash_input: format!("html:{content}"),
            });
        }
    }

    if let Some(snapshot) = image_snapshot {
        return Some(snapshot);
    }

    #[cfg(target_os = "windows")]
    if let Some(snapshot) = windows_bitmap_snapshot() {
        return Some(snapshot);
    }

    if let Ok(files) = clipboard.get().file_list() {
        if !files.is_empty() {
            if files.len() == 1 {
                if let Some(snapshot) = image_file_snapshot(&files[0]) {
                    return Some(snapshot);
                }
            }
            let content = files
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join("\n");
            let title = if files.len() == 1 {
                files[0]
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "文件".into())
            } else {
                format!("{} 个文件/文件夹", files.len())
            };
            return Some(ClipboardSnapshot {
                kind: "file".into(),
                title,
                preview: make_preview(&content, 160),
                hash_input: format!("file:{content}"),
                content,
            });
        }
    }

    if let Ok(text) = clipboard.get_text() {
        let normalized = text.trim().to_string();
        if !normalized.is_empty() {
            return Some(ClipboardSnapshot {
                kind: "text".into(),
                title: make_title(&normalized),
                preview: make_preview(&normalized, 160),
                hash_input: format!("text:{normalized}"),
                content: normalized,
            });
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn clipboard_sequence_number() -> Option<u32> {
    clipboard_win::raw::seq_num().map(|number| number.get())
}

#[cfg(target_os = "windows")]
fn windows_bitmap_snapshot() -> Option<ClipboardSnapshot> {
    use clipboard_win::{formats, Clipboard as WinClipboard, Getter};

    if !clipboard_win::is_format_avail(formats::CF_BITMAP) {
        return None;
    }

    let _clipboard = WinClipboard::new_attempts(5).ok()?;
    let mut bytes = Vec::new();
    formats::Bitmap.read_clipboard(&mut bytes).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let image = image::load_from_memory_with_format(&bytes, ImageFormat::Bmp).ok()?;
    let (width, height) = (image.width(), image.height());
    let data_url = dynamic_image_to_png_data_url(image).ok()?;
    let title = format!("图片 {}x{}", width, height);
    Some(ClipboardSnapshot {
        kind: "image".into(),
        title: title.clone(),
        preview: title,
        hash_input: image_clipboard_hash(&data_url),
        content: data_url,
    })
}

fn is_meaningful_rich_text(html: &str, clipboard: &mut Clipboard) -> bool {
    if html.is_empty() {
        return false;
    }

    let plain_from_html = strip_html_tags(html);
    if plain_from_html.is_empty() {
        return false;
    }

    if let Ok(text) = clipboard.get_text() {
        if normalize_plain_text(&plain_from_html) == normalize_plain_text(&text)
            && !html_has_rich_structure(html)
        {
            return false;
        }
    }

    html_has_rich_structure(html)
}

fn enrich_rich_text_html(html: &str, image_snapshot: Option<&ClipboardSnapshot>) -> String {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let content = inline_rich_text_images(trimmed, image_snapshot);
    if html_contains_image(&content) {
        return content;
    }

    if let Some(snapshot) = image_snapshot {
        if snapshot.content.starts_with("data:image/") {
            return format!(
                "{}<p><img src=\"{}\" alt=\"{}\" /></p>",
                content,
                snapshot.content, snapshot.title
            );
        }
    }

    content
}

fn html_contains_image(html: &str) -> bool {
    html.to_ascii_lowercase().contains("<img")
}

fn inline_rich_text_images(html: &str, fallback_image: Option<&ClipboardSnapshot>) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0usize;
    let mut fallback_used = false;
    let mut changed = false;

    while let Some(relative_start) = find_ascii_case_insensitive(&html[cursor..], "<img") {
        let start = cursor + relative_start;
        let Some(end) = find_html_tag_end(html, start) else {
            break;
        };
        output.push_str(&html[cursor..start]);
        let tag = &html[start..end];
        let (rewritten, used_fallback) = rewrite_img_tag(tag, fallback_image, fallback_used);
        if rewritten != tag {
            changed = true;
        }
        fallback_used = fallback_used || used_fallback;
        output.push_str(&rewritten);
        cursor = end;
    }

    if !changed {
        return html.to_string();
    }
    output.push_str(&html[cursor..]);
    output
}

fn rewrite_img_tag(
    tag: &str,
    fallback_image: Option<&ClipboardSnapshot>,
    fallback_used: bool,
) -> (String, bool) {
    let src_range = find_html_attr(tag, "src");
    let data_src_range = find_html_attr(tag, "data-src");
    let data_original_range = find_html_attr(tag, "data-original");
    let candidate_range = src_range
        .as_ref()
        .or(data_src_range.as_ref())
        .or(data_original_range.as_ref());

    let current_src = candidate_range
        .and_then(|range| match (range.value_start, range.value_end) {
            (Some(start), Some(end)) => Some(decode_html_attr_value(&tag[start..end])),
            _ => None,
        })
        .unwrap_or_default();

    let mut used_fallback = false;
    let next_src = if current_src.trim_start().starts_with("data:image/") {
        None
    } else {
        image_src_to_data_url(&current_src).or_else(|| {
            if fallback_used {
                return None;
            }
            let fallback = fallback_image?;
            if fallback.content.starts_with("data:image/") {
                used_fallback = true;
                Some(fallback.content.clone())
            } else {
                None
            }
        })
    };

    let Some(next_src) = next_src else {
        return (tag.to_string(), false);
    };

    let mut rewritten = if let Some(range) = src_range {
        match (range.value_start, range.value_end) {
            (Some(start), Some(end)) => {
                let mut next_tag = String::with_capacity(tag.len() + next_src.len());
                next_tag.push_str(&tag_ref(tag, 0, start));
                next_tag.push_str(&html_escape_attr(&next_src));
                next_tag.push_str(&tag_ref(tag, end, tag.len()));
                next_tag
            }
            _ => insert_img_src_attr(tag, &next_src),
        }
    } else {
        insert_img_src_attr(tag, &next_src)
    };
    rewritten = remove_html_attr(&rewritten, "srcset");
    (rewritten, used_fallback)
}

fn tag_ref(value: &str, start: usize, end: usize) -> String {
    value.get(start..end).unwrap_or_default().to_string()
}

fn insert_img_src_attr(tag: &str, src: &str) -> String {
    let insert_at = tag.rfind("/>").unwrap_or_else(|| tag.rfind('>').unwrap_or(tag.len()));
    let mut output = String::with_capacity(tag.len() + src.len() + 8);
    output.push_str(&tag[..insert_at]);
    output.push_str(" src=\"");
    output.push_str(&html_escape_attr(src));
    output.push('"');
    output.push_str(&tag[insert_at..]);
    output
}

fn remove_html_attr(tag: &str, attr_name: &str) -> String {
    let mut output = tag.to_string();
    while let Some(range) = find_html_attr(&output, attr_name) {
        output.replace_range(range.full_start..range.full_end, "");
    }
    output
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

fn find_html_tag_end(html: &str, start: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (offset, byte) in html.as_bytes().iter().enumerate().skip(start) {
        match quote {
            Some(current) if *byte == current => quote = None,
            Some(_) => {}
            None if *byte == b'\'' || *byte == b'"' => quote = Some(*byte),
            None if *byte == b'>' => return Some(offset + 1),
            None => {}
        }
    }
    None
}

fn find_html_attr(tag: &str, attr_name: &str) -> Option<HtmlAttrRange> {
    let bytes = tag.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && !is_attr_name_char(bytes[i]) {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let full_start = i;
        let name_start = i;
        while i < bytes.len() && is_attr_name_char(bytes[i]) {
            i += 1;
        }
        let name_end = i;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        let mut value_start = None;
        let mut value_end = None;
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'\'' || bytes[i] == b'"') {
                let quote = bytes[i];
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                value_start = Some(start);
                value_end = Some(i.min(bytes.len()));
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                    i += 1;
                }
                value_start = Some(start);
                value_end = Some(i);
            }
        }

        let full_end = i;
        if tag[name_start..name_end].eq_ignore_ascii_case(attr_name) {
            return Some(HtmlAttrRange {
                full_start,
                full_end,
                value_start,
                value_end,
            });
        }
    }
    None
}

fn is_attr_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':')
}

fn image_src_to_data_url(src: &str) -> Option<String> {
    let src = decode_html_attr_value(src);
    let src = src.trim().trim_matches('\0');
    if src.to_ascii_lowercase().starts_with("data:image/") {
        return Some(src.to_string());
    }
    let path = image_src_to_local_path(src)?;
    image_path_to_data_url(&path)
}

fn image_src_to_local_path(src: &str) -> Option<PathBuf> {
    let lower = src.to_ascii_lowercase();
    let path_text = if lower.starts_with("file://localhost/") {
        percent_decode_path(&src["file://localhost/".len()..])
    } else if lower.starts_with("file:///") {
        percent_decode_path(&src["file:///".len()..])
    } else if lower.starts_with("file://") {
        percent_decode_path(&src["file://".len()..])
    } else if lower.starts_with("file:/") {
        percent_decode_path(&src["file:/".len()..])
    } else {
        src.to_string()
    };
    let normalized = normalize_local_image_path(&path_text);
    let path = PathBuf::from(normalized);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn normalize_local_image_path(value: &str) -> String {
    let mut path = value.trim().to_string();
    if path.starts_with('/') && path.len() > 2 && path.as_bytes().get(2) == Some(&b':') {
        path.remove(0);
    }
    path
}

fn image_path_to_data_url(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;
    dynamic_image_to_png_data_url(image).ok()
}

fn percent_decode_path(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_value(bytes[index + 1]), hex_value(bytes[index + 2])) {
                output.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).to_string()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode_html_attr_value(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn html_escape_attr(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}

fn start_global_keyboard_listener(
    app: AppHandle,
    db_path: PathBuf,
    config: Arc<Mutex<HotkeyConfig>>,
    inline_state: Arc<Mutex<InlineState>>,
) {
    thread::spawn(move || {
        let mut last_hit: Option<Instant> = None;

        let callback = move |event: Event| {
            if let EventType::KeyPress(key) = event.event_type {
                handle_main_hotkey(&app, &config, &inline_state, key, &mut last_hit);
                handle_inline_key(&app, &db_path, &inline_state, key);
            }
        };

        if let Err(error) = listen(callback) {
            eprintln!("global keyboard listener stopped: {error:?}");
        }
    });
}

fn handle_main_hotkey(
    app: &AppHandle,
    config: &Arc<Mutex<HotkeyConfig>>,
    inline_state: &Arc<Mutex<InlineState>>,
    key: Key,
    last_hit: &mut Option<Instant>,
) {
    let hotkey = config
        .lock()
        .map(|config| {
            if config.enabled {
                Some(config.main_hotkey)
            } else {
                None
            }
        })
        .unwrap_or(None);

    let Some(hotkey) = hotkey else {
        return;
    };

    if !hotkey.matches_key(key) {
        return;
    }

    let now = Instant::now();
    let is_double_hit = last_hit
        .map(|last| now.duration_since(last) <= Duration::from_millis(420))
        .unwrap_or(false);
    *last_hit = Some(now);

    if is_double_hit {
        *last_hit = None;
        if let Ok(mut state) = inline_state.lock() {
            state.target_hwnd = current_foreground_window();
        }
        let _ = show_main_window(app.clone());
    }
}

fn handle_inline_key(
    app: &AppHandle,
    db_path: &Path,
    inline_state: &Arc<Mutex<InlineState>>,
    key: Key,
) {
    let enabled = inline_state
        .lock()
        .map(|state| state.enabled)
        .unwrap_or(false);
    if !enabled {
        return;
    }

    let active = inline_state
        .lock()
        .map(|state| state.active)
        .unwrap_or(false);
    if active {
        if key == Key::Escape {
            hide_floating_window(app);
            reset_inline_state(inline_state);
        }
        return;
    }

    if key != Key::Slash {
        if let Ok(mut state) = inline_state.lock() {
            state.last_slash = None;
        }
        return;
    }

    let payload = {
        let mut state = match inline_state.lock() {
            Ok(state) => state,
            Err(_) => return,
        };

        let now = Instant::now();
        let is_double_slash = state
            .last_slash
            .map(|last| now.duration_since(last) <= Duration::from_millis(450))
            .unwrap_or(false);
        state.last_slash = Some(now);

        if is_double_slash {
            state.active = true;
            state.query.clear();
            state.selected_index = 0;
            state.item_ids = matching_quick_input_ids(db_path, "").unwrap_or_default();
            state.target_hwnd = current_foreground_window();
            Some(FloatingPayload {
                query: String::new(),
                trigger: state.trigger.clone(),
                selected_index: 0,
            })
        } else {
            None
        }
    };

    if let Some(payload) = payload {
        let app_handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(140));
            show_floating_window(&app_handle, payload);
        });
    }
}

fn insert_clipboard_snapshot(db_path: &Path, snapshot: ClipboardSnapshot) -> AppResult<()> {
    let conn = Connection::open(db_path)?;
    migrate_database(&conn)?;
    seed_defaults(&conn)?;

    let hash = content_hash(&snapshot.hash_input);
    let timestamp = now();
    let source_app = active_app_name();

    conn.execute(
        "INSERT OR IGNORE INTO clipboard_items(kind, title, content, preview, source_app, content_hash, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            snapshot.kind,
            snapshot.title,
            snapshot.content,
            snapshot.preview,
            source_app,
            hash,
            timestamp
        ],
    )?;

    let settings = read_settings(&conn)?;
    trim_history(&conn, settings.history_limit)?;
    Ok(())
}

fn trim_history(conn: &Connection, limit: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM clipboard_items
         WHERE id NOT IN (
            SELECT id FROM clipboard_items
            ORDER BY datetime(created_at) DESC
            LIMIT ?1
         )",
        params![limit],
    )?;
    Ok(())
}

fn matching_quick_input_ids(db_path: &Path, query: &str) -> AppResult<Vec<i64>> {
    let conn = Connection::open(db_path)?;
    let query = query
        .trim()
        .trim_start_matches("//")
        .trim_start_matches('#')
        .to_string();
    let like = format!("%{}%", query);
    let tag_like = format!("%\"{}\"%", query);
    let mut statement = conn.prepare(
        "SELECT id FROM quick_inputs
         WHERE ?1 = '' OR title LIKE ?2 OR content LIKE ?2 OR tags LIKE ?2 OR tags LIKE ?3 OR prefix LIKE ?2
         ORDER BY sort_order ASC, id ASC
         LIMIT 30",
    )?;
    let ids = statement
        .query_map(params![query, like, tag_like], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn next_quick_sort_order(conn: &Connection) -> AppResult<i64> {
    let next = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM quick_inputs",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(next)
}

fn image_to_data_url(image: &ImageData<'_>) -> Option<String> {
    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.to_vec(),
    )?;
    dynamic_image_to_png_data_url(DynamicImage::ImageRgba8(buffer)).ok()
}

fn image_data_url_to_png_bytes(content: &str) -> AppResult<Vec<u8>> {
    let image = image_data_url_to_rgba(content)?;
    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.into_owned(),
    )
    .ok_or_else(|| AppError::Message("图片像素数据不完整".into()))?;
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(buffer)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|error| AppError::Message(format!("编码图片失败: {error}")))?;
    Ok(png)
}

fn write_temp_image_file(content: &str) -> AppResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join("SheepClip")
        .join("clipboard-images");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("sheepclip-{}.png", content_hash(content)));
    if !path.exists() {
        fs::write(&path, image_data_url_to_png_bytes(content)?)?;
    }
    Ok(path)
}

fn image_file_snapshot(path: &Path) -> Option<ClipboardSnapshot> {
    if !is_image_path(path) {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;
    let (width, height) = (image.width(), image.height());
    let data_url = dynamic_image_to_png_data_url(image).ok()?;
    let title = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "图片文件".into());
    Some(ClipboardSnapshot {
        kind: "image".into(),
        title: title.clone(),
        preview: format!("{title} · {}x{}", width, height),
        hash_input: image_clipboard_hash(&data_url),
        content: data_url,
    })
}

fn clipboard_item_meta(kind: &str, content: &str, preview: &str) -> String {
    match kind {
        "text" => format!("{} 字", content.chars().count()),
        "rich_text" => format!("{} 字", strip_html_tags(content).chars().count()),
        "image" => image_meta(content).unwrap_or_else(|| preview.to_string()),
        "file" => file_meta(content),
        _ => String::new(),
    }
}

fn image_meta(content: &str) -> Option<String> {
    let image = image_data_url_to_rgba(content).ok()?;
    Some(format!(
        "{}x{} · {}",
        image.width,
        image.height,
        format_bytes(content.len() as u64)
    ))
}

fn file_meta(content: &str) -> String {
    let paths = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return "0 项".into();
    }

    let mut total_size = 0u64;
    let mut missing = 0usize;
    for path in &paths {
        match path_size(path) {
            Some(size) => total_size = total_size.saturating_add(size),
            None => missing += 1,
        }
    }

    let mut parts = vec![format!("{} 项", paths.len()), format_bytes(total_size)];
    if missing > 0 {
        parts.push(format!("{missing} 项失效"));
    }
    parts.join(" · ")
}

fn path_size(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.is_file() {
        return Some(metadata.len());
    }
    if metadata.is_dir() {
        return Some(dir_size(path));
    }
    Some(0)
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(current) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_file() {
                total = total.saturating_add(metadata.len());
            } else if metadata.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    total
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp"
            )
        })
        .unwrap_or(false)
}

fn dynamic_image_to_png_data_url(image: DynamicImage) -> AppResult<String> {
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|error| AppError::Message(format!("编码图片失败: {error}")))?;
    Ok(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(png)
    ))
}

fn image_data_url_to_rgba(content: &str) -> AppResult<ImageData<'static>> {
    let (_mime, encoded) = parse_data_url(content, "image/")
        .ok_or_else(|| AppError::Message("图片数据格式不正确".into()))?;
    let bytes = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| AppError::Message(format!("解析图片失败: {error}")))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| AppError::Message(format!("读取图片失败: {error}")))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Ok(ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(image.into_raw()),
    })
}

fn normalize_image_data_url(content: &str) -> AppResult<String> {
    let image = image_data_url_to_rgba(content)?;
    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.into_owned(),
    )
    .ok_or_else(|| AppError::Message("图片像素数据不完整".into()))?;
    dynamic_image_to_png_data_url(DynamicImage::ImageRgba8(buffer))
}

fn image_clipboard_hash(content: &str) -> String {
    let normalized = normalize_image_data_url(content).unwrap_or_else(|_| content.to_string());
    format!("image:{}", content_hash(&normalized))
}

fn parse_data_url<'a>(content: &'a str, expected_mime_prefix: &str) -> Option<(&'a str, &'a str)> {
    let rest = content.strip_prefix("data:")?;
    let (metadata, encoded) = rest.split_once(',')?;
    let mut parts = metadata.split(';');
    let mime = parts.next()?.trim();
    if !mime.starts_with(expected_mime_prefix) {
        return None;
    }
    if !parts.any(|part| part.eq_ignore_ascii_case("base64")) {
        return None;
    }
    Some((mime, encoded.trim()))
}

fn strip_html_tags(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                output.push(' ');
            }
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_plain_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn html_has_rich_structure(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "<b",
        "<strong",
        "<i",
        "<em",
        "<u",
        "<span",
        "<font",
        "<table",
        "<tr",
        "<td",
        "<th",
        "<ul",
        "<ol",
        "<li",
        "<img",
        "<a ",
        "style=",
        "class=",
        "font-",
        "color:",
        "background",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn clipboard_skip_marker(kind: &str, content: &str) -> String {
    match kind {
        "image" => {
            let normalized =
                normalize_image_data_url(content).unwrap_or_else(|_| content.to_string());
            format!("image:{}", content_hash(&normalized))
        }
        "file" => format!("file:{content}"),
        "rich_text" => format!("html:{content}"),
        _ => format!("text:{content}"),
    }
}

fn normalize_content_for_clipboard(kind: &str, content: &str) -> String {
    if kind == "rich_text" && content.trim_start().starts_with('<') {
        inline_rich_text_images(content, None)
    } else {
        content.to_string()
    }
}

fn set_clipboard_by_kind(clipboard: &mut Clipboard, kind: &str, content: &str) -> AppResult<()> {
    match kind {
        "file" => {
            let paths = content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            if paths.is_empty() {
                clipboard.set_text(content.to_string())?;
            } else {
                clipboard.set().file_list(&paths)?;
            }
        }
        "image" => {
            if content.starts_with("data:image/") {
                clipboard.set_image(image_data_url_to_rgba(content)?)?;
                #[cfg(target_os = "windows")]
                {
                    if let Ok(path) = write_temp_image_file(content) {
                        let path_text = path.display().to_string();
                        let paths = [path_text.as_str()];
                        let _clipboard =
                            clipboard_win::Clipboard::new_attempts(5).map_err(|error| {
                                AppError::Message(format!("打开剪贴板失败: {error}"))
                            })?;
                        clipboard_win::raw::set_file_list_with(
                            &paths,
                            clipboard_win::options::NoClear,
                        )
                        .map_err(|error| {
                            AppError::Message(format!("写入图片文件路径失败: {error}"))
                        })?;
                    }
                }
            } else {
                clipboard.set_text(content.to_string())?;
            }
        }
        "rich_text" => {
            let plain = strip_html_tags(content);
            if content.trim_start().starts_with('<') {
                clipboard.set_html(content.to_string(), Some(plain))?;
            } else {
                clipboard.set_text(plain)?;
            }
        }
        _ => {
            clipboard.set_text(content.to_string())?;
        }
    }
    Ok(())
}

fn show_floating_window(app: &AppHandle, payload: FloatingPayload) {
    if let Some(window) = app.get_webview_window("floating") {
        let _ = window.show();
        let _ = app.emit_to("floating", "floating-triggered", payload);
        thread::sleep(Duration::from_millis(40));
        let _ = window.set_focus();
    }
}

fn hide_floating_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("floating") {
        let _ = window.hide();
    }
}

fn reset_inline_state(inline_state: &Arc<Mutex<InlineState>>) {
    if let Ok(mut state) = inline_state.lock() {
        state.active = false;
        state.query.clear();
        state.selected_index = 0;
        state.item_ids.clear();
        state.last_slash = None;
        state.target_hwnd = 0;
    }
}

fn inline_target_window(inline_state: &Arc<Mutex<InlineState>>) -> isize {
    inline_state
        .lock()
        .map(|state| state.target_hwnd)
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn current_foreground_window() -> isize {
    unsafe { winapi::um::winuser::GetForegroundWindow() as isize }
}

#[cfg(not(target_os = "windows"))]
fn current_foreground_window() -> isize {
    0
}

#[cfg(target_os = "windows")]
fn focus_window(hwnd: isize) {
    if hwnd != 0 {
        unsafe {
            let _ = winapi::um::winuser::SetForegroundWindow(hwnd as _);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn focus_window(_hwnd: isize) {}

#[cfg(target_os = "windows")]
fn active_app_name() -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::{
        handleapi::CloseHandle,
        processthreadsapi::OpenProcess,
        winbase::QueryFullProcessImageNameW,
        winnt::PROCESS_QUERY_LIMITED_INFORMATION,
        winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId},
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        let mut title_buf = vec![0u16; 256];
        let title_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
        let title = if title_len > 0 {
            Some(
                OsString::from_wide(&title_buf[..title_len as usize])
                    .to_string_lossy()
                    .to_string(),
            )
        } else {
            None
        };

        let mut process_id: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == 0 {
            return title;
        }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if handle.is_null() {
            return title;
        }

        let mut path_buf = vec![0u16; 1024];
        let mut size = path_buf.len() as DWORD;
        let process_name =
            if QueryFullProcessImageNameW(handle, 0, path_buf.as_mut_ptr(), &mut size) != 0 {
                let path = OsString::from_wide(&path_buf[..size as usize])
                    .to_string_lossy()
                    .to_string();
                Path::new(&path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            } else {
                None
            };
        CloseHandle(handle);

        match (process_name, title) {
            (Some(process), Some(title)) if !title.is_empty() => {
                Some(format!("{process} - {title}"))
            }
            (Some(process), _) => Some(process),
            (_, title) => title,
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn active_app_name() -> Option<String> {
    None
}

fn paste_with_backspace(backspace_count: usize) -> AppResult<()> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|error| AppError::Message(format!("初始化键盘模拟失败: {error:?}")))?;

    for _ in 0..backspace_count.min(32) {
        enigo
            .key(EnigoKey::Backspace, Direction::Click)
            .map_err(|error| AppError::Message(format!("发送退格失败: {error:?}")))?;
        thread::sleep(Duration::from_millis(12));
    }

    enigo
        .key(EnigoKey::Control, Direction::Press)
        .map_err(|error| AppError::Message(format!("按下 Ctrl 失败: {error:?}")))?;
    enigo
        .key(EnigoKey::Unicode('v'), Direction::Click)
        .map_err(|error| AppError::Message(format!("发送 V 失败: {error:?}")))?;
    enigo
        .key(EnigoKey::Control, Direction::Release)
        .map_err(|error| AppError::Message(format!("释放 Ctrl 失败: {error:?}")))?;

    Ok(())
}

fn make_title(content: &str) -> String {
    content
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| make_preview(line, 42))
        .unwrap_or_else(|| "文本剪贴板".into())
}

fn make_preview(content: &str, max_chars: usize) -> String {
    let compact = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut output = String::new();
    for character in compact.chars().take(max_chars) {
        output.push(character);
    }
    if compact.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

fn derive_prefix(content: &str, length: usize) -> String {
    content
        .chars()
        .filter(|character| !character.is_whitespace())
        .take(length)
        .collect()
}

fn content_hash(content: &str) -> String {
    let mut hash: u64 = 1469598103934665603;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{hash:016x}")
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn normalize_tag(value: &str) -> AppResult<String> {
    let normalized = value
        .trim()
        .trim_start_matches('#')
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || *character == '_' || *character == '-'
        })
        .collect::<String>()
        .to_ascii_lowercase();

    if normalized.is_empty() {
        return Err(AppError::Message(
            "标签不能为空，且仅支持英文、数字、下划线和短横线".into(),
        ));
    }

    Ok(normalized)
}

impl MainHotkey {
    fn from_setting(value: &str) -> Self {
        if value == "Ctrl" {
            Self::Ctrl
        } else {
            Self::Alt
        }
    }

    fn matches_key(self, key: Key) -> bool {
        match self {
            Self::Alt => matches!(key, Key::Alt | Key::AltGr),
            Self::Ctrl => matches!(key, Key::ControlLeft | Key::ControlRight),
        }
    }
}
