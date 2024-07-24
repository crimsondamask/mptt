use axum::extract::{Form, State};
use maud::{html, Markup};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio_modbus::prelude::*;
use tokio_serial::SerialStream;

use serde::{Deserialize, Serialize};
use tokio_modbus::client::Context;
use tokio_modbus::FunctionCode;

const MARGIN: usize = 20;
const WINDOW_WIDTH: usize = 400;
const TABLE_HEIGHT: usize = 180;
const TABLE_WIDTH: usize = WINDOW_WIDTH - 20;
const TABLE_COL_WIDTH: usize = TABLE_WIDTH / 3;

const STATUS_BAR_FIELD_WIDTH: usize = (WINDOW_WIDTH - 15) / 2;

const SERIAL_TIMEOUT: u64 = 2; // 2 seconds timeout for the serial port.

#[derive(Serialize, Deserialize)]
pub struct ModbusSerialForm {
    pub com: String,
    pub baudrate: u32,
    pub slave: u8,
}
#[derive(Serialize, Deserialize)]
pub struct ModbusTcpForm {
    pub address: String,
    pub port: usize,
}
#[derive(Serialize, Deserialize)]
pub struct ModbusWriteForm {
    pub register: u16,
    pub write_function: String,
    pub float32: String,
    pub value: u16,
}
#[derive(Serialize, Deserialize)]
pub struct ModbusPollingForm {
    pub register: u16,
    pub count: u16,
    pub function: String,
    pub float32: String,
}
#[derive(Serialize, Deserialize)]
pub struct ModbusSerialFormInput {
    pub com: String,
    pub baudrate: u32,
    pub slave: u8,
    pub register: u16,
    pub count: u16,
    pub function: String,
    pub float32: String,
}
pub struct ModbusState {
    pub context: Option<Context>,
    pub poll_time: Option<Duration>,
    pub protocol_options: ProtocolOpts,
}

pub struct ProtocolOpts {
    pub function_code: FunctionCode,
    pub start_register: u16,
    pub count: u16,
    pub float32: bool,
}

pub async fn connect_modbus_tcp(
    State(mtx): State<Arc<Mutex<ModbusState>>>,
    Form(form_input): Form<ModbusTcpForm>,
) -> Markup {
    println!("{}:{}", &form_input.address, &form_input.port);

    let sock_address = format!("{}:{}", &form_input.address, &form_input.port);
    let sock_address = sock_address.parse();
    if let Ok(sock_address) = sock_address {
        if let Ok(ctx) = tcp::connect(sock_address).await {
            let mut mtx = mtx.lock().await;
            mtx.context = Some(ctx);
            html! {
                #modbus_connect_content {
                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)) {  "STATUS: Connected"  }
                }
            }
        } else {
            let mut mtx = mtx.lock().await;
            mtx.context = None;
            html! {
                #modbus_connect_content {
                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  "STATUS: Could not connect to slave!"  }
                }
            }
        }
    } else {
        html! {
            #modbus_connect_content {
                    p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  "STATUS: Could not parse the address or port!"  }
            }
        }
    }
}

pub async fn connect_modbus_serial(
    State(mtx): State<Arc<Mutex<ModbusState>>>,
    Form(form_input): Form<ModbusSerialForm>,
) -> Markup {
    println!("{}:{}", &form_input.com, &form_input.baudrate);

    //let sock_address = format!("{}:{}", &form_input.address, &form_input.port);
    let slave = Slave(form_input.slave);
    let builder = tokio_serial::new(&form_input.com, form_input.baudrate)
        .timeout(Duration::from_secs(SERIAL_TIMEOUT));
    let port = SerialStream::open(&builder);

    if let Ok(port) = port {
        let ctx = rtu::attach_slave(port, slave);
        let mut mtx = mtx.lock().await;
        mtx.context = Some(ctx);
        html! {
            #modbus_connect_content {
                    p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)) {  "STATUS: Connected"  }
            }
        }
    } else {
        html! {
            #modbus_connect_content {
                    p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  "STATUS: Could not open port!"  }
            }
        }
    }
}
pub fn double_register_as_float(reg1: u16, reg2: u16) -> f32 {
    let data_32bit_rep = ((reg1 as u32) << 16) | reg2 as u32;
    let data_32_array = data_32bit_rep.to_ne_bytes();
    f32::from_ne_bytes(data_32_array)
}

