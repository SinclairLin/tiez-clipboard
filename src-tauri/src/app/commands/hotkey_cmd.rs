use tauri::{AppHandle, Manager};
use crate::app_state::SettingsState;
use crate::error::{AppResult, AppError};
use crate::global_state::HOTKEY_STRING;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

fn is_wayland_session() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
            || std::env::var_os("WAYLAND_DISPLAY").is_some()
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn hotkey_registration_error(hotkey: &str, err: impl std::fmt::Display) -> AppError {
    let err_str = err.to_string();

    if err_str.contains("AlreadyRegistered") {
        return AppError::Internal(format!("快捷键 `{}` 已被其他程序或当前应用占用", hotkey));
    }

    #[cfg(target_os = "linux")]
    if is_wayland_session() {
        let session = std::env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| std::env::var("DESKTOP_SESSION").ok())
            .unwrap_or_else(|| "Wayland".to_string());
        return AppError::Internal(format!(
            "快捷键 `{}` 注册失败：当前会话是 Wayland ({})。TieZ 当前使用的全局热键后端在该环境下可能不可用，请改用 X11 会话，或在合成器配置里绑定启动/显示 TieZ 的命令。原始错误: {}",
            hotkey, session, err_str
        ));
    }

    AppError::Internal(format!("快捷键 `{}` 注册失败: {}", hotkey, err_str))
}

pub(crate) fn register_shortcut(app_handle: &AppHandle, hotkey: &str) -> AppResult<()> {
    if hotkey.is_empty()
        || hotkey.eq_ignore_ascii_case("MouseMiddle")
        || hotkey.eq_ignore_ascii_case("MButton")
    {
        return Ok(());
    }

    let normalized = hotkey.replace("Win", "Super");
    let shortcut = normalized
        .parse::<Shortcut>()
        .map_err(|_| AppError::Validation(format!("快捷键格式无效: {}", hotkey)))?;

    app_handle
        .global_shortcut()
        .register(shortcut)
        .map_err(|e| hotkey_registration_error(hotkey, e))
}

#[tauri::command]
pub fn register_hotkey(app_handle: AppHandle, hotkey: String) -> AppResult<()> {
    {
        let mut guard = HOTKEY_STRING.lock().unwrap();
        *guard = hotkey.clone();
    }

    if let Some(settings) = app_handle.try_state::<SettingsState>() {
        let mut guard = settings.main_hotkey.lock().unwrap();
        *guard = hotkey.clone();
    }
    
    let _ = app_handle.global_shortcut().unregister_all();
    
    register_shortcut(&app_handle, &hotkey)?;
    
    // sequential hotkey
    let seq_hotkey = {
        let settings = app_handle.state::<SettingsState>();
        let val = settings.sequential_paste_hotkey.lock().unwrap().clone();
        val
    };
    register_shortcut(&app_handle, &seq_hotkey)?;
    
    // rich paste hotkey
    let rich_hotkey = {
        let settings = app_handle.state::<SettingsState>();
        let val = settings.rich_paste_hotkey.lock().unwrap().clone();
        val
    };
    register_shortcut(&app_handle, &rich_hotkey)?;

    // search hotkey
    let search_hotkey = {
        let settings = app_handle.state::<SettingsState>();
        let val = settings.search_hotkey.lock().unwrap().clone();
        val
    };
    register_shortcut(&app_handle, &search_hotkey)?;
    
    Ok(())
}

#[tauri::command]
pub fn test_hotkey_available(app_handle: AppHandle, hotkey: String) -> AppResult<bool> {
    if hotkey.is_empty() || hotkey.eq_ignore_ascii_case("MouseMiddle") || hotkey.eq_ignore_ascii_case("MButton") {
        return Ok(true);
    }
    
    let normalized = hotkey.replace("Win", "Super");
    let shortcut = normalized.parse::<Shortcut>().map_err(|_| AppError::Validation("快捷键格式无效".to_string()))?;
    
    match app_handle.global_shortcut().register(shortcut.clone()) {
        Ok(_) => {
            let _ = app_handle.global_shortcut().unregister(shortcut);
            Ok(true)
        },
        Err(e) => Err(hotkey_registration_error(&hotkey, e)),
    }
}
