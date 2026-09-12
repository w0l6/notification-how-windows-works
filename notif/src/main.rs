use windows::{
    core::{HSTRING, Result},
    Data::Xml::Dom::XmlDocument,
    UI::Notifications::{
        ToastNotification,
        ToastNotificationManager,
    },
};

fn main() -> Result<()> {
    let xml = XmlDocument::new()?;

    let xml_content = HSTRING::from(
        r#"<toast>
            <visual>
                <binding template="ToastGeneric">
                    <text>Mon programme</text>
                    <text>Hello depuis Windows 11 !</text>
                </binding>
            </visual>
        </toast>"#,
    );

    xml.LoadXml(&xml_content)?;

    let toast = ToastNotification::CreateToastNotification(&xml)?;

    let app_id = HSTRING::from("Microsoft.Windows.Explorer");

    let notifier =
        ToastNotificationManager::CreateToastNotifierWithId(&app_id)?;

    notifier.Show(&toast)?;

    Ok(())
}