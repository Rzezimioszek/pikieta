mod gml;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Okna otwierania i zapisu plików oraz zapis do ścieżek z tych okien.
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(gml::Docs::default())
        .invoke_handler(tauri::generate_handler![
            gml::gml_open,
            gml::gml_render,
            gml::gml_texts,
            gml::gml_class_table,
            gml::gml_properties,
            gml::gml_set,
            gml::gml_save,
            gml::gml_xml,
            gml::gml_close,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
