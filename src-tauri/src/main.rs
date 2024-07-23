// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{
    routing::{get, post},
    Router,
};
use maud::{html, Markup, DOCTYPE};
use mptt::modbus::*;
use std::sync::Arc;
use tauri::Manager;
use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};
use tokio::sync::Mutex;
use tokio_modbus::FunctionCode;
use tower_http::services::ServeDir;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let start_runtime = CustomMenuItem::new("start_runtime".to_string(), "Start Runtime");
    let stop_runtime = CustomMenuItem::new("stop_runtime".to_string(), "Stop Runtime");
    let tray_menu = SystemTrayMenu::new()
        .add_item(start_runtime)
        .add_item(stop_runtime)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    tauri::Builder::default()
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick {
                position: _,
                size: _,
                ..
            } => {
                println!("system tray received a left click");
            }
            SystemTrayEvent::RightClick {
                position: _,
                size: _,
                ..
            } => {
                println!("system tray received a right click");
            }
            SystemTrayEvent::DoubleClick {
                position: _,
                size: _,
                ..
            } => {
                println!("system tray received a double click");
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "start_runtime" => {
                    std::thread::spawn(move || {
                        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        {
                            let _ = rt.block_on(run_server());
                        }
                    });
                }
                "stop_runtime" => {}
                _ => {}
            },
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("Error while building tauri application.")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                api.prevent_exit();
            }
            _ => {}
        });
}

async fn run_server() {
    let state = Arc::new(Mutex::new(ModbusState {
        context: None,
        poll_time: None,
        protocol_options: ProtocolOpts {
            function_code: FunctionCode::ReadHoldingRegisters,
            start_register: 1,
            count: 5,
            float32: false,
        },
    }));
    let app = Router::new()
        .route("/", get(modbus_tcp))
        .route("/modbus_serial", get(modbus_serial))
        .route("/poll_modbus", get(poll_modbus))
        .route("/heartbeat", get(heartbeat))
        .route("/disconnect_modbus", get(disconnect_modbus))
        .route("/connect_modbus_tcp", post(connect_modbus_tcp))
        .route("/connect_modbus_serial", post(connect_modbus_serial))
        .route("/write_modbus", post(write_modbus))
        .route("/update_modbus", post(update_modbus))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Starting server...");
    axum::serve(listener, app).await.unwrap();
}

fn header(title: &str, icon: &str) -> Markup {
    html! {
        (DOCTYPE)
        head {
           title {
               (format!("{}", title))
           }
           link rel="icon" type="image/x-icon" href=(format!("assets/{}.ico", icon)) {}
           // Calling CSS
           //link rel="stylesheet" href="https://unpkg.com/98.css" {}
           link rel="stylesheet" href="assets/css/style.css" {}
           link rel="stylesheet" href="assets/css/docs/docs.css" {}
           link rel="stylesheet" href="assets/css/docs/vs.css" {}
           // Calling HTMX
           script src="assets/htmx.min.js" {}
        }

    }
}

pub async fn modbus_tcp() -> Markup {
    html! {
        (header("MPTT Modbus TCP", "modbus"))
        aside {
            ul class="tree-view" style="height: 500px;" {
                li {
                    a href="" { "Modbus" }
                }
               ul {
                   li {
                       a href="/" { "Modbus TCP" }
                   }
                   li {
                       a href="/modbus_serial" { "Modbus Serial" }
                   }
               }
           }
        }
        (modbus_tcp_body())
    }
}

pub async fn modbus_serial() -> Markup {
    html! {
        (header("MPTT Modbus Serial", "modbus"))
        aside {
            ul class="tree-view" style="height: 500px;" {
                li {
                    a href="" { "Modbus" }
                }
               ul {
                   li {
                       a href="/" { "Modbus TCP" }
                   }
                   li {
                       a href="/modbus_serial" { "Modbus Serial" }
                   }
               }
           }
        }
        (modbus_serial_body())
    }
}
