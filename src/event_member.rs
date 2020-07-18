use serde_json::{self, Result, Value};
use std::env;
use std::thread;
use std::io::{self, Write};
use serde_derive::{Serialize, Deserialize};
use failure::Error;

use log::{info, warn, error, trace};

use ::futures::Future;
use mysql;
use crossbeam_channel::{bounded, tick, Sender, Receiver, select};
use crate::event_room::*;
use crate::room::User;
use crate::room::ScoreInfo;

#[derive(Serialize, Deserialize)]
struct LoginData {
    id: String,
}

#[derive(Serialize, Deserialize)]
struct LogoutData {
    id: String,
}


pub fn login(id: String, v: Value, pool: mysql::Pool, sender: Sender<RoomEventData>, sender1: Sender<SqlData>)
 -> std::result::Result<(), Error>
{
    let data: LoginData = serde_json::from_value(v)?;
    sender.send(RoomEventData::Login(UserLoginData {u: User { id: id.clone(), hero: "default name".to_string(), honor: 50, online: true, ng1v1: ScoreInfo{score: 1000, WinCount: 0, LoseCount: 0}, ng5v5:ScoreInfo{score: 1000, WinCount: 0, LoseCount: 0}, rk1v1: ScoreInfo{score: 1000, WinCount: 0, LoseCount: 0}, rk5v5: ScoreInfo{score: 1000, WinCount: 0, LoseCount: 0}, ..Default::default()}, dataid: data.id}));
    Ok(())
    
}


pub fn logout(id: String, v: Value, pool: mysql::Pool, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: LogoutData = serde_json::from_value(v)?;
    let mut conn = pool.get_conn()?;
    let qres = conn.query(format!("update user set status='offline' where userid='{}';", data.id));
    let publish_packet = match qres {
        Ok(_) => {
            //sender.send(RoomEventData::Logout(UserLogoutData { id: id}));
        },
        _=> {
            
        }
    };
    sender.send(RoomEventData::Logout(UserLogoutData { id: id}));
    Ok(())
}

