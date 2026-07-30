mod commands;
mod detector;
mod error;

use detector::{DetectorResources, DetectorService};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let resources = DetectorResources::from_resource_dir(app.path().resource_dir()?);
            app.manage(DetectorService::new(resources));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::initialize_detector,
            commands::detect_frame
        ])
        .build(tauri::generate_context!())
        .expect("failed to build VISION/LAB");

    app.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            app_handle.state::<DetectorService>().shutdown();
        }
    });
}
