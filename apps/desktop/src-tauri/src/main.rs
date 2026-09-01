#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use void_core::{
    AppInfo, DmMessage, Engine, EngineConfig, GroupInfo, IdentityInfo, OwnProfileInfo,
    PeerInfo, PeerProfileInfo, PendingRequest, PresenceInfo, Settings, TorStatus,
};

struct AppState {
    engine: Mutex<Option<Arc<Engine>>>,
}

fn engine_of(state: &State<AppState>) -> Result<Arc<Engine>, String> {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .ok_or_else(|| "engine-not-ready".to_string())
}

#[tauri::command]
fn get_identity(state: State<AppState>) -> Result<IdentityInfo, String> {
    Ok(engine_of(&state)?.identity_info())
}

#[tauri::command]
fn get_tor_status(state: State<AppState>) -> TorStatus {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.status())
        .unwrap_or(TorStatus::Starting)
}

#[tauri::command]
fn set_display_name(state: State<AppState>, name: String) -> Result<(), String> {
    engine_of(&state)?.set_display_name(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_profile(
    state: State<AppState>,
    displayName: Option<String>,
    bio: Option<String>,
    status: Option<String>,
    accent: Option<String>,
    avatarB64: Option<String>,
) -> Result<(), String> {
    engine_of(&state)?
        .set_profile(displayName, bio, status, accent, avatarB64)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_own_profile(state: State<AppState>) -> Result<OwnProfileInfo, String> {
    Ok(engine_of(&state)?.own_profile())
}

#[tauri::command]
fn list_peer_profiles(state: State<AppState>) -> Vec<PeerProfileInfo> {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.peer_profiles())
        .unwrap_or_default()
}

#[tauri::command]
fn get_peer_profile(
    state: State<AppState>,
    onionId: String,
) -> Result<PeerProfileInfo, String> {
    engine_of(&state)?
        .peer_profile(&onionId)
        .ok_or_else(|| "pair inconnu".to_string())
}

#[tauri::command]
fn get_avatar(
    state: State<AppState>,
    onionId: Option<String>,
) -> Result<String, String> {
    let engine = engine_of(&state)?;
    engine
        .avatar_b64(onionId.as_deref())
        .ok_or_else(|| "avatar absent".to_string())
}

#[tauri::command]
fn get_recovery_phrase(state: State<AppState>) -> Result<String, String> {
    engine_of(&state)?
        .recovery_phrase()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn is_recovery_confirmed(state: State<AppState>) -> bool {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.recovery_confirmed())
        .unwrap_or(false)
}

#[tauri::command]
fn confirm_recovery_phrase(state: State<AppState>) -> Result<(), String> {
    engine_of(&state)?
        .confirm_recovery_phrase()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn restore_from_phrase(
    app: AppHandle,
    state: State<'_, AppState>,
    phrase: String,
) -> Result<IdentityInfo, String> {
    let engine = engine_of(&state)?;
    let info = engine
        .restore_from_phrase(&phrase)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit("identity:changed", info.clone());
    Ok(info)
}

#[tauri::command]
fn get_invite_link(state: State<AppState>) -> Result<String, String> {
    Ok(engine_of(&state)?.invite_link())
}

#[tauri::command]
fn get_invite_qr(state: State<AppState>) -> Result<String, String> {
    engine_of(&state)?.invite_qr_svg().map_err(|e| e.to_string())
}

#[tauri::command]
fn parse_invite_link(state: State<AppState>, link: String) -> Result<PeerInfo, String> {
    engine_of(&state)?.parse_invite(&link).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_peer(state: State<AppState>, link: String) -> Result<PeerInfo, String> {
    engine_of(&state)?.add_peer(&link).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_peers(state: State<AppState>) -> Vec<PeerInfo> {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.peers())
        .unwrap_or_default()
}

#[tauri::command]
fn list_requests(state: State<AppState>) -> Vec<PendingRequest> {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.pending_requests())
        .unwrap_or_default()
}

#[tauri::command]
fn accept_request(
    state: State<AppState>,
    onionId: String,
) -> Result<PeerInfo, String> {
    engine_of(&state)?
        .accept_friend_request(&onionId)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn decline_request(state: State<AppState>, onionId: String) -> Result<(), String> {
    engine_of(&state)?
        .decline_friend_request(&onionId)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_peer(state: State<AppState>, onionId: String) -> Result<(), String> {
    engine_of(&state)?
        .remove_peer(&onionId)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_presence(state: State<AppState>) -> Vec<PresenceInfo> {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.presence())
        .unwrap_or_default()
}

#[tauri::command]
fn send_ping(state: State<AppState>, onionId: String) -> Result<(), String> {
    let engine = engine_of(&state)?;
    engine.request_ping(&onionId);
    Ok(())
}

#[tauri::command]
fn send_dm(
    state: State<AppState>,
    onionId: String,
    text: String,
) -> Result<DmMessage, String> {
    engine_of(&state)?
        .send_dm(&onionId, &text)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn send_voice_dm(
    state: State<AppState>,
    onionId: String,
    data: String,
    durationMs: u32,
) -> Result<DmMessage, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| format!("audio illisible: {e}"))?;
    engine_of(&state)?
        .send_voice_dm(&onionId, &bytes, durationMs)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn send_voice_group(
    state: State<AppState>,
    groupId: String,
    data: String,
    durationMs: u32,
) -> Result<DmMessage, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| format!("audio illisible: {e}"))?;
    engine_of(&state)?
        .send_voice_group(&groupId, &bytes, durationMs)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_voice_blob(state: State<AppState>, messageId: String) -> Result<String, String> {
    use base64::Engine;
    let engine = engine_of(&state)?;
    engine
        .voice_blob(&messageId)
        .map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes))
        .ok_or_else(|| "note vocale introuvable".to_string())
}

#[tauri::command]
fn dm_history(
    state: State<AppState>,
    onionId: String,
    limit: Option<u64>,
    beforeId: Option<String>,
) -> Vec<DmMessage> {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|engine| {
            engine
                .dm_history(&onionId, limit.unwrap_or(100), beforeId.as_deref())
                .ok()
        })
        .unwrap_or_default()
}

#[tauri::command]
fn list_groups(state: State<AppState>) -> Vec<GroupInfo> {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.groups())
        .unwrap_or_default()
}

