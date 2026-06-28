use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{
    menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder},
    Emitter, LogicalSize, Manager, PhysicalPosition, Position, Runtime, Size, State, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

const OVERLAY_LABEL: &str = "overlay";
const TRAY_ID: &str = "miserarenaiyo";
const CMD_B_HOTKEY_ID: u32 = 1;
const CMD_B_HOTKEY_SIGNATURE: u32 = u32::from_be_bytes(*b"MRNY");
const MENU_STATUS_ID: &str = "menu-status";
const MENU_TOGGLE_ID: &str = "menu-toggle-overlay";
const MENU_TYPE1_ID: &str = "menu-type1";
const MENU_TYPE2_ID: &str = "menu-type2";
const MENU_QUIT_ID: &str = "menu-quit";
const OVERLAY_TYPE_CHANGED_EVENT: &str = "overlay-item-type-changed";

#[cfg(target_os = "macos")]
mod macos_global_shortcut {
    use super::*;
    use std::{ffi::c_void, mem::size_of, ptr::null_mut, sync::OnceLock};

    type OSStatus = i32;
    type ItemCount = u32;
    type OSType = u32;
    type EventParamName = u32;
    type EventParamType = u32;
    type EventTargetRef = *mut c_void;
    type EventRef = *mut c_void;
    type EventHandlerCallRef = *mut c_void;
    type EventHandlerRef = *mut c_void;
    type EventHotKeyRef = *mut c_void;
    type EventHandlerUPP = Option<
        unsafe extern "C" fn(
            in_handler_call_ref: EventHandlerCallRef,
            in_event: EventRef,
            in_user_data: *mut c_void,
        ) -> OSStatus,
    >;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct EventTypeSpec {
        event_class: OSType,
        event_kind: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct EventHotKeyID {
        signature: OSType,
        id: u32,
    }

    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn GetApplicationEventTarget() -> EventTargetRef;
        fn InstallEventHandler(
            in_target: EventTargetRef,
            in_handler: EventHandlerUPP,
            in_num_types: ItemCount,
            in_list: *const EventTypeSpec,
            in_user_data: *mut c_void,
            out_ref: *mut EventHandlerRef,
        ) -> OSStatus;
        fn RegisterEventHotKey(
            in_hot_key_code: u32,
            in_hot_key_modifiers: u32,
            in_hot_key_id: EventHotKeyID,
            in_target: EventTargetRef,
            in_options: u32,
            out_ref: *mut EventHotKeyRef,
        ) -> OSStatus;
        fn RemoveEventHandler(in_handler: EventHandlerRef) -> OSStatus;
        fn GetEventParameter(
            in_event: EventRef,
            in_name: EventParamName,
            in_desired_type: EventParamType,
            out_actual_type: *mut EventParamType,
            in_buffer_size: usize,
            out_actual_size: *mut usize,
            out_data: *mut c_void,
        ) -> OSStatus;
    }

    const NO_ERR: OSStatus = 0;
    const K_EVENT_CLASS_KEYBOARD: OSType = u32::from_be_bytes(*b"keyb");
    const K_EVENT_HOT_KEY_PRESSED: u32 = 5;
    const K_EVENT_PARAM_DIRECT_OBJECT: EventParamName = u32::from_be_bytes(*b"----");
    const TYPE_EVENT_HOT_KEY_ID: EventParamType = u32::from_be_bytes(*b"hkid");
    const K_EVENT_HOT_KEY_NO_OPTIONS: u32 = 0;
    const CMD_KEY_MASK: u32 = 1 << 8;
    const KEY_CODE_B: u32 = 0x0B;

    struct ShortcutContext {
        callback: Box<dyn Fn() + Send + 'static>,
    }

    struct ShortcutRegistration {
        _context: usize,
        _handler_ref: usize,
        _hotkey_ref: usize,
    }

    static REGISTRATION: OnceLock<ShortcutRegistration> = OnceLock::new();

