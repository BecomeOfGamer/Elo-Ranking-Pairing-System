#![allow(warnings)]
use log::{info, warn, error, trace};

mod event_member;
mod event_room;
mod room;
mod msg;
mod elo;

use std::cell::RefCell;
use std::rc::Rc;
use std::env;
use std::io::{self, Write};
use failure::Error;
use std::net::TcpStream;
use std::str;
use clap::{App, Arg};
use uuid::Uuid;
use rumqtt::{MqttClient, MqttOptions, QoS, ReconnectOptions};

use serde_derive::{Serialize, Deserialize};
use std::panic;
use std::thread;
use std::time::Duration;
use std::fs::File;
use std::io::prelude::*;
use log::Level;
use serde_json::{self, Value};
use regex::Regex;

use ::futures::Future;
use mysql;
use room::PrestartStatus;

extern crate toml;
use crossbeam_channel::{bounded, tick, Sender, Receiver, select};
use crate::event_room::RoomEventData;
use crate::event_room::SqlData;
use crate::event_room::QueueData;
use crate::msg::*;

#[doc(include = "char.md")]
#[cfg(any(
    all(
        target_os = "linux",
        any(
            target_arch = "aarch64",
            target_arch = "arm",
            target_arch = "hexagon",
            target_arch = "powerpc",
            target_arch = "powerpc64",
            target_arch = "s390x",
        )
    ),
    all(target_os = "android", any(target_arch = "aarch64", target_arch = "arm")),
    all(target_os = "l4re", target_arch = "x86_64"),
    all(
        target_os = "freebsd",
        any(
            target_arch = "aarch64",
            target_arch = "arm",
            target_arch = "powerpc",
            target_arch = "powerpc64",
        )
    ),
    all(
        target_os = "netbsd",
        any(
            target_arch = "aarch64",
            target_arch = "arm",
            target_arch = "powerpc",
        )
    ),
    all(target_os = "openbsd", target_arch = "aarch64"),
    all(
        target_os = "vxworks",
        any(
            target_arch = "aarch64",
            target_arch = "arm",
            target_arch = "powerpc",
            target_arch = "powerpc64",
        )
    ),
    all(target_os = "fuchsia", target_arch = "aarch64"),
))]
#[stable(feature = "raw_os", since = "1.1.0")]
pub type c_char = u8;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ServerSetting {
    SERVER_IP: Option<String>,
    PORT: Option<String>,
    SQL_IP: Option<String>,
    MYSQL_ACCOUNT: Option<String>,
    MYSQL_PASSWORD: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Setting {
    server_setting: Option<ServerSetting>,
}

fn generate_client_id() -> String {
    let s = format!("Elo_Pub_{}", Uuid::new_v4());
    (&s[..16]).to_string()
}

fn get_url(setting: Setting) -> String {
    let s = format!("mysql://{}:{}@{}:3306/erps", setting.server_setting.clone().unwrap().MYSQL_ACCOUNT.unwrap(), setting.server_setting.clone().unwrap().MYSQL_PASSWORD.unwrap(), setting.server_setting.clone().unwrap().SQL_IP.unwrap());
    println!("mysql!: {}", s);
    s
}

fn main() -> std::result::Result<(), Error> {
    // configure logging
    env::set_var("RUST_LOG", env::var_os("RUST_LOG").unwrap_or_else(|| "info".into()));
    env_logger::init();

    let file_path = "src/setting.toml";
    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => panic!("no such file {} exception:{}", file_path, e)
    };
    let mut str_val = String::new();
    match file.read_to_string(&mut str_val) {
        Ok(s) => s
        ,
        Err(e) => panic!("Error Reading file: {}", e)
    };
    let setting: Setting = toml::from_str(&str_val).unwrap();

    let matches = App::new("erps")
        .author("damody <t1238142000@gmail.com>")
        .arg(
            Arg::with_name("SERVER")
                .short("S")
                .long("server")
                .takes_value(true)
                .help("MQTT server address (127.0.0.1)"),
        ).arg(
            Arg::with_name("PORT")
                .short("P")
                .long("port")
                .takes_value(true)
                .help("MQTT server port (1883)"),
        ).arg(
            Arg::with_name("USER_NAME")
                .short("u")
                .long("username")
                .takes_value(true)
                .help("Login user name"),
        ).arg(
            Arg::with_name("PASSWORD")
                .short("p")
                .long("password")
                .takes_value(true)
                .help("Password"),
        ).arg(
            Arg::with_name("CLIENT_ID")
                .short("i")
                .long("client-identifier")
                .takes_value(true)
                .help("Client identifier"),
        ).arg(
            Arg::with_name("BACKUP")
            .short("b")
            .long("backup")
            .takes_value(true)
            .help("backup"),
        ).get_matches();

    let server_addr = matches.value_of("SERVER").unwrap_or(&setting.server_setting.clone().unwrap().SERVER_IP.unwrap()).to_owned();
    let server_port = matches.value_of("PORT").unwrap_or(&setting.server_setting.clone().unwrap().PORT.unwrap()).to_owned();
    let client_id = matches
        .value_of("CLIENT_ID")
        .map(|x| x.to_owned())
        .unwrap_or("Elo Rank Server".to_owned());
    let mut isBackup: bool = matches.value_of("BACKUP").unwrap_or("false").to_owned().parse().unwrap();
    println!("Backup: {}", isBackup);
    let mut mqtt_options = MqttOptions::new(client_id.as_str(), server_addr.as_str(), server_port.parse::<u16>()?);
    mqtt_options = mqtt_options.set_keep_alive(100);
    mqtt_options = mqtt_options.set_request_channel_capacity(100000);
    mqtt_options = mqtt_options.set_notification_channel_capacity(100000);
    let (mut mqtt_client, notifications) = MqttClient::start(mqtt_options.clone())?;
    
    // Server message
    mqtt_client.subscribe("server/+/res/heartbeat", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("server/+/send/login", QoS::AtMostOnce).unwrap();


    mqtt_client.subscribe("manager/+/send/equ_test", QoS::AtMostOnce).unwrap();
    // User Equipment
    mqtt_client.subscribe("manager/+/send/insert_equ", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("manager/+/send/modify_userequ", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("manager/+/send/delete_userequ", QoS::AtMostOnce).unwrap();

    // Total Equipment
    mqtt_client.subscribe("manager/+/send/modify_equ", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("manager/+/send/new_equ", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("manager/+/send/delete_equ", QoS::AtMostOnce).unwrap();

    // Total Option
    mqtt_client.subscribe("manager/+/send/modify_option", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("manager/+/send/new_option", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("manager/+/send/delete_option", QoS::AtMostOnce).unwrap();
    

    // Client message
    mqtt_client.subscribe("member/+/send/login", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("member/+/send/logout", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("member/+/send/choose_hero", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("member/+/send/status", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("member/+/send/reconnect", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("member/+/send/replay", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("member/+/send/add_black_list", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("member/+/send/query_black_list", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("member/+/send/remove_black_list", QoS::AtMostOnce).unwrap();


    mqtt_client.subscribe("room/+/send/create", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/close", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/start_queue", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/cancel_queue", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/invite", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/join", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/accept_join", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/kick", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/leave", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/prestart", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/prestart_get", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("room/+/send/start", QoS::AtMostOnce).unwrap();

    mqtt_client.subscribe("game/+/send/game_close", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/game_over", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/game_info", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/start_game", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/choose", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/leave", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/exit", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/upload", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/result_upload", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/ban", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/choose", QoS::AtMostOnce).unwrap();
    mqtt_client.subscribe("game/+/send/rankgame_status", QoS::AtMostOnce).unwrap();
    
    let mut isServerLive = true;
    
    #[cfg(target_os = "linux")]
    let (tx, rx):(Sender<MqttMsg>, Receiver<MqttMsg>) = bounded(10000);
    #[cfg(not(target_os = "linux"))]
    let (tx, rx):(Sender<MqttMsg>, Receiver<MqttMsg>) = crossbeam::unbounded();
    let pool = mysql::Pool::new(get_url(setting.clone()).as_str())?;
    thread::sleep_ms(100);
    
    for _ in 0..8 {
        let server_addr = server_addr.clone();
        let server_port = server_port.clone();
        let rx1 = rx.clone();
        
        thread::spawn(move || -> Result<(), Error> {
            
            let mut mqtt_options = MqttOptions::new(generate_client_id(), server_addr, server_port.parse::<u16>()?);
            mqtt_options = mqtt_options.set_keep_alive(100);
            mqtt_options = mqtt_options.set_request_channel_capacity(100000);
            mqtt_options = mqtt_options.set_notification_channel_capacity(100000);
            //mqtt_options = mqtt_options.set_reconnect_opts(ReconnectOptions::Always(1));
            //println!("mqtt_options {:#?}", mqtt_options);
            let update = tick(Duration::from_millis(1000));
            let (mut mqtt_client, notifications) = MqttClient::start(mqtt_options.clone())?;
            loop {
                select! {
                    recv(rx1) -> d => {
                        let handle = || -> Result<(), Error> 
                        {
                            if let Ok(d) = d {
                                //println!("topic {}", d.topic);
                                if d.topic == "server/0/res/dead" {
                                    isServerLive = false;
                                    isBackup = false;
                                }
                                if d.topic.len() > 2 {
                                    let msg_res = mqtt_client.publish(d.topic, QoS::AtMostOnce, false, d.msg);
                                    match msg_res {
                                        Ok(_) =>{},
                                        Err(x) => {
                                            panic!("??? {}", x);
                                        }
                                    }
                                }
                            }
                            Ok(())
                        };
                        
                        if let Err(msg) = handle() {
                            panic!("mqtt {:?}", msg);
                            let (mut mqtt_client, notifications) = MqttClient::start(mqtt_options.clone())?;
                        }
                    }
                }
            }
            
            Ok(())
        });
    }
    
    let server = Regex::new(r"\w+/(\w+)/res/(\w+)").unwrap();
    #[cfg(target_os = "linux")]
    let check_server = tick(Duration::from_millis(1000));

    #[cfg(not(target_os = "linux"))]
    let (txxx, check_server) = crossbeam_channel::unbounded();
    #[cfg(not(target_os = "linux"))]
    std::thread::spawn(move || {
        println!("not linux!!!!!!!");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            txxx.try_send(std::time::Instant::now()).unwrap();
            //println!("update5000ms: rx len: {}, tx len: {}", rx.len(), txxx.len());
        }
    });
    

    let relogin = Regex::new(r"(\w+)/(((\w+)(\-)*)+)/send/login").unwrap();
    let relogout = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/logout").unwrap();
    let readd_black_list = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/add_black_list").unwrap();
    let requery_black_list = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/query_black_list").unwrap();
    let reremove_black_list = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/remove_black_list").unwrap();
    let recreate = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/create").unwrap();
    let reclose = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/close").unwrap();
    let restart_queue = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/start_queue").unwrap();
    let recancel_queue = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/cancel_queue").unwrap();
    let represtart_get = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/prestart_get").unwrap();
    let represtart = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/prestart").unwrap();
    let reinvite = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/invite").unwrap();
    let rejoin = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/join").unwrap();
    let reset = Regex::new(r"reset").unwrap();
    let rechoosehero = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/choose_hero").unwrap();
    let releave = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/leave").unwrap();
    let restart_game = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/start_game").unwrap();
    let regame_over = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/game_over").unwrap();
    let regame_info = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/game_info").unwrap();
    let regame_close = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/game_close").unwrap();
    let restatus = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/status").unwrap();
    let rereconnect = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/reconnect").unwrap();
    let regetRP = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/replay").unwrap();
    let reuploadRP = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/upload").unwrap();
    let reuploadRPRes = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/result_upload").unwrap();
    let reban = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/ban").unwrap();
    let rechoose = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/choose").unwrap();
    let rerankgame_status = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/rankgame_status").unwrap();

    let reequ_test = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/equ_test").unwrap();
    // User Equipment
    let reinsert_equ = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/insert_equ").unwrap();
    let remodify_userequ = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/modify_userequ").unwrap();
    let redelete_userequ = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/delete_userequ").unwrap();
    
    // Total Equipment
    let remodify_equ = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/modify_equ").unwrap();
    let recreate_equ = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/new_equ").unwrap();
    let redelete_equ = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/delete_equ").unwrap();

    // Total Option
    let remodify_option = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/modify_option").unwrap();
    let renew_option = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/new_option").unwrap();
    let redelete_option = Regex::new(r"\w+/(((\w+)(\-)*)+)/send/delete_option").unwrap();
    
    // let relogin = Regex::new(r"(\w+)/(\w+)/send/login").unwrap();
    // let relogout = Regex::new(r"\w+/(\w+)/send/logout").unwrap();
    // let recreate = Regex::new(r"\w+/(\w+)/send/create").unwrap();
    // let reclose = Regex::new(r"\w+/(\w+)/send/close").unwrap();
    // let restart_queue = Regex::new(r"\w+/(\w+)/send/start_queue").unwrap();
    // let recancel_queue = Regex::new(r"\w+/(\w+)/send/cancel_queue").unwrap();
    // let represtart_get = Regex::new(r"\w+/(\w+)/send/prestart_get").unwrap();
    // let represtart = Regex::new(r"\w+/(\w+)/send/prestart").unwrap();
    // let reinvite = Regex::new(r"\w+/(\w+)/send/invite").unwrap();
    // let rejoin = Regex::new(r"\w+/(\w+)/send/join").unwrap();
    // let reset = Regex::new(r"reset").unwrap();
    // let rechoosehero = Regex::new(r"\w+/(\w+)/send/choose_hero").unwrap();
    // let releave = Regex::new(r"\w+/(\w+)/send/leave").unwrap();
    // let restart_game = Regex::new(r"\w+/(\w+)/send/start_game").unwrap();
    // let regame_over = Regex::new(r"\w+/(\w+)/send/game_over").unwrap();
    // let regame_info = Regex::new(r"\w+/(\w+)/send/game_info").unwrap();
    // let regame_close = Regex::new(r"\w+/(\w+)/send/game_close").unwrap();
    // let restatus = Regex::new(r"\w+/(\w+)/send/status").unwrap();
    // let rereconnect = Regex::new(r"\w+/(\w+)/send/reconnect").unwrap();
    // let regetRP = Regex::new(r"\w+/(\w+)/send/replay").unwrap();
    // let reuploadRP = Regex::new(r"\w+/(\w+)/send/upload").unwrap();
    
    // // User Equipment
    // let reinsert_equ = Regex::new(r"\w+/(\w+)/send/insert_equ").unwrap();
    // let remodify_userequ = Regex::new(r"\w+/(\w+)/send/modify_userequ").unwrap();
    // let redelete_userequ = Regex::new(r"\w+/(\w+)/send/delete_userequ").unwrap();
    
    // // Total Equipment
    // let remodify_equ = Regex::new(r"\w+/(\w+)/send/modify_equ").unwrap();
    // let recreate_equ = Regex::new(r"\w+/(\w+)/send/new_equ").unwrap();
    // let redelete_equ = Regex::new(r"\w+/(\w+)/send/delete_equ").unwrap();


    //let mut QueueSender: Sender<QueueData>;
    let mut sender1: Sender<SqlData> = event_room::HandleSqlRequest(pool.clone())?;
    let mut sender: Sender<RoomEventData> = event_room::init(tx.clone(), sender1.clone(), pool.clone(), server_addr.clone(), isBackup)?;
    let update = tick(Duration::from_millis(500));
    let mut is_live = true;
    let mut sender = sender.clone();
    
    loop {
        use rumqtt::Notification::Publish;
        
        select! {
            
            recv (check_server) -> _ => {
                if isBackup {
                    //println!("isServerLive {}", isServerLive);
                    if isServerLive == true {
                        isServerLive = false;
                    }
                    else {
                        println!("Main Server dead!!");
                        event_room::server_dead("0".to_string(), sender.clone())?;
                        isBackup = false;
                    }
                }
            },
            recv(notifications) -> notification => {
                if !is_live{
                    println!("Reconnect!");
                    
                    let mut sender1: Sender<RoomEventData> = event_room::init(tx.clone(), sender1.clone(), pool.clone(), server_addr.clone(), isBackup)?;
                    sender = sender1.clone();
                    
                    
                    thread::sleep(Duration::from_millis(2000));
                    
                }
                let handle = || -> Result<(), Error> {
                    
                    if let Ok(x) = notification {
                        if let Publish(x) = x {                         
                            let payload = x.payload;
                            let msg = match str::from_utf8(&payload[..]) {
                                Ok(msg) => msg,
                                Err(err) => {
                                    return Err(failure::err_msg(format!("Failed to decode publish message {:?}", err)));
                                    //continue;
                                }
                            };
                            let topic_name = x.topic_name.as_str();
                            let vo : serde_json::Result<Value> = serde_json::from_str(msg);
                            //println!("topic: {}", topic_name);
                            if server.is_match(topic_name) {
                                //println!("topic: {}", topic_name);
                                isServerLive = true;
                            }
                            if reset.is_match(topic_name) {
                                //info!("reset");
                                sender.send(RoomEventData::Reset());
                            }
                            let vo : serde_json::Result<Value> = serde_json::from_str(msg);
                            if let Ok(v) = vo {
                                if reinvite.is_match(topic_name) {
                                    let cap = reinvite.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("invite: userid: {} json: {:?}", userid, v);
                                    event_room::invite(userid, v, sender.clone())?;
                                } else if rechoosehero.is_match(topic_name) {
                                    let cap = rechoosehero.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("choose ng hero: userid: {} json: {:?}", userid, v);
                                    event_room::choose_ng_hero(userid, v, sender.clone())?;
                                } else if rejoin.is_match(topic_name) {
                                    let cap = rejoin.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("join: userid: {} json: {:?}", userid, v);
                                    event_room::join(userid, v, sender.clone())?;
                                } else if relogin.is_match(topic_name) {
                                    let cap = relogin.captures(topic_name).unwrap();
                                    let userid = cap[2].to_string();
                                    //println!("{:?}", cap);
                                    if cap[1].to_string() == "server" {
                                        info!("Server Login!");
                                        event_room::server_login(userid, v, sender.clone())?;
                                    } else {
                                        //info!("login: userid: {} json: {:?}", userid, v);
                                        event_member::login(userid, v, pool.clone(), sender.clone(), sender1.clone())?;
                                    }
                                } else if relogout.is_match(topic_name) {
                                    let cap = relogout.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("logout: userid: {} json: {:?}", userid, v);
                                    event_member::logout(userid, v, pool.clone(), sender.clone())?;
                                } else if recreate.is_match(topic_name) {
                                    let cap = recreate.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("create: userid: {} json: {:?}", userid, v);
                                    event_room::create(userid, v, sender.clone())?;
                                } else if reclose.is_match(topic_name) {
                                    let cap = reclose.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("close: userid: {} json: {:?}", userid, v);
                                    event_room::close(userid, v, sender.clone())?;
                                } else if restart_queue.is_match(topic_name) {
                                    let cap = restart_queue.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("start_queue: userid: {} json: {:?}", userid, v);
                                    event_room::start_queue(userid, v, sender.clone())?;
                                } else if recancel_queue.is_match(topic_name) {
                                    let cap = recancel_queue.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("cancel_queue: userid: {} json: {:?}", userid, v);
                                    event_room::cancel_queue(userid, v, sender.clone())?;
                                } else if represtart_get.is_match(topic_name) {
                                    let cap = represtart_get.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("prestart_get: userid: {} json: {:?}", userid, v);
                                    event_room::prestart_get(userid, v, sender.clone())?;
                                }  else if represtart.is_match(topic_name) {
                                    let cap = represtart.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("prestart: userid: {} json: {:?}", userid, v);
                                    event_room::prestart(userid, v, sender.clone())?;
                                } else if releave.is_match(topic_name) {
                                    let cap = releave.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("leave: userid: {} json: {:?}", userid, v);
                                    event_room::leave(userid, v, sender.clone())?;
                                } else if restart_game.is_match(topic_name) {
                                    let cap = restart_game.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("start_game: userid: {} json: {:?}", userid, v);
                                    event_room::start_game(userid, v, sender.clone())?;
                                } else if regame_over.is_match(topic_name) {
                                    let cap = regame_over.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("game_over: userid: {} json: {:?}", userid, v);
                                    event_room::game_over(userid, v, sender.clone())?;
                                } else if regame_info.is_match(topic_name) {
                                    let cap = regame_info.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("game_info: userid: {} json: {:?}", userid, v);
                                    event_room::game_info(userid, v, sender.clone())?;
                                } else if regame_close.is_match(topic_name) {
                                    let cap = regame_close.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("game_close: userid: {} json: {:?}", userid, v);
                                    event_room::game_close(userid, v, sender.clone())?;
                                } else if restatus.is_match(topic_name) {
                                    let cap = restatus.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("status: userid: {} json: {:?}", userid, v);
                                    event_room::status(userid, v, sender.clone())?;
                                } else if rereconnect.is_match(topic_name) {
                                    let cap = rereconnect.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::reconnect(userid, v, sender.clone())?;
                                } else if reban.is_match(topic_name) {
                                    println!("Rank Ban");
                                    let cap = reban.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::ban(userid, v, sender.clone())?;
                                } else if rechoose.is_match(topic_name) {
                                    println!("Rank Choose");
                                    let cap = rechoose.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::choose(userid, v, sender.clone())?;
                                } else if rerankgame_status.is_match(topic_name) {
                                    println!("Rank Status");
                                    let cap = rerankgame_status.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::rankgame_status(userid, v, sender.clone())?;
                                } else if regetRP.is_match(topic_name) {
                                    let cap = regetRP.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::getRP(userid, v, sender.clone())?;
                                } else if reuploadRP.is_match(topic_name) {
                                    println!("Upload Replay");
                                    
                                    let cap = reuploadRP.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::uploadRP(userid, v, sender.clone())?;
                                } else if reuploadRPRes.is_match(topic_name) {
                                    println!("Upload Res");
                                    
                                    let cap = reuploadRPRes.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::uploadRPRes(userid, v, sender.clone())?;
                                } else if reinsert_equ.is_match(topic_name) {
                                    let cap = reinsert_equ.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::insert_equ(userid, v, sender.clone())?;
                                } else if remodify_equ.is_match(topic_name) {
                                    let cap = remodify_equ.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::modify_equ(userid, v, sender.clone())?;
                                } else if recreate_equ.is_match(topic_name) {
                                    let cap = recreate_equ.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::create_equ(userid, v, sender.clone())?;
                                } else if redelete_equ.is_match(topic_name) {
                                    let cap = redelete_equ.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    //info!("reconnect: userid: {} json: {:?}", userid, v);
                                    event_room::delete_equ(userid, v, sender.clone())?;
                                } else if redelete_userequ.is_match(topic_name) {
                                    let cap = redelete_userequ.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    event_room::delete_userequ(userid, v, sender.clone())?;
                                } else if remodify_userequ.is_match(topic_name) {
                                    let cap = remodify_userequ.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    event_room::modify_userequ(userid, v, sender.clone())?;
                                } else if renew_option.is_match(topic_name) {
                                    //println!("New Option");
                                    let cap = renew_option.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    event_room::new_option(userid, v, sender.clone())?;
                                } else if remodify_option.is_match(topic_name) {
                                    //println!("Modify Option");
                                    let cap = remodify_option.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    event_room::modify_option(userid, v, sender.clone())?;
                                } else if redelete_option.is_match(topic_name) {
                                    //println!("Delete Option");
                                    let cap = redelete_option.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    event_room::delete_option(userid, v, sender.clone())?;
                                } else if readd_black_list.is_match(topic_name) {
                                    let cap = readd_black_list.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    event_room::add_black_list(userid, v, sender.clone())?;
                                } else if requery_black_list.is_match(topic_name) {
                                    let cap = requery_black_list.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    event_room::query_black_list(userid, v, sender.clone())?;
                                } else if reremove_black_list.is_match(topic_name) {
                                    let cap = reremove_black_list.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    event_room::remove_black_list(userid, v, sender.clone())?;
                                } else if reequ_test.is_match(topic_name) {
                                    let cap = reequ_test.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    event_room::equ_test(userid, v, sender.clone())?;
                                }
                            } else {
                                warn!("Json Parser error");
                            };
                        }
                    }
                    Ok(())
                };
                if let Err(msg) = handle() {
                    println!("{:?}", msg);
                }
            }
        }
    }
    Ok(())
}