#[tauri::command]
fn create_group(
    state: State<AppState>,
    name: String,
    members: Vec<String>,
) -> Result<GroupInfo, String> {
    engine_of(&state)?
        .create_group(&name, &members)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_group_member(
    state: State<AppState>,
    groupId: String,
    onionId: String,
) -> Result<GroupInfo, String> {
    engine_of(&state)?
        .add_group_member(&groupId, &onionId)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_group_member(
    state: State<AppState>,
    groupId: String,
    onionId: String,
) -> Result<GroupInfo, String> {
    engine_of(&state)?
        .remove_group_member(&groupId, &onionId)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn leave_group(state: State<AppState>, groupId: String) -> Result<(), String> {
    engine_of(&state)?
        .leave_group(&groupId)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn send_group_message(
    state: State<AppState>,
    groupId: String,
    text: String,
) -> Result<DmMessage, String> {
    engine_of(&state)?
        .send_group_message(&groupId, &text)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn group_history(
    state: State<AppState>,
    groupId: String,
    limit: Option<u64>,
    beforeId: Option<String>,
) -> Vec<DmMessage> {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|engine| {
            engine
                .group_history(&groupId, limit.unwrap_or(100), beforeId.as_deref())
                .ok()
        })
        .unwrap_or_default()
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.settings())
        .unwrap_or_default()
}

#[tauri::command]
fn set_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    engine_of(&state)?
        .set_settings(settings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_info(state: State<AppState>, app: AppHandle) -> AppInfo {
    let mut info = state
        .engine
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.app_info())
        .unwrap_or(AppInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            data_dir: String::new(),
            relay_queue: 0,
        });
    if info.data_dir.is_empty() {
        if let Ok(dir) = app.path().app_data_dir() {
            info.data_dir = dir.display().to_string();
        }
    }
    info
}

// Harden the WebView2 host: disable browser accelerator keys (Ctrl+F find bar,
// F5 reload, F12/devtools, Ctrl+P print…) and auto-grant the microphone so voice
// notes record without the native permission prompt.
#[cfg(windows)]
fn harden_webview(platform: tauri::webview::PlatformWebview) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Settings3, COREWEBVIEW2_PERMISSION_KIND,
        COREWEBVIEW2_PERMISSION_KIND_MICROPHONE, COREWEBVIEW2_PERMISSION_STATE_ALLOW,
    };
    use webview2_com::PermissionRequestedEventHandler;
    use windows::core::Interface;

    unsafe {
        let core = match platform.controller().CoreWebView2() {
            Ok(core) => core,
            Err(e) => {
                tracing::warn!("CoreWebView2 indisponible: {e}");
                return;
            }
        };

        if let Ok(settings) = core.Settings() {
            if let Ok(settings3) = settings.cast::<ICoreWebView2Settings3>() {
                if let Err(e) = settings3.SetAreBrowserAcceleratorKeysEnabled(false) {
                    tracing::warn!("desactivation des touches navigateur impossible: {e}");
                }
            }
        }

        let mut token: i64 = 0;
        if let Err(e) = core.add_PermissionRequested(
            &PermissionRequestedEventHandler::create(Box::new(|_, args| {
                let Some(args) = args else { return Ok(()) };
                let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
                args.PermissionKind(&mut kind)?;
                if kind == COREWEBVIEW2_PERMISSION_KIND_MICROPHONE {
                    args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
                }
                Ok(())
            })),
            &mut token,
        ) {
            tracing::warn!("auto-accord micro impossible: {e}");
        }
    }
}

