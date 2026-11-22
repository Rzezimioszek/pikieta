//! Natywna obsługa GML w aplikacji desktopowej: plik czyta i przetwarza Rust,
//! do interfejsu trafiają tylko widoki (indeks, bufor geometrii, tabele na żądanie).

use pikieta_gml::GmlDocument;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::State;
use tauri::ipc::Response;

#[derive(Default)]
pub struct Docs {
    next: AtomicU64,
    docs: Mutex<HashMap<String, GmlDocument>>,
}

impl Docs {
    fn with<T>(
        &self,
        id: &str,
        f: impl FnOnce(&mut GmlDocument) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut docs = self.docs.lock().map_err(|_| "blokada dokumentów")?;
        let doc = docs.get_mut(id).ok_or("dokument nie jest wczytany")?;
        f(doc)
    }
}

#[derive(serde::Serialize)]
pub struct Opened {
    id: String,
    index: String,
    issues: String,
}

/// Komendy czytają i zapisują tylko pliki GML/XML wskazane w oknie wyboru.
fn check_ext(path: &str) -> Result<(), String> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("gml" | "xml") => Ok(()),
        _ => Err("Obsługiwane są tylko pliki .gml i .xml.".into()),
    }
}

/// Wczytuje plik GML z dysku w wątku roboczym (nie blokuje okna).
#[tauri::command]
pub async fn gml_open(path: String, state: State<'_, Docs>) -> Result<Opened, String> {
    check_ext(&path)?;
    let doc = tauri::async_runtime::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| format!("Nie można odczytać pliku: {e}"))?;
        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            // Pliki z programów geodezyjnych bywają w Windows-1250.
            Err(e) => encoding_rs::WINDOWS_1250
                .decode(e.as_bytes())
                .0
                .into_owned(),
        };
        GmlDocument::parse(&text).map_err(|e| format!("Błąd XML w {e}"))
    })
    .await
    .map_err(|e| e.to_string())??;
    let id = format!("n{}", state.next.fetch_add(1, Ordering::Relaxed));
    let opened = Opened {
        id: id.clone(),
        index: doc.index_json(),
        issues: doc.issues_json(),
    };
    state
        .docs
        .lock()
        .map_err(|_| "blokada dokumentów")?
        .insert(id, doc);
    Ok(opened)
}

/// Geometria jako bufor binarny (bez JSON).
#[tauri::command]
pub fn gml_render(id: String, epsg: u32, state: State<'_, Docs>) -> Result<Response, String> {
    state.with(&id, |d| {
        Ok(Response::new(d.render_buffer((epsg != 0).then_some(epsg))))
    })
}

#[tauri::command]
pub fn gml_texts(id: String, epsg: u32, state: State<'_, Docs>) -> Result<String, String> {
    state.with(&id, |d| Ok(d.texts_json((epsg != 0).then_some(epsg))))
}

#[tauri::command]
pub fn gml_class_table(
    id: String,
    class_name: String,
    state: State<'_, Docs>,
) -> Result<String, String> {
    state.with(&id, |d| Ok(d.class_table_json(&class_name)))
}

#[tauri::command]
pub fn gml_properties(
    id: String,
    feature: usize,
    state: State<'_, Docs>,
) -> Result<String, String> {
    state.with(&id, |d| Ok(d.properties_json(feature)))
}

/// Zmienia atrybut; zwraca nową etykietę obiektu.
#[tauri::command]
pub fn gml_set(
    id: String,
    feature: usize,
    key: String,
    value: String,
    state: State<'_, Docs>,
) -> Result<String, String> {
    state.with(&id, |d| {
        d.set_property(feature, &key, &value)?;
        Ok(d.features[feature].label.clone())
    })
}

/// Zapisuje dokument do pliku.
#[tauri::command]
pub async fn gml_save(id: String, path: String, state: State<'_, Docs>) -> Result<(), String> {
    check_ext(&path)?;
    let xml = state.with(&id, |d| Ok(d.to_xml()))?;
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&path, xml).map_err(|e| format!("Nie można zapisać pliku: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Tekst dokumentu (do pliku projektu).
#[tauri::command]
pub fn gml_xml(id: String, state: State<'_, Docs>) -> Result<String, String> {
    state.with(&id, |d| Ok(d.to_xml()))
}

#[tauri::command]
pub fn gml_close(id: String, state: State<'_, Docs>) {
    if let Ok(mut docs) = state.docs.lock() {
        docs.remove(&id);
    }
}