    unsafe extern "C" fn hotkey_event_handler(
        _in_handler_call_ref: EventHandlerCallRef,
        in_event: EventRef,
        in_user_data: *mut c_void,
    ) -> OSStatus {
        let mut hotkey_id = EventHotKeyID {
            signature: 0,
            id: 0,
        };
        let status = GetEventParameter(
            in_event,
            K_EVENT_PARAM_DIRECT_OBJECT,
            TYPE_EVENT_HOT_KEY_ID,
            null_mut(),
            size_of::<EventHotKeyID>(),
            null_mut(),
            (&mut hotkey_id as *mut EventHotKeyID).cast(),
        );

        if status == NO_ERR
            && hotkey_id.signature == CMD_B_HOTKEY_SIGNATURE
            && hotkey_id.id == CMD_B_HOTKEY_ID
            && !in_user_data.is_null()
        {
            let context = &*(in_user_data as *const ShortcutContext);
            (context.callback)();
        }

        NO_ERR
    }

    pub fn register_cmd_b_shortcut(app: tauri::AppHandle) -> tauri::Result<()> {
        if REGISTRATION.get().is_some() {
            return Ok(());
        }

        let callback_app = app.clone();
        let context = Box::new(ShortcutContext {
            callback: Box::new(move || {
                let app_handle = callback_app.clone();
                let app_for_main_thread = app_handle.clone();
                let _ = app_handle.run_on_main_thread(move || {
                    let state = app_for_main_thread.state::<AppState>();
                    let _ = toggle_overlay(app_for_main_thread.clone(), state);
                });
            }),
        });
        let context_ptr = Box::into_raw(context);

        let event_types = [EventTypeSpec {
            event_class: K_EVENT_CLASS_KEYBOARD,
            event_kind: K_EVENT_HOT_KEY_PRESSED,
        }];

        let mut handler_ref: EventHandlerRef = null_mut();
        let install_status = unsafe {
            InstallEventHandler(
                GetApplicationEventTarget(),
                Some(hotkey_event_handler),
                event_types.len() as ItemCount,
                event_types.as_ptr(),
                context_ptr.cast(),
                &mut handler_ref,
            )
        };
        if install_status != NO_ERR {
            unsafe {
                drop(Box::from_raw(context_ptr));
            }
            return Err(std::io::Error::other(format!(
                "failed to install Cmd+B hotkey handler: {install_status}"
            ))
            .into());
        }

        let mut hotkey_ref: EventHotKeyRef = null_mut();
        let register_status = unsafe {
            RegisterEventHotKey(
                KEY_CODE_B,
                CMD_KEY_MASK,
                EventHotKeyID {
                    signature: CMD_B_HOTKEY_SIGNATURE,
                    id: CMD_B_HOTKEY_ID,
                },
                GetApplicationEventTarget(),
                K_EVENT_HOT_KEY_NO_OPTIONS,
                &mut hotkey_ref,
            )
        };
        if register_status != NO_ERR {
            unsafe {
                let _ = RemoveEventHandler(handler_ref);
                drop(Box::from_raw(context_ptr));
            }
            return Err(std::io::Error::other(format!(
                "failed to register Cmd+B hotkey: {register_status}"
            ))
            .into());
        }

        let registration = ShortcutRegistration {
            _context: context_ptr as usize,
            _handler_ref: handler_ref as usize,
            _hotkey_ref: hotkey_ref as usize,
        };

        let _ = REGISTRATION.set(registration);
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
mod macos_global_shortcut {
    use super::*;

    pub fn register_cmd_b_shortcut(_app: tauri::AppHandle) -> tauri::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum OverlayItemType {
    Type1,
    Type2,
}

impl OverlayItemType {
    fn size(self) -> (f64, f64) {
        match self {
            Self::Type1 => (240.0, 240.0),
            Self::Type2 => (320.0, 180.0),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Type1 => "Type1",
            Self::Type2 => "Type2",
        }
    }
}

struct AppState {
    overlay_item_type: Mutex<OverlayItemType>,
}

fn overlay_is_visible<R: Runtime, M: Manager<R>>(manager: &M) -> bool {
    manager
        .get_webview_window(OVERLAY_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

fn current_overlay_item_type(state: &State<'_, AppState>) -> OverlayItemType {
    *state
        .overlay_item_type
        .lock()
        .expect("overlay item type mutex poisoned")
}

fn overlay_dimensions(overlay_item_type: OverlayItemType) -> (f64, f64) {
    overlay_item_type.size()
}

fn sync_overlay_window_size<R: Runtime>(
    window: &WebviewWindow<R>,
    overlay_item_type: OverlayItemType,
) -> tauri::Result<()> {
    let (width, height) = overlay_dimensions(overlay_item_type);
    window.set_size(Size::Logical(LogicalSize::new(width, height)))?;
    Ok(())
}

fn position_overlay_window<R: Runtime>(
    window: &WebviewWindow<R>,
    overlay_item_type: OverlayItemType,
) -> tauri::Result<()> {
    sync_overlay_window_size(window, overlay_item_type)?;

    if let Ok(cursor_position) = window.cursor_position() {
        let scale_factor = window
            .monitor_from_point(cursor_position.x, cursor_position.y)?
            .or_else(|| window.current_monitor().ok().flatten())
            .or_else(|| window.primary_monitor().ok().flatten())
            .map(|monitor| monitor.scale_factor())
            .unwrap_or(1.0);

        let (width, height) = overlay_dimensions(overlay_item_type);
        let offset_x = (width * scale_factor) / 2.0;
        let offset_y = (height * scale_factor) / 2.0;
        let target_x = (cursor_position.x - offset_x).round() as i32;
        let target_y = (cursor_position.y - offset_y).round() as i32;

        window.set_position(Position::Physical(PhysicalPosition::new(
            target_x, target_y,
        )))?;
    }

    window.show()?;
    Ok(())
}

fn create_overlay_window<R: Runtime, M: Manager<R>>(
    manager: &M,
    overlay_item_type: OverlayItemType,
) -> tauri::Result<()> {
    if manager.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }

    let (width, height) = overlay_dimensions(overlay_item_type);
    let overlay_window = WebviewWindowBuilder::new(manager, OVERLAY_LABEL, WebviewUrl::default())
        .title("Miserarenaiyo Overlay")
        .inner_size(width, height)
        .resizable(true)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .visible_on_all_workspaces(true)
        .build()?;

    position_overlay_window(&overlay_window, overlay_item_type)?;
    Ok(())
}

fn update_tray_menu<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let Some(tray) = app_handle.tray_by_id(TRAY_ID) else {
        return Ok(());
    };

    let overlay_visible = overlay_is_visible(app_handle);
    let overlay_item_type = *app_handle
        .state::<AppState>()
        .overlay_item_type
        .lock()
        .expect("overlay item type mutex poisoned");

    let status_text = if overlay_visible {
        format!("Overlay: Visible ({})", overlay_item_type.label())
    } else {
        format!("Overlay: Hidden ({})", overlay_item_type.label())
    };
    let toggle_text = if overlay_visible {
        "Hide Overlay"
    } else {
        "Show Overlay"
    };

    let status_item = MenuItemBuilder::with_id(MENU_STATUS_ID, status_text)
        .enabled(false)
        .build(app_handle)?;
    let toggle_item = MenuItemBuilder::with_id(MENU_TOGGLE_ID, toggle_text).build(app_handle)?;
    let type1_item = CheckMenuItemBuilder::with_id(MENU_TYPE1_ID, "Type1")
        .checked(matches!(overlay_item_type, OverlayItemType::Type1))
        .build(app_handle)?;
    let type2_item = CheckMenuItemBuilder::with_id(MENU_TYPE2_ID, "Type2")
        .checked(matches!(overlay_item_type, OverlayItemType::Type2))
        .build(app_handle)?;
    let quit_item = MenuItemBuilder::with_id(MENU_QUIT_ID, "Quit").build(app_handle)?;

    let menu: Menu<R> = MenuBuilder::new(app_handle)
        .item(&status_item)
        .separator()
        .item(&toggle_item)
        .separator()
        .item(&type1_item)
        .item(&type2_item)
        .separator()
        .item(&quit_item)
        .build()?;

    tray.set_menu(Some(menu))?;
    Ok(())
}

fn set_overlay_item_type<R: Runtime>(
    app: &tauri::AppHandle<R>,
    state: &State<'_, AppState>,
    overlay_item_type: OverlayItemType,
) -> tauri::Result<()> {
    {
        let mut current = state
            .overlay_item_type
            .lock()
            .expect("overlay item type mutex poisoned");
        *current = overlay_item_type;
    }

    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        sync_overlay_window_size(&window, overlay_item_type)?;
    }

    app.emit(OVERLAY_TYPE_CHANGED_EVENT, overlay_item_type)?;
    update_tray_menu(app)?;
    Ok(())
}

#[tauri::command]
fn get_overlay_item_type(state: State<'_, AppState>) -> OverlayItemType {
    current_overlay_item_type(&state)
}

#[tauri::command]
fn show_overlay(app: tauri::AppHandle, state: State<'_, AppState>) -> tauri::Result<()> {
    let overlay_item_type = current_overlay_item_type(&state);

    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        position_overlay_window(&window, overlay_item_type)?;
        update_tray_menu(&app)?;
        return Ok(());
    }

    create_overlay_window(&app, overlay_item_type)?;
    update_tray_menu(&app)
}

#[tauri::command]
fn hide_overlay(app: tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        window.hide()?;
    }

