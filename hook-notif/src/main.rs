use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows::Foundation::TypedEventHandler;
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::{
    KnownNotificationBindings, NotificationKinds, UserNotification, UserNotificationChangedEventArgs,
};

fn print_user_notification(notif: &UserNotification) {
    let id = notif.Id().unwrap_or(0);
    
    let (name, aid) = if let Ok(info) = notif.AppInfo() {
        let name = info
            .DisplayInfo()
            .ok()
            .and_then(|disp| disp.DisplayName().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Inconnu".to_string());
            
        let aid = info
            .AppUserModelId()
            .ok()
            .map(|s| s.to_string())
            .unwrap_or_default();
            
        (name, aid)
    } else {
        ("Inconnu".to_string(), "".to_string())
    };

    let creation_time = notif
        .CreationTime()
        .ok()
        .map(|t| format!("{:?}", t))
        .unwrap_or_default();

    println!("--------------------------------------------------");
    println!("🔔 [NOUVELLE NOTIFICATION CAPTURÉE]");
    println!("  • ID           : {}", id);
    println!("  • Application  : {} ({})", name, aid);
    if !creation_time.is_empty() {
        println!("  • Date/Heure   : {}", creation_time);
    }

    if let Ok(notification) = notif.Notification() {
        if let Ok(visual) = notification.Visual() {
            if let Ok(toast_binding) = KnownNotificationBindings::ToastGeneric() {
                if let Ok(binding) = visual.GetBinding(&toast_binding) {
                    if let Ok(elems) = binding.GetTextElements() {
                        let mut lines = Vec::new();
                        for node in elems {
                            if let Ok(text) = node.Text() {
                                let value = text.to_string();
                                if !value.trim().is_empty() {
                                    lines.push(value);
                                }
                            }
                        }
                        if !lines.is_empty() {
                            println!("  • Titre        : {}", lines[0]);
                            if lines.len() > 1 {
                                for (idx, line) in lines[1..].iter().enumerate() {
                                    println!("  • Contenu [{}]  : {}", idx + 1, line);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    println!("--------------------------------------------------");
}

fn initialize_seen_notifications(
    listener: &UserNotificationListener,
    seen: &Arc<Mutex<HashSet<u32>>>,
) {
    if let Ok(op) = listener.GetNotificationsAsync(NotificationKinds::Toast) {
        if let Ok(notifs) = op.get() {
            if let Ok(count) = notifs.Size() {
                let mut ids = seen.lock().unwrap();
                for i in 0..count {
                    if let Ok(notif) = notifs.GetAt(i) {
                        if let Ok(id) = notif.Id() {
                            ids.insert(id);
                        }
                    }
                }
                println!("ℹ️ Historique ignoré : {} notifications existantes ignorées au démarrage.", count);
            }
        }
    }
}

fn check_for_new_notifications(
    listener: &UserNotificationListener,
    seen: &Arc<Mutex<HashSet<u32>>>,
) {
    if let Ok(op) = listener.GetNotificationsAsync(NotificationKinds::Toast) {
        if let Ok(notifs) = op.get() {
            if let Ok(count) = notifs.Size() {
                for i in 0..count {
                    if let Ok(notif) = notifs.GetAt(i) {
                        if let Ok(id) = notif.Id() {
                            let new = {
                                let mut ids = seen.lock().unwrap();
                                ids.insert(id)
                            };
                            if new {
                                print_user_notification(&notif);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() -> windows::core::Result<()> {
    println!("==================================================");
    println!("  Windows Notification Interceptor (Temps Réel)  ");
    println!("==================================================");

    let listener = UserNotificationListener::Current()?;

    println!("[1] Vérification des permissions Windows...");
    let status = match listener.RequestAccessAsync() {
        Ok(op) => match op.get() {
            Ok(s) => s,
            Err(e) => {
                println!("    ❌ Erreur accès: {:?}", e);
                UserNotificationListenerAccessStatus::Denied
            }
        },
        Err(e) => {
            println!("    ❌ Erreur WinRT: {:?}", e);
            UserNotificationListenerAccessStatus::Denied
        }
    };

    println!("    Statut d'accès : {:?}", status);

    match status {
        UserNotificationListenerAccessStatus::Allowed => {
            println!("    ✅ Accès autorisé aux notifications Windows !");
        }
        UserNotificationListenerAccessStatus::Denied => {
            println!("    ❌ Accès REFUSÉ par Windows.");
            println!("    -> Ouvrez 'Paramètres Windows' > 'Confidentialité et sécurité' > 'Notifications'");
            println!("    -> Activez 'Autoriser les applications à accéder à vos notifications'.");
        }
        UserNotificationListenerAccessStatus::Unspecified => {
            println!("    ⚠️ Statut d'accès non spécifié.");
        }
        _ => {}
    }

    let seen = Arc::new(Mutex::new(HashSet::<u32>::new()));

    initialize_seen_notifications(&listener, &seen);

    let listener_clone = listener.clone();
    let seen_clone = Arc::clone(&seen);
    let _ = listener.NotificationChanged(&TypedEventHandler::new(
        move |_sender: &Option<UserNotificationListener>, _args: &Option<UserNotificationChangedEventArgs>| {
            check_for_new_notifications(&listener_clone, &seen_clone);
            Ok(())
        },
    ));

    println!("\n🚀 [ÉCOUTEUR ACTIF] En écoute continue...");
    println!("   Dès qu'une nouvelle notification apparaît, elle sera affichée ci-dessous.");
    println!("   (Appuyez sur CTRL+C pour fermer)\n");

    loop {
        check_for_new_notifications(&listener, &seen);
        std::thread::sleep(Duration::from_millis(200));
    }
}