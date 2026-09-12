use std::ffi::c_void;
use minhook::MinHook;
use windows::UI::Notifications::ToastNotificationManager;
use windows_sys::Win32::Foundation::{BOOL, FALSE, TRUE};
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::Win32::UI::Shell::{NOTIFYICONDATAW, NIF_INFO, NIM_ADD, NIM_MODIFY};

type ShellNotifyIconWFn = unsafe extern "system" fn(dw_message: u32, lp_data: *const NOTIFYICONDATAW) -> BOOL;

static mut ORIGINAL_SHELL_NOTIFYICONW: Option<ShellNotifyIconWFn> = None;
type ToastNotifierShowFn = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;
static mut ORIGINAL_TOAST_NOTIFIER_SHOW: Option<ToastNotifierShowFn> = None;

unsafe extern "system" fn detour_toast_notifier_show(
    _this: *mut c_void,
    notification: *mut c_void,
) -> i32 {
    println!("[HOOK Rust] Toast moderne bloqué");
    if !notification.is_null() {
        if let Some(notification) =
            windows::core::from_raw_borrowed::<windows::UI::Notifications::ToastNotification>(
                &notification,
            )
        {
            if let Ok(content) = notification.Content() {
                if let Ok(xml) = content.GetXml() {
                    println!("[HOOK Rust] Contenu : {}", xml);
                }
            }
        }
    }
    0
}

unsafe extern "system" fn detour_shell_notifyiconw(dw_message: u32, lp_data: *const NOTIFYICONDATAW) -> BOOL {
    if !lp_data.is_null() && (dw_message == NIM_ADD || dw_message == NIM_MODIFY) {
        let data = &*lp_data;

        if (data.uFlags & NIF_INFO) != 0 {
            let title = String::from_utf16_lossy(&data.szInfoTitle)
                .trim_matches('\0')
                .to_string();
            let message = String::from_utf16_lossy(&data.szInfo)
                .trim_matches('\0')
                .to_string();

            println!("[HOOK Rust] Notification détectée !");
            println!("[HOOK Rust] Titre : {}", title);
            println!("[HOOK Rust] Message : {}", message);

            return TRUE;
        }
    }

    if let Some(original) = ORIGINAL_SHELL_NOTIFYICONW {
        original(dw_message, lp_data)
    } else {
        FALSE
    }
}

fn init_hook() {
    unsafe {
        if let Ok(manager) = ToastNotificationManager::GetDefault() {
            if let Ok(notifier) = manager.CreateToastNotifier() {
                let notifier_raw = windows::core::Interface::as_raw(&notifier);
                let vtable = *(notifier_raw as *mut *mut *mut c_void);
                let target = *vtable.add(6);
                let detour = detour_toast_notifier_show as *mut c_void;

                if let Ok(orig) = MinHook::create_hook(target, detour) {
                    ORIGINAL_TOAST_NOTIFIER_SHOW = Some(std::mem::transmute(orig));
                    let _ = MinHook::enable_hook(target);
                }
            }
        }

        let module_name = windows_sys::s!("shell32.dll");
        let handle = windows_sys::Win32::System::LibraryLoader::GetModuleHandleA(module_name);
        
        if handle == 0 {
            return;
        }

        let fn_name = windows_sys::s!("Shell_NotifyIconW");
        let target_ptr = windows_sys::Win32::System::LibraryLoader::GetProcAddress(handle, fn_name);

        if let Some(target) = target_ptr {
            let target_void = target as *mut c_void;
            let detour_void = detour_shell_notifyiconw as *mut c_void;

            if let Ok(orig) = MinHook::create_hook(target_void, detour_void) {
                ORIGINAL_SHELL_NOTIFYICONW = Some(std::mem::transmute(orig));
                
                let _ = MinHook::enable_hook(target_void);
            }
        }
    }
}

#[no_mangle]
pub extern "system" fn DllMain(
    _hmodule: *mut c_void,
    ul_reason_for_call: u32,
    _lpreserved: *mut c_void,
) -> BOOL {
    if ul_reason_for_call == DLL_PROCESS_ATTACH {
        std::thread::spawn(|| {
            init_hook();
        });
    }
    TRUE
}