    update_tray_menu(&app)?;
    Ok(())
}

#[tauri::command]
fn toggle_overlay(app: tauri::AppHandle, state: State<'_, AppState>) -> tauri::Result<()> {
    let overlay_item_type = current_overlay_item_type(&state);

    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        if window.is_visible().unwrap_or(false) {
            window.hide()?;
        } else {
            position_overlay_window(&window, overlay_item_type)?;
        }
        update_tray_menu(&app)?;
        return Ok(());
    }

    create_overlay_window(&app, overlay_item_type)?;
    update_tray_menu(&app)
}

#[tauri::command]
fn set_type1(app: tauri::AppHandle, state: State<'_, AppState>) -> tauri::Result<()> {
    set_overlay_item_type(&app, &state, OverlayItemType::Type1)
}

#[tauri::command]
fn set_type2(app: tauri::AppHandle, state: State<'_, AppState>) -> tauri::Result<()> {
    set_overlay_item_type(&app, &state, OverlayItemType::Type2)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(AppState {
            overlay_item_type: Mutex::new(OverlayItemType::Type1),
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_overlay_item_type,
            set_type1,
            set_type2,
            show_overlay,
            hide_overlay,
            toggle_overlay
        ])
        .setup(|app| {
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.hide()?;
            }

            macos_global_shortcut::register_cmd_b_shortcut(app.handle().clone())?;

            let overlay_item_type = *app
                .state::<AppState>()
                .overlay_item_type
                .lock()
                .expect("overlay item type mutex poisoned");

            create_overlay_window(app, overlay_item_type)?;
            update_tray_menu(app.handle())?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == OVERLAY_LABEL {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_TOGGLE_ID => {
                let _ = toggle_overlay(app.clone(), app.state::<AppState>());
            }
            MENU_TYPE1_ID => {
                let _ = set_type1(app.clone(), app.state::<AppState>());
            }
            MENU_TYPE2_ID => {
                let _ = set_type2(app.clone(), app.state::<AppState>());
            }
            MENU_QUIT_ID => {
                app.exit(0);
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } = event
        {
            let _ = show_overlay(app_handle.clone(), app_handle.state::<AppState>());
        }
    });
}