pub async fn disconnect_modbus(State(mtx): State<Arc<Mutex<ModbusState>>>) -> Markup {
    let mut mtx = mtx.lock().await;
    match mtx.context.as_mut() {
        Some(ctx) => match ctx.disconnect().await {
            Ok(_ctx) => {
                mtx.context = None;
                html! {
                    #modbus_connect_content {
                            p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)) {  "STATUS: Disconnected"  }
                    }
                }
            }
            _ => {
                mtx.context = None;
                html! {
                    #modbus_connect_content {
                            p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)) {  "STATUS: Disconnected"  }
                    }
                }
            }
        },
        None => {
            html! {
                #modbus_connect_content {
                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)) {  "STATUS: "  }
                }
            }
        }
    }
}

pub async fn heartbeat(State(mtx): State<Arc<Mutex<ModbusState>>>) -> Markup {
    match mtx.lock().await.poll_time {
        Some(time) => {
            html! {
                #heartbeat {
                    div hx-get="/heartbeat" hx-trigger="load delay:1s" hx-target="#heartbeat" hx-swap="innerHTML" {
                       p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)) {
                           (format!("SCANTIME:  {}   micros", time.as_micros()))
                       }
                    }
                }
            }
        }
        None => {
            html! {
                #heartbeat {
                    div hx-get="/heartbeat" hx-trigger="load delay:1s" hx-target="#heartbeat" hx-swap="innerHTML" {
                       p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){
                           "SCANTIME: "
                       }
                    }
                }
            }
        }
    }
}

pub async fn update_modbus(
    State(mtx): State<Arc<Mutex<ModbusState>>>,
    Form(form_input): Form<ModbusPollingForm>,
) -> Markup {
    println!(
        "{}:{} {}",
        &form_input.register, &form_input.count, &form_input.function
    );
    let mut mtx = mtx.lock().await;
    match form_input.function.as_str() {
        "1" => {
            mtx.protocol_options = ProtocolOpts {
                function_code: FunctionCode::ReadCoils,
                start_register: form_input.register,
                count: form_input.count,
                float32: false,
            };
        }
        "3" => match form_input.float32.as_str() {
            "int16" => {
                mtx.protocol_options = ProtocolOpts {
                    function_code: FunctionCode::ReadHoldingRegisters,
                    start_register: form_input.register,
                    count: form_input.count,
                    float32: false,
                };
            }
            "f32" => {
                mtx.protocol_options = ProtocolOpts {
                    function_code: FunctionCode::ReadHoldingRegisters,
                    start_register: form_input.register,
                    count: form_input.count,
                    float32: true,
                };
            }
            _ => {
                mtx.protocol_options = ProtocolOpts {
                    function_code: FunctionCode::ReadHoldingRegisters,
                    start_register: form_input.register,
                    count: form_input.count,
                    float32: false,
                };
            }
        },
        "4" => {
            mtx.protocol_options = ProtocolOpts {
                function_code: FunctionCode::ReadInputRegisters,
                start_register: form_input.register,
                count: form_input.count,
                float32: false,
            };
        }
        _ => {
            mtx.protocol_options = ProtocolOpts {
                function_code: FunctionCode::ReadHoldingRegisters,
                start_register: form_input.register,
                count: form_input.count,
                float32: false,
            };
        }
    }
    html! {
        #modbus_connect_content {
                p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)) {  "STATUS: Updated"  }
        }
    }
}