fn resolve_tor_dir(app: &tauri::AppHandle) -> PathBuf {
    let mut candidates: Vec<PathBuf> = vec![PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "\\resources\\tor"
    ))];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("resources").join("tor"));
            candidates.push(dir.join("tor"));
        }
    }
    if let Ok(resource) = app
        .path()
        .resolve("resources/tor", tauri::path::BaseDirectory::Resource)
    {
        candidates.push(resource);
    }
    if let Ok(resource) = app.path().resolve("tor", tauri::path::BaseDirectory::Resource) {
        candidates.push(resource);
    }
    for candidate in &candidates {
        if candidate.join("tor.exe").exists() {
            return candidate.clone();
        }
    }
    tracing::error!(
        "tor.exe introuvable ; chemins essayes : {:?}",
        candidates
    );
    candidates.remove(0)
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "void=debug,tauri=info,warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            engine: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_identity,
            get_tor_status,
            set_display_name,
            set_profile,
            get_own_profile,
            list_peer_profiles,
            get_peer_profile,
            get_avatar,
            get_recovery_phrase,
            is_recovery_confirmed,
            confirm_recovery_phrase,
            restore_from_phrase,
            get_invite_link,
            get_invite_qr,
            parse_invite_link,
            add_peer,
            list_peers,
            list_requests,
            accept_request,
            decline_request,
            remove_peer,
            get_presence,
            send_ping,
            send_dm,
            send_voice_dm,
            send_voice_group,
            get_voice_blob,
            dm_history,
            list_groups,
            create_group,
            add_group_member,
            remove_group_member,
            leave_group,
            send_group_message,
            group_history,
            get_settings,
            set_settings,
            get_app_info
        ])
        .setup(|app| {
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.with_webview(harden_webview);
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let data_dir = match handle.path().app_data_dir() {
                    Ok(dir) => dir,
                    Err(e) => {
                        tracing::error!("app_data_dir indisponible: {e}");
                        let _ = handle.emit(
                            "tor:status",
                            TorStatus::Failed {
                                error: format!("app_data_dir indisponible: {e}"),
                            },
                        );
                        return;
                    }
                };
                let tor_dir = resolve_tor_dir(&handle);
                tracing::info!(
                    "data_dir={}, tor_dir={}",
                    data_dir.display(),
                    tor_dir.display()
                );
                let cfg = EngineConfig { data_dir, tor_dir };
                match Engine::start(cfg).await {
                    Ok(engine) => {
                        let mut rx = engine.subscribe();
                        {
                            let state = handle.state::<AppState>();
                            *state.engine.lock().unwrap() = Some(Arc::clone(&engine));
                        }
                        let _ = handle.emit("identity:ready", engine.identity_info());
                        let _ = handle.emit("tor:status", engine.status());
                        let _ = handle.emit("presence:changed", engine.presence());

                        let presence_handle = handle.clone();
                        let presence_engine = Arc::clone(&engine);
                        tauri::async_runtime::spawn(async move {
                            let mut prx = presence_engine.subscribe_presence();
                            loop {
                                if prx.changed().await.is_err() {
                                    break;
                                }
                                let presence = prx.borrow().clone();
                                let _ = presence_handle.emit("presence:changed", presence);
                            }
                        });

                        let event_handle = handle.clone();
                        let event_engine = Arc::clone(&engine);
                        tauri::async_runtime::spawn(async move {
                            let mut erx = event_engine.subscribe_events();
                            loop {
                                match erx.recv().await {
                                    Ok(event) => {
                                        let _ = event_handle.emit("core:event", event);
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                        continue;
                                    }
                                    Err(_) => break,
                                }
                            }
                        });

                        loop {
                            if rx.changed().await.is_err() {
                                break;
                            }
                            let status = rx.borrow().clone();
                            let _ = handle.emit("tor:status", status);
                        }
                    }
                    Err(e) => {
                        tracing::error!("démarrage du moteur impossible: {e:#}");
                        let _ = handle.emit(
                            "tor:status",
                            TorStatus::Failed {
                                error: format!("{e:#}"),
                            },
                        );
                    }
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("erreur de build void")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                if let Some(engine) = app.state::<AppState>().engine.lock().unwrap().take() {
                    let _ = tauri::async_runtime::block_on(engine.shutdown());
                }
            }
        });
}
