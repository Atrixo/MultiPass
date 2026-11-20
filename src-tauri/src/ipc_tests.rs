use serde_json::{json, Value};
use std::path::Path;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{WebviewWindow, WebviewWindowBuilder};

fn window() -> (tauri::App<tauri::test::MockRuntime>, WebviewWindow<tauri::test::MockRuntime>) {
    let app = crate::setup(mock_builder()).build(mock_context(noop_assets())).unwrap();
    let w = WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
    (app, w)
}

fn call(w: &WebviewWindow<tauri::test::MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    get_ipc_response(
        w,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: if cfg!(windows) { "http://tauri.localhost" } else { "tauri://localhost" }.parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|b| b.deserialize::<Value>().unwrap())
}

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures").join(name).to_string_lossy().into_owned()
}

#[test]
fn oberflaechen_aufrufe_wie_im_frontend() {
    let (_app, w) = window();
    let loaded = call(
        &w,
        "load_files",
        json!({ "paths": [fixture("5_T13_Kontur.cnc"), fixture("4_T16_Gravur.cnc"), fixture("gibt_es_nicht.cnc")] }),
    )
    .unwrap();
    let files = loaded["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0]["name"], "4_T16_Gravur.cnc");
    assert_eq!(files[0]["stockTop"], true);
    assert_eq!(files[0]["tools"][0]["number"], 1);
    assert_eq!(loaded["errors"][0]["error"]["code"], "io");
    let ids: Vec<Value> = files.iter().map(|f| f["id"].clone()).collect();

    let maps = call(&w, "auto_tools", json!({ "ids": ids, "inventory": [] })).unwrap();
    assert_eq!(maps, json!([{ "1": 1 }, { "1": 2 }]));

    let prepared = call(
        &w,
        "prepare_merge",
        json!({ "ids": ids, "maps": [{ "1": 16 }, { "1": 13 }], "restore": true, "safeZ": 1.0, "stamp": "2026-11-19 12:00" }),
    )
    .unwrap();
    assert_eq!(prepared["warnings"], json!([]));
    assert!(prepared["rapid"]["converted"].as_u64().unwrap() > 0);
    assert_eq!(prepared["suggestedName"], "GESAMT.cnc");

    let dir = std::env::temp_dir().join(format!("multipass_ipc_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("GESAMT.cnc");
    let saved = call(&w, "save_merge", json!({ "path": out.to_string_lossy() })).unwrap();
    assert_eq!(saved["path"], out.to_string_lossy().as_ref());
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("\r\nT16M6 \r\n") && text.contains("\r\nT13M6 \r\n"));
    assert!(text
        .starts_with(&format!("(Zusammengefuegt mit MultiPass {} am 2026-11-19 12:00)", multipass_core::APP_VERSION)));
    let again = call(&w, "load_files", json!({ "paths": [out.to_string_lossy()] })).unwrap();
    assert_eq!(again["files"], json!([]));
    assert_eq!(again["errors"][0]["error"]["code"], "alreadyMerged");
    assert!(again["errors"][0]["message"].as_str().unwrap().contains("doppelt"));
    std::fs::remove_dir_all(&dir).unwrap();

    let est = call(&w, "estimate", json!({ "ids": ids, "safeZ": 1.0 })).unwrap();
    assert_eq!(est["rapidAllowed"], true);
    let f0 = &est["files"][0];
    assert!(f0["plain"]["cutMin"].as_f64().unwrap() > f0["fast"]["cutMin"].as_f64().unwrap());
    let again = call(&w, "estimate", json!({ "ids": ids, "safeZ": 20.0 })).unwrap();
    assert_eq!(again["files"][0]["plain"], f0["plain"]);
    assert_ne!(again["files"][0]["fast"], f0["fast"]);

    let prev = call(&w, "preview", json!({ "ids": ids, "restore": true, "safeZ": 1.0 })).unwrap();
    let p0 = &prev[0]["preview"];
    assert!(p0["points"].as_u64().unwrap() > 10);
    assert!(p0["polylines"].as_array().unwrap().iter().any(|l| l["rapid"] == true));
    assert_eq!(p0["bounds"].as_array().unwrap().len(), 4);

    let err = call(&w, "prepare_merge", json!({ "ids": [], "maps": [], "restore": false, "safeZ": 1.0, "stamp": "x" }))
        .unwrap_err();
    assert_eq!(err["error"]["code"], "noFiles");

    call(&w, "forget_files", json!({ "ids": ids })).unwrap();
    assert!(call(&w, "auto_tools", json!({ "ids": ids, "inventory": [] })).is_err());
}