pub async fn write_modbus(
    State(mtx): State<Arc<Mutex<ModbusState>>>,
    Form(form_input): Form<ModbusWriteForm>,
) -> Markup {
    let mtx = mtx.clone();
    let mut res = mtx.lock().await;

    match res.context.as_mut() {
        Some(ctx) => match form_input.write_function.as_str() {
            "6" => {
                let res = ctx
                    .write_single_register(form_input.register, form_input.value)
                    .await;
                match res {
                    Ok(res) => match res {
                        Ok(_) => {
                            html! {
                                #modbus_connect_content {
                                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  (format!("Wrote: {} to H-Register: {}", form_input.value, form_input.register))  }
                                }
                            }
                        }
                        Err(e) => {
                            html! {
                                #modbus_connect_content {
                                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  (format!("{:?}", e))  }
                                }
                            }
                        }
                    },
                    Err(e) => {
                        html! {
                            #modbus_connect_content {
                                    p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  (format!("{:?}", e))  }
                            }
                        }
                    }
                }
            }
            "5" => match form_input.value {
                1 => {
                    let res = ctx.write_single_coil(form_input.register, true).await;
                    match res {
                        Ok(_) => {
                            html! {
                                #modbus_connect_content {
                                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  (format!("Wrote: {} to Coil: {}", form_input.value, form_input.register))  }
                                }
                            }
                        }
                        Err(e) => {
                            html! {
                                #modbus_connect_content {
                                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  (format!("{:?}", e))  }
                                }
                            }
                        }
                    }
                }
                0 => {
                    let res = ctx.write_single_coil(form_input.register, false).await;
                    match res {
                        Ok(_) => {
                            html! {
                                #modbus_connect_content {
                                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  (format!("Wrote: {} to Coil: {}", form_input.value, form_input.register))  }
                                }
                            }
                        }
                        Err(e) => {
                            html! {
                                #modbus_connect_content {
                                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  (format!("{:?}", e))  }
                                }
                            }
                        }
                    }
                }
                _ => {
                    html! {
                        #modbus_connect_content {
                                p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){ "Only 1 or 0 values accepted!"  }
                        }
                    }
                }
            },
            _ => {
                html! {
                    #modbus_connect_content {
                            p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){ "Bad input!"  }
                    }
                }
            }
        },
        None => {
            html! {
                #modbus_connect_content {
                        p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)){  "STATUS: There is no connection!"  }
                }
            }
        }
    }
}
pub async fn poll_modbus(State(mtx): State<Arc<Mutex<ModbusState>>>) -> Markup {
    let mtx = mtx.clone();
    let mut res = mtx.lock().await;
    let now = Instant::now();
    let function_code = res.protocol_options.function_code.clone();
    let start_register = res.protocol_options.start_register;
    let count = res.protocol_options.count;
    let float32 = res.protocol_options.float32;
    match res.context.as_mut() {
        Some(ctx) => match function_code {
            FunctionCode::ReadInputRegisters => {
                let result = ctx.read_input_registers(start_register, count).await;
                match result {
                    Ok(result) => match result {
                        Ok(result) => {
                            if float32 && result.len() >= 2 {
                                let mut buff: Vec<f32> = Vec::new();
                                let result_copy = result.clone();
                                let mut i = 0;
                                while i < (result_copy.len() - 1) {
                                    buff.push(double_register_as_float(
                                        result_copy[i],
                                        result_copy[i + 1],
                                    ));
                                    i += 2;
                                }
                                res.poll_time = Some(now.elapsed());
                                html! {
                                     #modbus_table {
                                        div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table" hx-swap="innerHTML" {

                                            table class="interactive" {
                                                thead {
                                                    tr {
                                                        th {
                                                            "Register"
                                                        }
                                                        th { "Value" }
                                                        th { "Value (HEX)" }
                                                    }
                                                }
                                                tbody {
                                                    @for (i, value) in buff.iter().enumerate() {
                                                        tr {
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) { (start_register as usize + (i * 2)) }
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                                {
                                                                    (format!("{:.2}", value))
                                                                }
                                                            }
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                                {
                                                                    ""
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                res.poll_time = Some(now.elapsed());
                                html! {
                                     #modbus_table {
                                        div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table" hx-swap="innerHTML" {

                                            table class="interactive" {
                                                thead {
                                                    tr {
                                                        th {
                                                            "Register"
                                                        }
                                                        th { "Value" }
                                                        th { "Value (HEX)" }
                                                    }
                                                }
                                                tbody {
                                                    @for (i, value) in result.iter().enumerate() {
                                                        tr {
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) { (start_register as usize + i) }
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                                {
                                                                    (format!("{}", value))
                                                                }
                                                            }
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                                {
                                                                    (format!("{:#06X}", value))
                                                                }
                                                            }
                                                        }

                                                    }
                                                }

                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            html! {
                                 #modbus_table {
                                    div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table" hx-swap="innerHTML" {
                                        table class="interactive" {
                                            thead {
                                                tr {
                                                    th {
                                                        "Register"
                                                    }
                                                    th { "Value" }
                                                    th { "Value (HEX)" }
                                                }
                                            }
                                            tbody {
                                                tr {
                                                    td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                        (format!("{:?}", e))

                                                    }
                                                    td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                    }
                                                    td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => {
                        res.poll_time = None;
                        html! {
                            #modbus_table {
                               div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus" hx-swap="innerHTML" {

                                   table class="interactive" {
                                       thead {
                                           tr {
                                               th {
                                                   "Register"
                                               }
                                               th { "Value" }
                                           }
                                       }
                                       tbody {
                                           tr {
                                               td style=(format!("width: {}px", TABLE_COL_WIDTH)) { "0" }
                                               td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                   {
                                                       p { (format!("{:?}", e)) }
                                                   }
                                               }
                                           }
                                       }

                                   }
                               }
                           }
                        }
                    }
                }
            }
            FunctionCode::ReadHoldingRegisters => {
                let result = ctx.read_holding_registers(start_register, count).await;
                match result {
                    Ok(result) => match result {
                        Ok(result) => {
                            if float32 && result.len() >= 2 {
                                let mut buff: Vec<f32> = Vec::new();
                                let result_copy = result.clone();
                                let mut i = 0;
                                while i < (result_copy.len() - 1) {
                                    buff.push(double_register_as_float(
                                        result_copy[i],
                                        result_copy[i + 1],
                                    ));
                                    i += 2;
                                }
                                res.poll_time = Some(now.elapsed());
                                html! {
                                     #modbus_table {
                                        div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table" hx-swap="innerHTML" {

                                            table class="interactive" {
                                                thead {
                                                    tr {
                                                        th {
                                                            "Register"
                                                        }
                                                        th { "Value" }
                                                        th { "Value (HEX)" }
                                                    }
                                                }
                                                tbody {
                                                    @for (i, value) in buff.iter().enumerate() {
                                                        tr {
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) { (start_register as usize + (i * 2)) }
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                                {
                                                                    (format!("{:.2}", value))
                                                                }
                                                            }
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                                {
                                                                    ""
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                res.poll_time = Some(now.elapsed());
                                html! {
                                     #modbus_table {
                                        div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table" hx-swap="innerHTML" {

                                            table class="interactive" {
                                                thead {
                                                    tr {
                                                        th {
                                                            "Register"
                                                        }
                                                        th { "Value" }
                                                        th { "Value (HEX)" }
                                                    }
                                                }
                                                tbody {
                                                    @for (i, value) in result.iter().enumerate() {
                                                        tr {
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) { (start_register as usize + i) }
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                                {
                                                                    (format!("{}", value))
                                                                }
                                                            }
                                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                                {
                                                                    (format!("{:#06X}", value))
                                                                }
                                                            }
                                                        }

                                                    }
                                                }

                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            html! {
                                 #modbus_table {
                                    div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table" hx-swap="innerHTML" {
                                        table class="interactive" {
                                            thead {
                                                tr {
                                                    th {
                                                        "Register"
                                                    }
                                                    th { "Value" }
                                                    th { "Value (HEX)" }
                                                }
                                            }
                                            tbody {
                                                tr {
                                                    td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                        (format!("{:?}", e))

                                                    }
                                                    td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                    }
                                                    td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => {
                        res.poll_time = None;
                        html! {
                            #modbus_table {
                               div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus" hx-swap="innerHTML" {

                                   table class="interactive" {
                                       thead {
                                           tr {
                                               th {
                                                   "Register"
                                               }
                                               th { "Value" }
                                           }
                                       }
                                       tbody {
                                           tr {
                                               td style=(format!("width: {}px", TABLE_COL_WIDTH)) { "0" }
                                               td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                   {
                                                       p { (format!("{:?}", e)) }
                                                   }
                                               }
                                           }
                                       }

                                   }
                               }
                           }
                        }
                    }
                }
            }
            _ => {
                let result = ctx.read_holding_registers(start_register, count).await;
                match result {
                    Ok(_result) => {
                        res.poll_time = Some(now.elapsed());
                        html! {
                            #modbus_table {
                               div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table" hx-swap="innerHTML" {

                                   table class="interactive" {
                                       thead {
                                           tr {
                                               th {
                                                   "Register"
                                               }
                                               th { "Value" }
                                           }
                                       }
                                       tbody {
                                           tr {
                                               td style=(format!("width: {}px", TABLE_COL_WIDTH)) { "0" }
                                               td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                   {
                                                       p { "" }
                                                   }
                                               }
                                           }
                                       }

                                   }
                               }
                           }
                        }
                    }
                    Err(e) => {
                        res.poll_time = None;
                        html! {
                            #modbus_table {
                               div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table" hx-swap="innerHTML" {

                                   table class="interactive" {
                                       thead {
                                           tr {
                                               th {
                                                   "Register"
                                               }
                                               th { "Value" }
                                           }
                                       }
                                       tbody {
                                           tr {
                                               td style=(format!("width: {}px", TABLE_COL_WIDTH)) { "0" }
                                               td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                   {
                                                       p { (format!("{:?}", e)) }
                                                   }
                                               }
                                           }
                                       }

                                   }
                               }
                           }
                        }
                    }
                }
            }
        },
        None => {
            html! {
                #modbus_table {
                   div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table" hx-swap="innerHTML" {

                       table class="interactive" {
                           thead {
                               tr {
                                   th {
                                       "Register"
                                   }
                                   th { "Value" }
                                   th { "Value (HEX)" }
                               }
                           }
                           tbody {
                               tr {
                                   td style=(format!("width: {}px", TABLE_COL_WIDTH)) { "0" }
                                   td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                       {
                                           p { "No connection." }
                                       }
                                   }
                                   td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                       {
                                            ""
                                       }
                                   }
                               }
                           }

                       }
                   }
               }
            }
        }
    }
}
pub fn modbus_serial_body() -> Markup {
    html! {
        body {
            main {
                div class="window" style=(format!("margin: {}px; width: {}px", MARGIN, WINDOW_WIDTH)) {
                    div class="title-bar" {
                       div class="title-bar-text" { "Modbus Serial Settings" }
                    }
                    div class="window-body" {
                        form hx-post="/connect_modbus_serial" hx-target="#modbus_connect_content" hx-swap="innerHTML" {
                            fieldset {
                                legend { "Slave Settings" }
                                div class="field-row-stacked" style="width: 200px" {
                                    label for="com" { "COM Port: " }
                                    input type="text" id="com" name="com" value="COM4" {}
                                    label for="baudrate" { "Baudrate: " }
                                    input type="number" id="baudrate" name="baudrate" value="9600" {}
                                    label for="slave" { "Slave ID: " }
                                    input type="number" id="slave" name="slave" value="1" {}
                                    //button type="submit" { "Connect" }
                                    button hx-post="/connect_modbus_serial" { "Connect" }
                                    button hx-get="/disconnect_modbus" hx-target="#modbus_connect_content" hx-swap="innerHTML" { "Disconnect" }
                                }
                            }
                        }
                        form hx-post="/update_modbus" hx-target="#modbus_connect_content" hx-swap="innerHTML" {
                            fieldset {
                                legend { "Polling Options" }
                                div class="field-row-stacked" style="width: 200px" {
                                    label for="function" { "Function Code: " }
                                    select name="function" id="function" {
                                        option value="3" { "0x03-Read Holding Registers" }
                                        option value="4" { "0x04-Read Input Registers" }
                                        option value="1" { "0x01-Read Coils" }
                                    }
                                    label for="register" { "Register: " }
                                    input type="number" id="register" name="register" value="1" {}
                                    label for="count" { "Count: (Default 5)" }
                                    input type="number" id="count" name="count" value="5" {}
                                    label for="float32" { "Use double registers as float: " }
                                    select name="float32" id="function" {
                                        option value="int16" { "16 bit integer" }
                                        option value="f32" { "32 bit float" }
                                    }
                                    button hx-post="/update_modbus" { "Send" }
                                }
                            }
                        }
                        form hx-post="/write_modbus" hx-target="#modbus_connect_content" hx-swap="innerHTML" {
                            fieldset {
                                legend { "Write Options" }
                                details {
                                    summary { "Show" }
                                    div class="field-row-stacked" style="width: 200px" {
                                        label for="write_function" { "Function Code: " }
                                        select name="write_function" id="write_function" {
                                            option value="6" { "0x06-Write Holding Register" }
                                            option value="5" { "0x05-Write Coil" }
                                        }
                                        label for="register" { "Register: " }
                                        input type="number" id="register" name="register" value="1" {}
                                        label for="float32" { "Use double registers as float: " }
                                        select name="float32" id="function" {
                                            option value="int16" { "16 bit integer" }
                                            option value="f32" { "32 bit float" }
                                        }
                                        label for="value" { "Value: " }
                                        input type="number" id="value" name="value" value="1" {}
                                        button type="submit" { "Write" }
                                    }
                                }
                            }
                        }
                        div class="sunken-panel" style=(format!("height: {}px; width: {}px", TABLE_HEIGHT, TABLE_WIDTH)) {
                            #modbus_table {
                                table class="interactive" {
                                    thead {
                                        tr {
                                            th {
                                                "Register"
                                            }
                                            th { "Value" }
                                            th { "Value (HEX)" }
                                        }
                                    }
                                    tbody {
                                        tr {
                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) { "0" }
                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                {
                                                    p { "This is Modbus data." }
                                                }
                                            }
                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                {
                                                    p { "" }
                                                }
                                            }
                                        }
                                    }
                                }

                            }
                        }
                        div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table"  hx-swap="innerHTML" {}
                    }
                    // Status bar
                    (modbus_status_bar())
                }

            }
        }

    }
}
pub fn modbus_tcp_body() -> Markup {
    html! {
        body {
            main {
                div class="window" style=(format!("margin: {}px; width: {}px", MARGIN, WINDOW_WIDTH)) {
                    div class="title-bar" {
                       div class="title-bar-text" { "Modbus TCP Settings" }
                    }
                    div class="window-body" {
                        form hx-post="/connect_modbus_tcp" hx-target="#modbus_connect_content" hx-swap="innerHTML" {
                            fieldset {
                                legend { "Slave Settings" }
                                div class="field-row-stacked" style="width: 200px" {
                                    label for="address" { "Address: " }
                                    input type="text" id="address" name="address" value="127.0.0.1" {}
                                    label for="port" { "Port: (Default 502)" }
                                    input type="number" id="port" name="port" value="5502" {}
                                    //button type="submit" { "Connect" }
                                    button hx-post="/connect_modbus_tcp" { "Connect" }
                                    button hx-get="/disconnect_modbus" hx-target="#modbus_connect_content" hx-swap="innerHTML" { "Disconnect" }
                                }
                            }
                        }
                        form hx-post="/update_modbus" hx-target="#modbus_connect_content" hx-swap="innerHTML" {
                            fieldset {
                                legend { "Polling Options" }
                                div class="field-row-stacked" style="width: 200px" {
                                    label for="function" { "Function Code: " }
                                    select name="function" id="function" {
                                        option value="3" { "0x03-Read Holding Registers" }
                                        option value="4" { "0x04-Read Input Registers" }
                                        option value="1" { "0x01-Read Coils" }
                                    }
                                    label for="register" { "Register: " }
                                    input type="number" id="register" name="register" value="1" {}
                                    label for="count" { "Count: (Default 5)" }
                                    input type="number" id="count" name="count" value="5" {}
                                    label for="float32" { "Use double registers as float: " }
                                    select name="float32" id="function" {
                                        option value="int16" { "16 bit integer" }
                                        option value="f32" { "32 bit float" }
                                    }
                                    button hx-post="/update_modbus" { "Send" }
                                }
                            }
                        }
                        form hx-post="/write_modbus" hx-target="#modbus_connect_content" hx-swap="innerHTML" {
                            fieldset {
                                legend { "Write Options" }
                                details {
                                    summary { "Show" }
                                    div class="field-row-stacked" style="width: 200px" {
                                        label for="write_function" { "Function Code: " }
                                        select name="write_function" id="write_function" {
                                            option value="6" { "0x06-Write Holding Register" }
                                            option value="5" { "0x05-Write Coil" }
                                        }
                                        label for="register" { "Register: " }
                                        input type="number" id="register" name="register" value="1" {}
                                        label for="float32" { "Use double registers as float: " }
                                        select name="float32" id="function" {
                                            option value="int16" { "16 bit integer" }
                                            option value="f32" { "32 bit float" }
                                        }
                                        label for="value" { "Value: " }
                                        input type="number" id="value" name="value" value="1" {}
                                        button type="submit" { "Write" }
                                    }

                                }
                            }
                        }

                        div class="sunken-panel" style=(format!("height: {}px; width: {}px", TABLE_HEIGHT, TABLE_WIDTH)) {
                            #modbus_table {
                                table class="interactive" {
                                    thead {
                                        tr {
                                            th {
                                                "Register"
                                            }
                                            th { "Value" }
                                            th { "Value (HEX)" }
                                        }
                                    }
                                    tbody {
                                        tr {
                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) { "0" }
                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                {
                                                    p { "This is Modbus data." }
                                                }
                                            }
                                            td style=(format!("width: {}px", TABLE_COL_WIDTH)) {
                                                {
                                                    p { "" }
                                                }
                                            }
                                        }
                                    }
                                }

                            }
                        }
                        div hx-get="/poll_modbus" hx-trigger="load delay:1s" hx-target="#modbus_table"  hx-swap="innerHTML" {}
                    }
                    // Status bar
                    (modbus_status_bar())
                }

            }
        }

    }
}
pub fn modbus_status_bar() -> Markup {
    html! {
        div class="status-bar" {
            #heartbeat {
                div hx-get="/heartbeat" hx-trigger="load delay:1s" hx-target="#heartbeat" hx-swap="innerHTML" {
                   p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)) {
                       "SCANTIME:   "
                   }
                }
            }
            #modbus_connect_content {
                p class="status-bar-field" style=(format!("width: {}px", STATUS_BAR_FIELD_WIDTH)) {  "STATUS:  No connection"  }
            }
        }
    }
}
