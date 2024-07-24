// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{
    routing::{get, post},
    Router,
};
use std::{process::Command, sync::Mutex, time::Duration};

use maud::{html, Markup, DOCTYPE};
use mptt::modbus::*;
use std::sync::Arc;
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};
use tokio_modbus::FunctionCode;
use tower_http::services::ServeDir;

struct AppState {
    shutdown_signal: Arc<Mutex<bool>>,
}
// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let show_ui = CustomMenuItem::new("show_ui".to_string(), "Show UI");
    let app_state = AppState {
        shutdown_signal: Arc::new(Mutex::new(false)),
    };
    let shutdown_signal = app_state.shutdown_signal.clone();
    let state = shutdown_signal.clone();
    std::thread::spawn(move || {
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            let _ = rt.block_on(run_server(state));
        }
    });
    let tray_menu = SystemTrayMenu::new()
        .add_item(show_ui)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    tauri::Builder::default()
        .manage(app_state)
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .on_system_tray_event(move |_app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "start_runtime" => {}
                "show_ui" => {
                    let _output = if cfg!(target_os = "windows") {
                        Command::new("explorer")
                            .arg("http://127.0.0.1:3000")
                            .spawn()
                            .expect("failed to execute process")
                    } else {
                        Command::new("open")
                            .arg("-n")
                            .arg("http://127.0.0.1:3000")
                            .spawn()
                            .expect("failed to execute process")
                    };
                }
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

async fn run_server(_shutdown_signal: Arc<Mutex<bool>>) {
    let state = Arc::new(tokio::sync::Mutex::new(ModbusState {
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
        .nest_service("/assets", ServeDir::new("./assets/"))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Starting server...");

    axum::serve(listener, app)
        /*
        .with_graceful_shutdown(async move {
            loop {
                if let Ok(mut signal) = shutdown_signal.lock() {
                    if *signal {
                        println!("Shutting down server...");
                        *signal = false;
                        return;
                    }
                }
            }
        })
         */
        .await
        .unwrap();
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
        (header("MPTT Modbus TCP", "MPTT"))
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
               details {
                   summary { "About" }
                   ul {
                       li {
                           "Author: Abdelkader Madoui"
                       }
                       li {
                           "Email: abdelkadermadoui@protonmail.com"
                       }
                   }
               }
           }
        }
        (modbus_tcp_body())
    }
}

pub async fn modbus_serial() -> Markup {
    html! {
        (header("MPTT Modbus Serial", "MPTT"))
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
               details {
                   summary { "About" }
                   ul {
                       li {
                           "Author: Abdelkader Madoui"
                       }
                       li {
                           "Email: abdelkadermadoui@protonmail.com"
                       }
                   }
               }
           }

        }
        (modbus_serial_body())
    }
}
