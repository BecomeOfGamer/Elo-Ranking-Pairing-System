use serde_json::{self, Value};
use std::env;
use std::io::{self, Write};
use serde_derive::{Serialize, Deserialize};
use serde_json::json;
use std::io::{ErrorKind};
use log::{info, warn, error, trace};
use std::thread;
use std::panic;
use std::time::{Duration, Instant};

use ::futures::Future;
use mysql;
use std::sync::{Arc, Mutex, Condvar, RwLock};
use crossbeam_channel::{bounded, tick, Sender, Receiver, select};
use std::collections::{HashMap, BTreeMap};
use std::cell::RefCell;
use std::rc::Rc;
use failure::Error;
use rayon::prelude::*;
use rayon::slice::*;

use crate::room::*;
use crate::msg::*;
use crate::elo::*;
use std::process::Command;

const TEAM_SIZE: i16 = 1;
const MATCH_SIZE: usize = 2;
const SCORE_INTERVAL: i16 = 100;
const RECENT_GAME_NUM: usize = 2;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateRoomData {
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CloseRoomData {
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InviteRoomData {
    pub room: String,
    pub invite: String,
    pub from: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JoinRoomData {
    pub room: String,
    pub join: String,
}

#[derive(Clone, Debug)]
pub struct UserLoginData {
    pub u: User,
    pub dataid: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserNGHeroData {
    pub id: String,
    pub hero: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserLogoutData {
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StartQueueData {
    pub id: String,
    pub action: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CancelQueueData {
    pub id: String,
    pub action: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreGameData {
    pub rid: Vec<Vec<u32>>,
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreStartData {
    pub room: String,
    pub id: String,
    pub accept: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreStartGetData {
    pub room: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LeaveData {
    pub room: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StartGameData {
    pub game: u32,
    pub action: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StartGameSendData {
    pub game: u32,  
    pub member: Vec<HeroCell>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HeroCell {
    pub id: String,
    pub team: u16,
    pub name: String,
    pub hero: String,
    pub buff: BTreeMap<String, f32>,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GameOverData {
    pub game: u32,  
    pub win: Vec<String>,
    pub lose: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GameCloseData {
    pub game: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StatusData {
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReconnectData {
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DeadData {
    pub ServerDead: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GameInfoData {
    pub game: u32,
    pub users: Vec<UserInfoData>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserInfoData {
    pub id: String,
    pub hero: String,
    pub level: u16,
    pub equ: Vec<String>,
    pub damage: u16,
    pub take_damage: u16, 
    pub heal: u16,
    pub kill: u16,
    pub death: u16,
    pub assist: u16,
    pub gift: UserGift,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserGift {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub d: u16,
    pub e: u16,
}

#[derive(Debug)]
pub enum RoomEventData {
    Reset(),
    Login(UserLoginData),
    Logout(UserLogoutData),
    Create(CreateRoomData),
    Close(CloseRoomData),
    ChooseNGHero(UserNGHeroData),
    Invite(InviteRoomData),
    Join(JoinRoomData),
    StartQueue(StartQueueData),
    CancelQueue(CancelQueueData),
    UpdateGame(PreGameData),
    PreStart(PreStartData),
    PreStartGet(PreStartGetData),
    Leave(LeaveData),
    StartGame(StartGameData),
    GameOver(GameOverData),
    GameInfo(GameInfoData),
    GameClose(GameCloseData),
    Status(StatusData),
    Reconnect(ReconnectData),
    MainServerDead(DeadData),
}

#[derive(Clone, Debug)]
pub struct SqlLoginData {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct SqlScoreData {
    pub id: String,
    pub score: i16,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SqlGameInfoData {
    pub game : u32,
    pub id: String,
    pub hero: String,
    pub level: u16,
    pub equ: String,
    pub damage: u16, 
    pub take_damage: u16,
    pub heal: u16,
    pub kill: u16,
    pub death: u16,
    pub assist: u16,
    pub gift: UserGift,
}


pub enum SqlData {
    Login(SqlLoginData),
    UpdateScore(SqlScoreData),
    UpdateGameInfo(SqlGameInfoData)
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QueueRoomData {
    pub user_name: Vec<String>,
    pub rid: u32,
    pub gid: u32,
    pub user_len: i16,
    pub avg_ng: i16,
    pub avg_rk: i16,
    pub ready: i8,
    pub queue_cnt: i16,
    pub block: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReadyGroupData {
    pub user_name: Vec<String>,
    pub gid: u32,
    pub rid: Vec<u32>,
    pub user_len: i16,
    pub avg_ng: i16,
    pub avg_rk: i16,
    pub game_status: u16,
    pub queue_cnt: i16,
    pub block: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReadyGameData {
    pub gid: Vec<u32>,
    pub group: Vec<Vec<u32>>,
    pub team_len: usize,
    pub block: Vec<String>,
}

pub struct RemoveRoomData {
    pub rid: u32,
}

pub enum QueueData {
    UpdateRoom(QueueRoomData),
    RemoveRoom(RemoveRoomData),
}

// Prints the elapsed time.
fn show(dur: Duration) {
    println!(
        "Elapsed: {}.{:03} sec",
        dur.as_secs(),
        dur.subsec_nanos() / 1_000_000
    );
}

fn SendGameList(game: &Rc<RefCell<FightGame>>, msgtx: &Sender<MqttMsg>, conn: &mut mysql::PooledConn)
    -> Result<(), Error> {
    let mut res: StartGameSendData = Default::default();
    res.game = game.borrow().game_id;
    for (i, t) in game.borrow().teams.iter().enumerate() {
        let ids = t.borrow().get_users_id_hero();
        for (id, name, hero) in &ids {
            let h: HeroCell = HeroCell {id:id.clone(), team: (i+1) as u16, name:name.clone(), hero:hero.clone(), ..Default::default() };
            res.member.push(h);
        }
    }
    //info!("{:?}", res);
    msgtx.try_send(MqttMsg{topic:format!("game/{}/res/start_game", res.game), 
                                        msg: json!(res).to_string()})?;
    Ok(())
}

fn get_rid_by_id(id: &String, users: &BTreeMap<String, Rc<RefCell<User>>>) -> u32 {
    let u = users.get(id);
    if let Some(u) = u {
        return u.borrow().rid;
    }
    return 0;
}

fn get_gid_by_id(id: &String, users: &BTreeMap<String, Rc<RefCell<User>>>) -> u32 {
    let u = users.get(id);
    if let Some(u) = u {
        return u.borrow().gid;
    }
    return 0;
}

fn get_game_id_by_id(id: &String, users: &BTreeMap<String, Rc<RefCell<User>>>) -> u32 {
    let u = users.get(id);
    if let Some(u) = u {
        return u.borrow().game_id;
    }
    return 0;
}

fn get_user(id: &String, users: &BTreeMap<String, Rc<RefCell<User>>>) -> Option<Rc<RefCell<User>>> {
    let u = users.get(id);
    if let Some(u) = u {
        return Some(Rc::clone(u))
    }
    None
}

fn get_users(ids: &Vec<String>, users: &BTreeMap<String, Rc<RefCell<User>>>) -> Result<Vec<Rc<RefCell<User>>>, Error> {
    let mut res: Vec<Rc<RefCell<User>>> = vec![];
    for id in ids {
        let u = get_user(id, users);
        if let Some(u) = u {
            res.push(u);
        }
    }
    if ids.len() == res.len() {
        Ok(res)
    }
    else {
        Err(failure::err_msg("some user not found"))
    }
}

fn user_score(u: &Rc<RefCell<User>>, value: i16, msgtx: &Sender<MqttMsg>, sender: &Sender<SqlData>, conn: &mut mysql::PooledConn) -> Result<(), Error> {
    u.borrow_mut().ng += value;
    msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", u.borrow().id), 
        msg: format!(r#"{{"msg":"ok", "ng":{}, "rk":{} }}"#, u.borrow().ng, u.borrow().rk)})?;
    //println!("Update!");
    sender.send(SqlData::UpdateScore(SqlScoreData {id: u.borrow().id.clone(), score: u.borrow().ng.clone()}));
        //let sql = format!("UPDATE user_ng as a JOIN user as b ON a.id=b.id SET score={} WHERE b.userid='{}';", u.borrow().ng, u.borrow().id);
    //println!("sql: {}", sql);
    //let qres = conn.query(sql.clone())?;
    Ok(())
}

fn get_ng(team : &Vec<Rc<RefCell<User>>>) -> Vec<i32> {
    let mut res: Vec<i32> = vec![];
    for u in team {
        res.push(u.borrow().ng.into());
    }
    res
}

fn get_rk(team : &Vec<Rc<RefCell<User>>>) -> Vec<i32> {
    let mut res: Vec<i32> = vec![];
    for u in team {
        res.push(u.borrow().rk.into());
    }
    res
}

fn settlement_ng_score(win: &Vec<Rc<RefCell<User>>>, lose: &Vec<Rc<RefCell<User>>>, msgtx: &Sender<MqttMsg>, sender: &Sender<SqlData>, conn: &mut mysql::PooledConn) {
    if win.len() == 0 || lose.len() == 0 {
        return;
    }
    let win_ng = get_ng(win);
    let lose_ng = get_ng(lose);
    let elo = EloRank {k:20.0};
    let (rw, rl) = elo.compute_elo_team(&win_ng, &lose_ng);
    println!("Game Over");
    for (i, u) in win.iter().enumerate() {
        user_score(u, (rw[i]-win_ng[i]) as i16, msgtx, sender, conn);
    }
    for (i, u) in lose.iter().enumerate() {
        user_score(u, (rl[i]-lose_ng[i]) as i16, msgtx, sender, conn);
    }
}

pub fn HandleSqlRequest(pool: mysql::Pool)
    -> Result<Sender<SqlData>, Error> {
        let (tx1, rx1): (Sender<SqlData>, Receiver<SqlData>) = bounded(10000);
        let start = Instant::now();
        let update1000ms = tick(Duration::from_millis(2000));
        let mut NewUsers: Vec<String> = Vec::new();
        let mut len = 0;
        let mut UpdateInfo: Vec<SqlGameInfoData> = Vec::new();
        let mut info_len = 0; 

        thread::spawn(move || -> Result<(), Error> {
            let mut conn = pool.get_conn()?;
            loop{
                select! {

                    recv(update1000ms) -> _ => {
                        if len > 0 {
                            // insert new user in NewUsers
                            let mut insert_str: String = "insert into user (userid, name, status) values".to_string();
                            for (i, u) in NewUsers.iter().enumerate() {
                                let mut new_user = format!(" ('{}', 'default name', 'online')", u);
                                insert_str += &new_user;
                                if i < len-1 {
                                    insert_str += ",";
                                }
                            }
                            insert_str += ";";
                            //println!("{}", insert_str);
                            {
                                conn.query(insert_str.clone())?;
                            }

                            let sql = format!("select id from user where userid='{}';", NewUsers[0]);
                            //println!("sql: {}", sql);
                            let qres = conn.query(sql.clone())?;
                            let mut id = 0;
                            let mut name: String = "".to_owned();
                            for row in qres {
                                let a = row?.clone();
                                id = mysql::from_value(a.get("id").unwrap());
                            }

                            let mut insert_rk: String = "insert into user_rank (id, score) values".to_string();
                            for i in 0..len {
                                let mut new_user = format!(" ({}, 1000)", id+i);
                                insert_rk += & new_user;
                                if i < len-1 {
                                    insert_rk += ",";
                                }
                            }
                            insert_rk += ";";
                            //println!("{}", insert_rk);
                            {
                                conn.query(insert_rk.clone())?;
                            }

                            let mut insert_ng: String = "insert into user_ng (id, score) values".to_string();
                            for i in 0..len {
                                let mut new_user = format!(" ({}, 1000)", id+i);
                                insert_ng += &new_user;
                                if i < len-1 {
                                    insert_ng += ",";
                                }
                            }
                            insert_ng += ";";
                            //println!("{}", insert_ng);
                            {
                                conn.query(insert_ng.clone())?;
                            }
                            
                            len = 0;
                            NewUsers.clear();
                        }

                        if info_len > 0 {
                            
                            let mut insert_info: String = "insert into game_info (userid, game_id, hero, level, damage, take_damage, heal, kill_cnt, death, assist) values".to_string();
                            let mut user_info: String = "insert into user_info (userid, game_id, equ, gift_A, gift_B, gift_C, gift_D, gift_E) values".to_string();
                            for (i, info) in UpdateInfo.iter().enumerate() {
                                let mut new_user = format!(" ({}, {}, '{}', {}, {}, {}, {}, {}, {}, {})", info.id, info.game, info.hero, info.level, info.damage, info.take_damage, info.heal, info.kill, info.death, info.assist);
                                insert_info += &new_user;
                                let mut new_user1 = format!(" ({}, {}, '{}', {}, {}, {}, {}, {})", info.id, info.game, info.equ, info.gift.a, info.gift.b, info.gift.c, info.gift.d, info.gift.e);
                                user_info += &new_user1;
                                if i < info_len-1 {
                                    insert_info += ",";
                                    user_info += ",";
                                }
                            }
                            insert_info += ";";
                            user_info += ";";
                            {
                                conn.query(insert_info.clone())?;
                            }
                            {
                                conn.query(user_info.clone())?;
                            }
                            

                            info_len = 0;
                            UpdateInfo.clear();
                        }

                    }

                    recv(rx1) -> d => {
                        let handle = || -> Result<(), Error> {
                            if let Ok(d) = d {
                                match d {
                                    
                                    SqlData::Login(x) => {                                       
                                        NewUsers.push(x.id.clone());
                                        len+=1;                                        
                                    }
                                    SqlData::UpdateScore(x) => {
                                        //println!("in");
                                        let sql = format!("UPDATE user_ng as a JOIN user as b ON a.id=b.id SET score={} WHERE b.userid='{}';", x.score, x.id);
                                        //println!("sql: {}", sql);
                                        let qres = conn.query(sql.clone())?;
                                    }
                                    SqlData::UpdateGameInfo(x) => {
                                        UpdateInfo.push(x.clone());
                                        info_len += 1;
                                    }
                                }
                            }
                            Ok(())
                        };
                        if let Err(msg) = handle() {
                            println!("{:?}", msg);
                            continue;
                        }
                    }
                }
            }
        });
    Ok(tx1)
}

pub fn HandleQueueRequest(msgtx: Sender<MqttMsg>, sender: Sender<RoomEventData>)
    -> Result<Sender<QueueData>, Error> {
    let (tx, rx):(Sender<QueueData>, Receiver<QueueData>) = bounded(10000);
    let start = Instant::now();
    let update = tick(Duration::from_millis(1000));
        
    thread::spawn(move || -> Result<(), Error> {
        let mut QueueRoom: BTreeMap<u32, Rc<RefCell<QueueRoomData>>> = BTreeMap::new();
        let mut ReadyGroups: BTreeMap<u32, Rc<RefCell<ReadyGroupData>>> = BTreeMap::new();
        
        let mut group_id = 0;
        loop {
            select! {
                recv(update) -> _ => {
                    
                    let mut new_now = Instant::now();
                    if QueueRoom.len() >= MATCH_SIZE {
                        let mut g: ReadyGroupData = Default::default();
                        let mut tq: Vec<Rc<RefCell<QueueRoomData>>> = vec![];
                        let mut id: Vec<u32> = vec![];
                        let mut new_now = Instant::now();
                        tq = QueueRoom.iter().map(|x|Rc::clone(x.1)).collect();
                        //println!("Collect Time: {:?}",Instant::now().duration_since(new_now));
                        //tq.sort_by_key(|x| x.borrow().avg_rk);
                        
                        let mut new_now = Instant::now();
                        tq.sort_by_key(|x| x.borrow().avg_ng);
                        //println!("Sort Time: {:?}",Instant::now().duration_since(new_now));
                        let mut new_now1 = Instant::now();
                        for (k, v) in &mut QueueRoom {
                            let mut block: bool = false;
                            
                            for u in &v.borrow_mut().user_name {
                                
                                if g.block.contains(u) {
                                    block = true;
                                    break;
                                }
                            }
                            if block {
                                continue;
                            }
                            if g.user_len > 0 && g.user_len < TEAM_SIZE && (g.avg_ng + v.borrow().queue_cnt*SCORE_INTERVAL) < v.borrow().avg_ng {
                                for r in g.rid {
                                    id.push(r);
                                }
                                g = Default::default();
                                g.rid.push(v.borrow().rid);
                                g.block = [g.block.as_slice(), v.borrow().block.clone().as_slice()].concat();
                                g.user_name = [g.user_name.as_slice(), v.borrow().user_name.as_slice()].concat();
                               
                                let mut ng = (g.avg_ng * g.user_len + v.borrow().avg_ng * v.borrow().user_len) as i16 / (g.user_len + v.borrow().user_len) as i16;
                                g.avg_ng = ng;
                                g.user_len += v.borrow().user_len;
                                v.borrow_mut().ready = 1;
                                v.borrow_mut().gid = group_id + 1;
                                v.borrow_mut().queue_cnt += 1;
                                
                            }

                            if v.borrow().ready == 0 &&
                                v.borrow().user_len as i16 + g.user_len <= TEAM_SIZE {
                                
                                let Difference: i16 = i16::abs(v.borrow().avg_ng - g.avg_ng);
                                if g.avg_ng == 0 || Difference <= SCORE_INTERVAL * v.borrow().queue_cnt {
                                    g.rid.push(v.borrow().rid);
                                    g.block = [g.block.as_slice(), v.borrow().block.clone().as_slice()].concat();
                                    g.user_name = [g.user_name.as_slice(), v.borrow().user_name.as_slice()].concat();
                                    
                                    let mut ng = (g.avg_ng * g.user_len + v.borrow().avg_ng * v.borrow().user_len) as i16 / (g.user_len + v.borrow().user_len) as i16;
                                    g.avg_ng = ng;
                                    g.user_len += v.borrow().user_len;
                                    v.borrow_mut().ready = 1;
                                    v.borrow_mut().gid = group_id + 1;
                                }
                                else {
                                    v.borrow_mut().queue_cnt += 1;
                                }
                            }
                            if g.user_len == TEAM_SIZE {
                                //println!("match team_size!");
                                group_id += 1;
                                //info!("new group_id: {}", group_id);
                                g.gid = group_id;
                                
                                g.queue_cnt = 1;
                                ReadyGroups.insert(group_id, Rc::new(RefCell::new(g.clone())));
                                
                                g = Default::default();
                            }

                        }
                        //println!("Time 2: {:?}",Instant::now().duration_since(new_now1));
                        if g.user_len < TEAM_SIZE {
                            for r in g.rid {
                                let mut room = QueueRoom.get(&r);
                                if let Some(room) = room {
                                    room.borrow_mut().ready = 0;
                                    room.borrow_mut().gid = 0;
                                }
                            }
                            for r in id {
                                let mut room = QueueRoom.get(&r);
                                if let Some(room) = room {
                                    room.borrow_mut().ready = 0;
                                    room.borrow_mut().gid = 0;
                                }
                            }
                        }
                    }

                    if ReadyGroups.len() >= MATCH_SIZE {
                        let mut fg: ReadyGameData = Default::default();
                        //let mut prestart = false;
                        let mut total_ng: i16 = 0;
                        let mut rm_ids: Vec<u32> = vec![];
                        //println!("ReadyGroup!! {}", ReadyGroups.len());
                        let mut new_now2 = Instant::now();
                        for (id, rg) in &mut ReadyGroups {
                            
                            let mut block: bool = false;
                            
                            for u in &rg.borrow_mut().user_name {
                                
                                if fg.block.contains(u) {
                                    block = true;
                                    break;
                                }
                                
                            }
                            if block {
                                continue;
                            }
                            

                            if rg.borrow().game_status == 0 && fg.team_len < MATCH_SIZE {
                                if total_ng == 0 {
                                    total_ng += rg.borrow().avg_ng as i16;
                                    fg.group.push(rg.borrow().rid.clone());
                                    fg.gid.push(*id);
                                    fg.team_len += 1;
                                    fg.block = [fg.block.as_slice(), rg.borrow().block.clone().as_slice()].concat();
                                    continue;
                                }
                                
                                let mut difference = 0;
                                if fg.team_len > 0 {
                                    difference = i16::abs(rg.borrow().avg_ng as i16 - total_ng/fg.team_len as i16);
                                }
                                if difference <= SCORE_INTERVAL * rg.borrow().queue_cnt {
                                    total_ng += rg.borrow().avg_ng as i16;
                                    fg.group.push(rg.borrow().rid.clone());
                                    fg.block = [fg.block.as_slice(), rg.borrow().block.clone().as_slice()].concat();
                                    fg.team_len += 1;
                                    fg.gid.push(*id);
                                }
                                else {
                                    rg.borrow_mut().queue_cnt += 1;
                                }
                            }
                            if fg.team_len == MATCH_SIZE {
                                sender.send(RoomEventData::UpdateGame(PreGameData{rid: fg.group.clone()}));
                                for id in fg.gid {
                                    rm_ids.push(id);
                                }
                                fg = Default::default();
                            }
                        }
                        
                        for id in rm_ids {
                            let rg = ReadyGroups.remove(&id);
                            if let Some(rg) = rg {
                                for rid in &rg.borrow().rid {
                                    
                                    QueueRoom.remove(&rid);
                                    
                                }
                            }
                        }

                        //println!("Time 3: {:?}",Instant::now().duration_since(new_now2));
                    }
                    


                }

                recv(rx) -> d => {
                    let handle = || -> Result<(), Error> {
                        if let Ok(d) = d {
                            match d {
                                QueueData::UpdateRoom(x) => {
                                    //println!("rid: {}", x.rid);
                                    QueueRoom.insert(x.rid.clone(), Rc::new(RefCell::new(x.clone())));
                                }
                                QueueData::RemoveRoom(x) => {
                                    //println!("Remove Room!!!  x.rid: {}", &x.rid);
                                    let r = QueueRoom.get(&x.rid);
                                    if let Some(r) = r {
                                        //println!("Start Remove!! gid: {}", &r.borrow().gid);
                                        let mut rg = ReadyGroups.get(&r.borrow().gid);
                                        if let Some(rg) = rg {
                                            //println!("ReadyGroup Remove!!");
                                            for rid in &rg.borrow().rid {
                                                if rid == &x.rid {
                                                    continue;
                                                }
                                                let mut room = QueueRoom.get(rid);
                                                if let Some(room) = room {
                                                    
                                                    room.borrow_mut().gid = 0;
                                                    room.borrow_mut().ready = 0;
                                                    //println!("gid: {}, rid: {}, ready: {}", room.borrow().gid, room.borrow().rid, room.borrow().ready);
                                                }
                                            }
                                        }
                                        ReadyGroups.remove(&r.borrow().gid);
                                    }
                                    QueueRoom.remove(&x.rid);
                                }
                            }
                        }
                        Ok(()) 
                    };
                    if let Err(msg) = handle() {
                        println!("{:?}", msg);
                        continue;
                    }
                }
            }
        }
    });
    Ok(tx)
}


pub fn init(msgtx: Sender<MqttMsg>, sender: Sender<SqlData>, pool: mysql::Pool, QueueSender1: Option<Sender<QueueData>>, isBackup: bool) 
    -> Result<(Sender<RoomEventData>, Sender<QueueData>), Error> {
    let (tx, rx):(Sender<RoomEventData>, Receiver<RoomEventData>) = bounded(10000);
    let mut tx1: Sender<QueueData>;
    match QueueSender1.clone() {
        Some(s) => {
            tx1 = s;
            println!("in");
        },
        None => {
            tx1 = HandleQueueRequest(msgtx.clone(), tx.clone())?;
            println!("2 in");
        },
    }
    
    
    let start = Instant::now();
    let update5000ms = tick(Duration::from_millis(5000));
    let update200ms = tick(Duration::from_millis(200));
    let update1000ms = tick(Duration::from_millis(1000));
    let QueueSender = tx1.clone();

    let tx2 = tx.clone();
    thread::spawn(move || -> Result<(), Error> {
        let mut conn = pool.get_conn()?;
        let mut isServerLive = true;
        let mut isBackup = isBackup.clone();
        let mut TotalRoom: BTreeMap<u32, Rc<RefCell<RoomData>>> = BTreeMap::new();
        //let mut QueueRoom: BTreeMap<u32, Rc<RefCell<RoomData>>> = BTreeMap::new();
        let mut ReadyGroups: BTreeMap<u32, Rc<RefCell<FightGroup>>> = BTreeMap::new();
        let mut PreStartGroups: BTreeMap<u32, Rc<RefCell<FightGame>>> = BTreeMap::new();
        let mut GameingGroups: BTreeMap<u32, Rc<RefCell<FightGame>>> = BTreeMap::new();
        let mut TotalUsers: BTreeMap<String, Rc<RefCell<User>>> = BTreeMap::new();
        let mut LossSend: Vec<MqttMsg> = vec![];
        let mut room_id: u32 = 0;
        let mut group_id: u32 = 0;
        let mut game_id: u32 = 0;
        let mut game_port: u16 = 7777;

        let sql = format!(r#"select userid, a.score as ng, b.score as rk, name from user as c 
                            join user_ng as a on a.id=c.id 
                            join user_rank as b on b.id=c.id;"#);
        let qres2: mysql::QueryResult = conn.query(sql.clone())?;
        let mut userid: String = "".to_owned();
        let mut ng: i16 = 0;
        let mut rk: i16 = 0;
        let mut name: String = "".to_owned();
        let id = 0;
        for row in qres2 {
            let a = row?.clone();
            let user = User {
                id: mysql::from_value(a.get("userid").unwrap()),
                hero: mysql::from_value(a.get("name").unwrap()),
                online: false,
                ng: mysql::from_value(a.get("ng").unwrap()),
                rk: mysql::from_value(a.get("rk").unwrap()),
                ..Default::default()
            };
            userid = mysql::from_value(a.get("userid").unwrap());
            //println!("userid: {}", userid);
            //ng = mysql::from_value(a.get("ng").unwrap());
            //rk = mysql::from_value(a.get("rk").unwrap());
            //name = mysql::from_value(a.get("name").unwrap());
            TotalUsers.insert(userid, Rc::new(RefCell::new(user.clone())));
        }

        /*
        let get_game_id = format!("select MAX(game_id) from game_info;");
        let qres3: mysql::QueryResult = conn.query(get_game_id.clone())?;
        
        for row in qres3 {
            let a = row?.clone();
            game_id = mysql::from_value(a.get("MAX(game_id)").unwrap());
            
            //println!("game id: {}", game_id);
        }
        */
        println!("game id: {}", game_id);
        
        loop {
            select! {
                
                recv(update200ms) -> _ => {
                    //show(start.elapsed());
                    // update prestart groups
                    let mut rm_ids: Vec<u32> = vec![];
                    let mut start_cnt: u16 = 0;
                    for (id, group) in &mut PreStartGroups {
                        //if start_cnt >= 10 {
                        //    thread::sleep(Duration::from_millis(1000));
                        //    break;
                        //}
                        let res = group.borrow().check_prestart();
                        
                        match res {
                            
                            PrestartStatus::Ready => {
                                start_cnt += 1;
                                rm_ids.push(*id);
                                game_port += 1;
                                if game_port > 65500 {
                                    game_port = 7777;
                                }
                                group.borrow_mut().ready();
                                group.borrow_mut().update_names();
                                group.borrow_mut().game_port = game_port;
                                
                                GameingGroups.remove(&group.borrow().game_id);
                                //PreStartGroups.remove(&group.borrow().game_id);
                                GameingGroups.insert(group.borrow().game_id.clone(), group.clone());
                                //info!("game_port: {}", game_port);
                                //info!("game id {}", group.borrow().game_id);
                                
                                let cmd = Command::new("/home/damody/LinuxNoEditor/CF1/Binaries/Linux/CF1Server")
                                        .arg(format!("-Port={}", game_port))
                                        .arg(format!("-gameid {}", group.borrow().game_id))
                                        .arg("-NOSTEAM")
                                        .spawn();
                                        match cmd {
                                            Ok(_) => {},
                                            Err(_) => {warn!("Fail open CF1Server")},
                                        }
                                if !isBackup || (isBackup && isServerLive == false){
                                    msgtx.try_send(MqttMsg{topic:format!("game/{}/res/game_signal", group.borrow().game_id), 
                                        msg: format!(r#"{{"game":{}}}"#, group.borrow().game_id)})?;
                                    LossSend.push(MqttMsg{topic:format!("game/{}/res/game_signal", group.borrow().game_id), 
                                        msg: format!(r#"{{"game":{}}}"#, group.borrow().game_id)});
                                }
                                
                            },
                            PrestartStatus::Cancel => {
                                group.borrow_mut().update_names();
                                group.borrow_mut().clear_queue();
                                group.borrow_mut().game_status = 0;
                                let mut rm_rid: Vec<u32> = vec![]; 
                                let mut users: Vec<String> = vec![];
                                let mut block: Vec<String> = vec![];
                                for t in &group.borrow().teams {
                                    for c in &t.borrow().checks {
                                        if c.check < 0 {
                                            let u = TotalUsers.get(&c.id);
                                            if let Some(u) = u {
                                                rm_rid.push(u.borrow().rid);
                                            }
                                        }
                                    }
                                    for room in &t.borrow().rooms {
                                        for u in &room.borrow().users {
                                            
                                            users.push(u.borrow().id.clone());
                                            for index in &u.borrow().recent_users {
                                                block = [block.as_slice(), index.clone().as_slice()].concat();
                                            }
                                            block = [block.as_slice(), u.borrow().blacklist.as_slice()].concat();
                                            
                                        }
                                    }
                                }
                                for r in &group.borrow().room_names {
                                    if !isBackup || (isBackup && isServerLive == false) {
                                        msgtx.try_send(MqttMsg{topic:format!("room/{}/res/prestart", r), 
                                            msg: format!(r#"{{"msg":"stop queue"}}"#)})?;
                                        LossSend.push(MqttMsg{topic:format!("room/{}/res/prestart", r), 
                                            msg: format!(r#"{{"msg":"stop queue"}}"#)});
                                    }
                                }
                                for t in &group.borrow().teams {
                                    for r in &t.borrow().rooms {
                                        if !rm_rid.contains(&r.borrow().rid) {
                                            let mut data = QueueRoomData {
                                                user_name: users.clone(),
                                                rid: r.borrow().rid.clone(),
                                                gid: 0,
                                                user_len: r.borrow().users.len().clone() as i16,
                                                avg_ng: r.borrow().avg_ng.clone(),
                                                avg_rk: r.borrow().avg_rk.clone(),
                                                ready: 0,
                                                queue_cnt: 1,
                                                block: block.clone(),
                                            };
                                            QueueSender.send(QueueData::UpdateRoom(data));
                                        }
                                    }
                                }
                                rm_ids.push(*id);
                            },
                            PrestartStatus::Wait => {
                                
                            }
                        }
                    }
                    for k in rm_ids {   
                        PreStartGroups.remove(&k);
                    }
                }
                recv(update1000ms) -> _ => {
                    if !isBackup || (isBackup && isServerLive == false) {
                        //msgtx.try_send(MqttMsg{topic:format!("server/0/res/heartbeat"), 
                        //                    msg: format!(r#"{{"msg":"live"}}"#)})?;
                    }
                }

                recv(update5000ms) -> _ => {
                    //println!("rx len: {}, tx len: {}", rx.len(), tx2.len());
                    LossSend.clear();
                    for (id, group) in &mut PreStartGroups {
                        let res1 = group.borrow().check_prestart_get();
                        //println!("check result: {}", res1);
                        if res1 == false {
                            for r in &group.borrow().room_names {
                                if !isBackup || (isBackup && isServerLive == false) {
                                    msgtx.try_send(MqttMsg{topic:format!("room/{}/res/prestart", r), msg: r#"{"msg":"prestart"}"#.to_string()})?;
                                    LossSend.push(MqttMsg{topic:format!("room/{}/res/prestart", r), msg: r#"{"msg":"prestart"}"#.to_string()});
                                }
                            }
                            continue;
                        }
                    }
                }
                
                recv(rx) -> d => {
                    
                    let handle = || -> Result<(), Error> {
                        let mut mqttmsg: MqttMsg = MqttMsg{topic: format!(""), msg: format!("")};
                        if let Ok(d) = d {
                            match d {
                                RoomEventData::Status(x) => {
                                    let u = get_user(&x.id, &TotalUsers);
                                    if let Some(u) = u {
                                        if u.borrow().game_id != 0 {
                                            mqttmsg = MqttMsg{topic:format!("member/{}/res/status", x.id), 
                                                msg: format!(r#"{{"msg":"gaming"}}"#)};
                                            //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/status", x.id), 
                                            //    msg: format!(r#"{{"msg":"gaming"}}"#)})?;
                                        } else {
                                            mqttmsg = MqttMsg{topic:format!("member/{}/res/status", x.id), 
                                                msg: format!(r#"{{"msg":"normal"}}"#)};
                                            //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/status", x.id), 
                                            //    msg: format!(r#"{{"msg":"normal"}}"#)})?;
                                        }
                                    } else {
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/status", x.id), 
                                            msg: format!(r#"{{"msg":"id not found"}}"#)};
                                        //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/status", x.id), 
                                        //        msg: format!(r#"{{"msg":"id not found"}}"#)})?;
                                    }
                                    //info!("Status TotalUsers {:#?}", TotalUsers);
                                },
                                RoomEventData::Reconnect(x) => {
                                    let u = get_user(&x.id, &TotalUsers);
                                    if let Some(u) = u {
                                        let g = GameingGroups.get(&u.borrow().game_id);
                                        if let Some(g) = g {
                                            mqttmsg = MqttMsg{topic:format!("member/{}/res/reconnect", x.id), 
                                                msg: format!(r#"{{"server":"114.32.129.195:{}"}}"#, g.borrow().game_port)};
                                            //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/reconnect", x.id), 
                                            //    msg: format!(r#"{{"server":"114.32.129.195:{}"}}"#, g.borrow().game_port)})?;
                                        }
                                    }
                                },
                                RoomEventData::GameClose(x) => {
                                    //let p = PreStartGroups.remove(&x.game);
                                    let g = GameingGroups.remove(&x.game);
                                    if let Some(g) = g {
                                        for u in &g.borrow().user_names {
                                            let u = get_user(&u, &TotalUsers);
                                            match u {
                                                Some(u) => {
                                                    // remove room
                                                    let r = TotalRoom.get(&u.borrow().rid);
                                                    let mut is_null = false;
                                                    if let Some(r) = r {
                                                        r.borrow_mut().rm_user(&u.borrow().id);
                                                        r.borrow_mut().ready = 0;
                                                        if r.borrow().users.len() == 0 {
                                                            is_null = true;
                                                            //info!("remove success {}", u.borrow().id);
                                                        }
                                                    }
                                                    else {
                                                        //info!("remove fail {}", u.borrow().id);
                                                    }
                                                    if is_null {
                                                        TotalRoom.remove(&u.borrow().rid);
                                                        //QueueRoom.remove(&u.borrow().rid);
                                                    }
                                                    u.borrow_mut().rid = 0;
                                                    u.borrow_mut().gid = 0;
                                                    u.borrow_mut().game_id = 0;
                                                },
                                                None => {
                                                    //info!("remove fail ");
                                                }
                                            }
                                        }
                                        //info!("GameClose {}", x.game);
                                        //info!("TotalUsers {:#?}", TotalUsers);
                                    }
                                    else {
                                        //info!("GameingGroups {:#?}", GameingGroups);
                                    }
                                },
                                RoomEventData::GameOver(x) => {
                                    let win = get_users(&x.win, &TotalUsers)?;
                                    let lose = get_users(&x.lose, &TotalUsers)?;
                                    settlement_ng_score(&win, &lose, &msgtx, &sender, &mut conn);
                                    // remove game
                                    let g = GameingGroups.remove(&x.game);
                                    let mut users: Vec<String> = Vec::new();
                                    match g {
                                        Some(g) => {
                                            for u in &g.borrow().user_names {
                                                let u = get_user(&u, &TotalUsers);
                                                if let Some(u) = u {
                                                    users.push(u.borrow().id.clone())
                                                }
                                            }
                                        for u in &g.borrow().user_names {
                                            let u = get_user(&u, &TotalUsers);
                                            match u {
                                                Some(u) => {
                                                    // add recent users
                                                    if u.borrow().recent_users.len() >= RECENT_GAME_NUM {
                                                        u.borrow_mut().recent_users.remove(0);                                                        
                                                    }
                                                    let mut tmp_users = users.clone();
                                                    let index = tmp_users.iter().position(|x| *x == u.borrow().id).unwrap();
                                                    tmp_users.remove(index);
                                                    u.borrow_mut().recent_users.push(tmp_users.clone());
                                                    
                                                    // remove room
                                                    let r = TotalRoom.get(&u.borrow().rid);
                                                    let mut is_null = false;
                                                    if let Some(r) = r {
                                                        r.borrow_mut().rm_user(&u.borrow().id);
                                                        if r.borrow().users.len() == 0 {
                                                            is_null = true;
                                                            //info!("remove success {}", u.borrow().id);
                                                        }
                                                    }
                                                    else {
                                                        //info!("remove fail {}", u.borrow().id);
                                                    }
                                                    if is_null {
                                                        TotalRoom.remove(&u.borrow().rid);
                                                        //QueueRoom.remove(&u.borrow().rid);
                                                    }
                                                    // remove group
                                                    //ReadyGroups.remove(&u.borrow().gid);
                                                    // remove game
                                                    PreStartGroups.remove(&u.borrow().game_id);
                                                    GameingGroups.remove(&u.borrow().game_id);

                                                    u.borrow_mut().rid = 0;
                                                    u.borrow_mut().gid = 0;
                                                    u.borrow_mut().game_id = 0;
                                                },
                                                None => {
                                                    error!("remove fail ");
                                                }
                                            }
                                        }
                                        //g.borrow_mut().leave_room();
                                        for u in &g.borrow().user_names {
                                            let u = get_user(&u, &TotalUsers);
                                            match u {
                                                Some(u) => {
                                                    //mqttmsg = MqttMsg{topic:format!("member/{}/res/status", u.borrow().id), 
                                                    //    msg: format!(r#"{{"msg":"game_id = {}"}}"#, u.borrow().game_id)};
                                                    if !isBackup || (isBackup && isServerLive == false) {
                                                        msgtx.try_send(MqttMsg{topic:format!("member/{}/res/status", u.borrow().id), 
                                                            msg: format!(r#"{{"msg":"game_id = {}"}}"#, u.borrow().game_id)})?;
                                                        LossSend.push(MqttMsg{topic:format!("member/{}/res/status", u.borrow().id), 
                                                            msg: format!(r#"{{"msg":"game_id = {}"}}"#, u.borrow().game_id)});
                                                    }
                                                },
                                                None => {

                                                }
                                            }
                                        }
                                        //info!("Remove game_id!");
                                        }
                                        ,
                                        None => {
                                            //info!("remove game fail {}", x.game);
                                        }
                                    }
                                },
                                RoomEventData::GameInfo(x) => {
                                    //println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                                    for u in &x.users {
                                        let mut update_info: SqlGameInfoData = Default::default();
                                        update_info.game = x.game.clone();
                                        update_info.id = u.id.clone();
                                        update_info.hero = u.hero.clone();
                                        update_info.level = u.level.clone();
                                        for (i, e) in u.equ.iter().enumerate() {
                                            update_info.equ += e;
                                            if i < u.equ.len()-1 {
                                                update_info.equ += ", ";
                                            }
                                        }
                                        update_info.damage = u.damage.clone();
                                        update_info.take_damage = u.take_damage.clone();
                                        update_info.heal = u.heal.clone();
                                        update_info.kill = u.kill.clone();
                                        update_info.death = u.death.clone();
                                        update_info.assist = u.assist.clone();
                                        update_info.gift = u.gift.clone();
                                        
                                        sender.send(SqlData::UpdateGameInfo(update_info));
                                    }
                                },
                                RoomEventData::StartGame(x) => {
                                    let g = GameingGroups.get(&x.game);
                                    if let Some(g) = g {
                                        SendGameList(&g, &msgtx, &mut conn);
                                        for r in &g.borrow().room_names {
                                            if !isBackup || (isBackup && isServerLive == false) {
                                                msgtx.try_send(MqttMsg{topic:format!("room/{}/res/start", r), 
                                                    msg: format!(r#"{{"room":"{}","msg":"start","server":"114.32.129.195:{}","game":{}}}"#, 
                                                        r, g.borrow().game_port, g.borrow().game_id)})?;
                                            }
                                        }
                                    }
                                    
                                },
                                RoomEventData::Leave(x) => {
                                    let u = TotalUsers.get(&x.id);
                                    if let Some(u) = u {
                                            let r = TotalRoom.get(&u.borrow().rid);
                                            //println!("id: {}, rid: {}", u.borrow().id, &get_rid_by_id(&u.borrow().id, &TotalUsers));
                                            let mut is_null = false;
                                            if let Some(r) = r {
                                                let m = r.borrow().master.clone();
                                                r.borrow_mut().rm_user(&x.id);
                                                if r.borrow().users.len() > 0 {
                                                    r.borrow().publish_update(&msgtx, m)?;
                                                }
                                                else {
                                                    is_null = true;
                                                }
                                                mqttmsg = MqttMsg{topic:format!("room/{}/res/leave", x.id), 
                                                    msg: format!(r#"{{"msg":"ok", "id": "{}"}}"#, x.id)};
                                                //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/leave", x.id), 
                                                //    msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                            }
                                            if is_null {
                                                TotalRoom.remove(&u.borrow().rid);
                                                //println!("Totalroom rid: {}", &u.borrow().rid);
                                                QueueSender.send(QueueData::RemoveRoom(RemoveRoomData{rid: u.borrow().rid}));
                                                //QueueRoom.remove(&u.borrow().rid);
                                            }
                                            u.borrow_mut().rid = 0;
                                            //println!("id: {}, rid: {}", u.borrow().id, &get_rid_by_id(&u.borrow().id, &TotalUsers));
                                    }
                                },
                                RoomEventData::ChooseNGHero(x) => {
                                    let u = TotalUsers.get(&x.id);
                                    if let Some(u) = u {
                                        u.borrow_mut().hero = x.hero;
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/choose_hero", u.borrow().id), 
                                            msg: format!(r#"{{"id":"{}", "hero":"{}"}}"#, u.borrow().id, u.borrow().hero)}
                                        //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/choose_hero", u.borrow().id), 
                                        //    msg: format!(r#"{{"id":"{}", "hero":"{}"}}"#, u.borrow().id, u.borrow().hero)})?;
                                    }
                                },
                                RoomEventData::Invite(x) => {
                                    if TotalUsers.contains_key(&x.from) {
                                        mqttmsg = MqttMsg{topic:format!("room/{}/res/invite", x.invite.clone()), 
                                            msg: format!(r#"{{"room":"{}","from":"{}"}}"#, x.room.clone(), x.from.clone())};
                                        //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/invite", x.invite.clone()), 
                                        //    msg: format!(r#"{{"room":"{}","from":"{}"}}"#, x.room.clone(), x.from.clone())})?;
                                    }
                                },
                                RoomEventData::Join(x) => {
                                    let u = TotalUsers.get(&x.room);
                                    let j = TotalUsers.get(&x.join);
                                    let mut sendok = false;
                                    if let Some(u) = u {
                                        if let Some(j) = j {
                                            let r = TotalRoom.get(&u.borrow().rid);
                                            
                                            if let Some(r) = r {
                                                if r.borrow().ready == 0 && r.borrow().users.len() < TEAM_SIZE as usize {
                                                    r.borrow_mut().add_user(Rc::clone(j));
                                                    let m = r.borrow().master.clone();
                                                    r.borrow().publish_update(&msgtx, m)?;
                                                    r.borrow().publish_update(&msgtx, x.join.clone())?;
                                                    mqttmsg = MqttMsg{topic:format!("room/{}/res/join", x.join.clone()), 
                                                        msg: format!(r#"{{"room":"{}","msg":"ok"}}"#, r.borrow().master)};
                                                    //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/join", x.join.clone()), 
                                                    //    msg: format!(r#"{{"room":"{}","msg":"ok"}}"#, r.borrow().master)})?;
                                                    //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/join", x.join.clone()), 
                                                    //    msg: format!(r#"{{"room":"{}","msg":"ok"}}"#, x.room.clone())})?;
                                                    sendok = true;
                                                }
                                            }
                                        }
                                    }
                                    if sendok == false {
                                        mqttmsg = MqttMsg{topic:format!("room/{}/res/join", x.join.clone()), 
                                            msg: format!(r#"{{"room":"{}","msg":"fail"}}"#, x.room.clone())};
                                        //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/join", x.join.clone()), 
                                        //    msg: format!(r#"{{"room":"{}","msg":"fail"}}"#, x.room.clone())})?;
                                    }
                                    //println!("TotalRoom {:#?}", TotalRoom);
                                },
                                RoomEventData::Reset() => {
                                    TotalRoom.clear();
                                    //QueueRoom.clear();
                                    ReadyGroups.clear();
                                    PreStartGroups.clear();
                                    GameingGroups.clear();
                                    TotalUsers.clear();
                                    room_id = 0;
                                },
                                RoomEventData::PreStart(x) => {
                                    let u = TotalUsers.get(&x.room);
                                    if let Some(u) = u {
                                        let gid = u.borrow().gid;
                                        //println!("gid: {}, id: {}", gid, u.borrow().id);
                                        if u.borrow().prestart_get == true {
                                            
                                            if gid != 0 {
                                                let g = ReadyGroups.get(&gid);
                                                if let Some(gr) = g {
                                                    if x.accept == true {
                                                        gr.borrow_mut().user_ready(&x.id);
                                                        //info!("PreStart user_ready");
                                                        mqttmsg = MqttMsg{topic:format!("room/{}/res/start_get", u.borrow().id), 
                                                            msg: format!(r#"{{"msg":"start"}}"#)};
                                                        //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/start_get", u.borrow().id), 
                                                        //        msg: format!(r#"{{"msg":"start"}}"#)})?;
                                                    } else {
                                                        println!("accept false!");
                                                        gr.borrow_mut().user_cancel(&x.id);
                                                        let mut users: Vec<String> = vec![];
                                                        let mut block: Vec<String> = vec![];
                                                        for room in &gr.borrow().rooms {
                                                            for u in &room.borrow().users {
                                                                
                                                                users.push(u.borrow().id.clone());
                                                                for index in &u.borrow().recent_users {
                                                                    block = [block.as_slice(), index.clone().as_slice()].concat();
                                                                }
                                                                block = [block.as_slice(), u.borrow().blacklist.as_slice()].concat();
                                            
                                                            }
                                                        }
                                                        for r in &gr.borrow().rooms {
                                                            //println!("r_rid: {}, u_rid: {}", r.borrow().rid, u.borrow().rid);
                                                            if r.borrow().rid != u.borrow().rid {
                                                                let mut data = QueueRoomData {
                                                                    user_name: users.clone(),
                                                                    rid: r.borrow().rid.clone(),
                                                                    gid: 0,
                                                                    user_len: r.borrow().users.len().clone() as i16,
                                                                    avg_ng: r.borrow().avg_ng.clone(),
                                                                    avg_rk: r.borrow().avg_rk.clone(),
                                                                    ready: 0,
                                                                    queue_cnt: 1,
                                                                    block: block.clone(),
                                                                };
                                                                QueueSender.send(QueueData::UpdateRoom(data));
                                                            }
                                                        }
                                                        ReadyGroups.remove(&gid);
                                                        let r = TotalRoom.get(&u.borrow().rid);
                                                        //QueueSender.send(QueueData::RemoveRoom(RemoveRoomData{rid: u.borrow().rid}));
                                                        if let Some(r) = r {
                                                            mqttmsg = MqttMsg{topic:format!("room/{}/res/cancel_queue", r.borrow().master), 
                                                                msg: format!(r#"{{"msg":"ok"}}"#)};
                                                            //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/cancel_queue", r.borrow().master), 
                                                            //    msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                                        }
                                                    }
                                                }
                                                else
                                                {
                                                    error!("gid not found {}", gid);
                                                }
                                            }
                                        }
                                    }
                                },
                                RoomEventData::UpdateGame(x) => {
                                    //println!("Update Game!");
                                    let mut fg: FightGame = Default::default();
                                    for r in &x.rid {
                                        let mut g: FightGroup = Default::default();
                                        for rid in r {
                                            let room = TotalRoom.get(&rid);
                                            if let Some(room) = room {
                                                g.add_room(Rc::clone(&room));
                                            }
                                        }
                                        g.prestart();
                                        group_id += 1;
                                        g.set_group_id(group_id);
                                        g.game_status = 1;
                                        ReadyGroups.insert(group_id, Rc::new(RefCell::new(g.clone())));
                                        let mut rg = ReadyGroups.get(&group_id);
                                        if let Some(rg) = rg {
                                            fg.teams.push(Rc::clone(rg));
                                        }
                                    }

                                    fg.update_names();
                                    for r in &fg.room_names {
                                        //thread::sleep_ms(100);
                                        if !isBackup || (isBackup && isServerLive == false) {
                                            msgtx.try_send(MqttMsg{topic:format!("room/{}/res/prestart", r), msg: r#"{"msg":"prestart"}"#.to_string()})?;
                                        }
                                    }
                                    
                                    
                                    game_id += 1;
                                    fg.set_game_id(game_id);
                                    PreStartGroups.insert(game_id, Rc::new(RefCell::new(fg)));

                                },
                                RoomEventData::PreStartGet(x) => {
                                    let mut u = TotalUsers.get(&x.id);
                                    if let Some(u) = u {
                                        u.borrow_mut().prestart_get = true;
                                        
                                    }
                                },
                                RoomEventData::StartQueue(x) => {
                                    let mut success = false;
                                    let mut hasRoom = false;
                                    let u = TotalUsers.get(&x.id);
                                    let mut rid = 0;
                                    if let Some(u) = u {
                                        if u.borrow().rid != 0 {
                                            hasRoom = true;
                                            rid = u.borrow().rid;
                                        }
                                    }
                                    if hasRoom {
                                        let r = TotalRoom.get(&rid);
                                        if let Some(y) = r {
                                            let mut users: Vec<String> = vec![];
                                            let mut block: Vec<String> = vec![];
                                            for u in &y.borrow().users {                  
                                                users.push(u.borrow().id.clone());
                                                
                                                for index in &u.borrow().recent_users {
                                                    block = [block.as_slice(), index.clone().as_slice()].concat();
                                                }
                                                block = [block.as_slice(), u.borrow().blacklist.as_slice()].concat();
                                                
                                            }
                                            println!("users: {:?}", users);
                                            println!("block: {:?}", block);
                                            y.borrow_mut().update_avg();
                                            let mut data = QueueRoomData {
                                                user_name: users.clone(),
                                                rid: y.borrow().rid.clone(),
                                                gid: 0,
                                                user_len: y.borrow().users.len().clone() as i16,
                                                avg_ng: y.borrow().avg_ng.clone(),
                                                avg_rk: y.borrow().avg_rk.clone(),
                                                ready: 0,
                                                queue_cnt: 1,
                                                block: block.clone(),
                                            };
                                            //println!("Totalroom rid: {}", rid);
                                            QueueSender.send(QueueData::UpdateRoom(data));
                                            //QueueRoom.insert(
                                            //    y.borrow().rid,
                                            //    Rc::clone(y)
                                            //);
                                            success = true;
                                            if success {
                                                mqttmsg = MqttMsg{topic:format!("room/{}/res/start_queue", y.borrow().master.clone()), 
                                                    msg: format!(r#"{{"msg":"ok"}}"#)};
                                                //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/start_queue", y.borrow().master.clone()), 
                                                //    msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                            } else {
                                                mqttmsg = MqttMsg{topic:format!("room/{}/res/start_queue", y.borrow().master.clone()), 
                                                    msg: format!(r#"{{"msg":"fail"}}"#)}
                                                //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/start_queue", y.borrow().master.clone()), 
                                                //    msg: format!(r#"{{"msg":"fail"}}"#)})?;
                                            }
                                        }
                                    }
                                },
                                RoomEventData::CancelQueue(x) => {
                                    let mut success = false;
                                    let u = TotalUsers.get(&x.id);
                                    if let Some(u) = u {
                                        
                                        //let r = QueueRoom.remove(&u.borrow().rid);
                                        let g = ReadyGroups.get(&u.borrow().gid);
                                        if let Some(g) = g {
                                            for r in &g.borrow().rooms {
                                                r.borrow_mut().ready = 0;
                                            }
                                        }
                                        
                                        let gr = ReadyGroups.remove(&u.borrow().gid);
                                        let r = TotalRoom.get(&u.borrow().rid);
                                        //println!("Totalroom rid: {}", &u.borrow().rid);
                                        QueueSender.send(QueueData::RemoveRoom(RemoveRoomData{rid: u.borrow().rid}));
                                        if let Some(r) = r {
                                            success = true;
                                            if success {
                                                mqttmsg = MqttMsg{topic:format!("room/{}/res/cancel_queue", r.borrow().master.clone()), 
                                                    msg: format!(r#"{{"msg":"ok"}}"#)};
                                                //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/cancel_queue", r.borrow().master.clone()), 
                                                //    msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                            } else {
                                                mqttmsg = MqttMsg{topic:format!("room/{}/res/cancel_queue", r.borrow().master.clone()), 
                                                    msg: format!(r#"{{"msg":"fail"}}"#)};
                                                //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/cancel_queue", r.borrow().master.clone()), 
                                                //    msg: format!(r#"{{"msg":"fail"}}"#)})?;
                                            }
                                        }
                                    }
                                },
                                RoomEventData::Login(x) => {
                                    let mut success = true;                         
                                    if TotalUsers.contains_key(&x.u.id) {
                                        let u2 = TotalUsers.get(&x.u.id);
                                        if let Some(u2) = u2 {
                                            u2.borrow_mut().online = true;
                                            mqttmsg = MqttMsg{topic:format!("member/{}/res/login", u2.borrow().id.clone()), 
                                                msg: format!(r#"{{"msg":"ok", "ng":{}, "rk":{} }}"#, u2.borrow().ng, u2.borrow().rk)};
                                            //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", u2.borrow().id.clone()), 
                                            //    msg: format!(r#"{{"msg":"ok", "ng":{}, "rk":{} }}"#, u2.borrow().ng, u2.borrow().rk)})?;
                                        }
                                        
                                    }
                                    else {
                                        TotalUsers.insert(x.u.id.clone(), Rc::new(RefCell::new(x.u.clone())));
                                        //thread::sleep(Duration::from_millis(50));
                                        sender.send(SqlData::Login(SqlLoginData {id: x.dataid.clone(), name: name.clone()}));
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/login", x.u.id.clone()), 
                                            msg: format!(r#"{{"msg":"ok", "ng":{}, "rk":{} }}"#, x.u.ng, x.u.rk)};
                                        //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", x.u.id.clone()), 
                                        //    msg: format!(r#"{{"msg":"ok", "ng":{}, "rk":{} }}"#, x.u.ng, x.u.rk)})?;
                                    }
                                    
                                    /*
                                    if success {
                                        //TotalUsers.insert(x.u.id.clone(), Rc::new(RefCell::new(x.u.clone())));
                                        msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", x.u.id.clone()), 
                                            msg: format!(r#"{{"msg":"ok", "ng":{}, "rk":{} }}"#, x.u.ng, x.u.rk)})?;
                                    } else {
                                        //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", x.u.id.clone()), 
                                        //    msg: format!(r#"{{"msg":"fail"}}"#)})?;
                                        msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", x.u.id.clone()), 
                                            msg: format!(r#"{{"msg":"ok", "ng":{}, "rk":{} }}"#, x.u.ng, x.u.rk)})?;
                                    }
                                    */
                                },
                                RoomEventData::Logout(x) => {
                                    let mut success = false;
                                    let u = TotalUsers.get(&x.id); 
                                    let u2 = get_user(&x.id, &TotalUsers);
                                    if let Some(u2) = u2 {
                                        u2.borrow_mut().online = false;
                                    }                             
                                    if let Some(u) = u {
                                        let mut is_null = false;
                                        if u.borrow().game_id == 0 {
                                            let gid = u.borrow().gid;
                                            let rid = u.borrow().rid;
                                            let r = TotalRoom.get(&u.borrow().rid);
                                            
                                            if let Some(r) = r {
                                                let m = r.borrow().master.clone();
                                                r.borrow_mut().rm_user(&x.id);
                                                if r.borrow().users.len() > 0 {
                                                    r.borrow().publish_update(&msgtx, m)?;
                                                }
                                                else {
                                                    is_null = true;
                                                }
                                                //mqttmsg = MqttMsg{topic:format!("room/{}/res/leave", x.id), 
                                                //    msg: format!(r#"{{"msg":"ok"}}"#)};
                                                if !isBackup || (isBackup && isServerLive == false) {
                                                    msgtx.try_send(MqttMsg{topic:format!("room/{}/res/leave", x.id), 
                                                        msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                                    LossSend.push(MqttMsg{topic:format!("room/{}/res/leave", x.id), 
                                                        msg: format!(r#"{{"msg":"ok"}}"#)});
                                                }
                                            }
                                            
                                            if gid != 0 {
                                                let g = ReadyGroups.get(&gid);
                                                if let Some(gr) = g {
                                                    gr.borrow_mut().user_cancel(&x.id);
                                                    ReadyGroups.remove(&gid);
                                                    let r = TotalRoom.get(&rid);
                                                    //println!("Totalroom rid: {}", &u.borrow().rid);
                                                    QueueSender.send(QueueData::RemoveRoom(RemoveRoomData{rid: rid}));
                                                    if let Some(r) = r {
                                                        //mqttmsg = MqttMsg{topic:format!("room/{}/res/cancel_queue", r.borrow().master), 
                                                        //    msg: format!(r#"{{"msg":"ok"}}"#)};
                                                        if !isBackup || (isBackup && isServerLive == false) {
                                                            msgtx.try_send(MqttMsg{topic:format!("room/{}/res/cancel_queue", r.borrow().master), 
                                                                msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                                            LossSend.push(MqttMsg{topic:format!("room/{}/res/cancel_queue", r.borrow().master), 
                                                                msg: format!(r#"{{"msg":"ok"}}"#)});
                                                        }
                                                    }
                                                    //TotalRoom.remove(&rid);
                                                }
                                            }
                                            //println!("{:?}", TotalRoom);
                                            //TotalRoom.remove(&u.borrow().rid);
                                            success = true;
                                        } else {
                                            success = true;
                                        }
                                        if is_null {
                                            TotalRoom.remove(&u.borrow().rid);
                                            //QueueRoom.remove(&u.borrow().rid);
                                        }
                                    }
                                    
                                    /*
                                    if success {
                                        success = false;
                                        let mut u = TotalUsers.get(&x.id);
                                        if let Some(u) = u {
                                            if u.borrow().game_id == 0 {
                                                success = true;
                                            }
                                        }
                                        if success {
                                            TotalUsers.remove(&x.id);
                                        }
                                    }
                                    */
                                    if success {
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/logout", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"ok"}}"#)};
                                        //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/logout", x.id.clone()), 
                                        //    msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                    } else {
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/logout", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"fail"}}"#)};
                                        //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/logout", x.id.clone()), 
                                        //    msg: format!(r#"{{"msg":"fail"}}"#)})?;
                                    }
                                },
                                RoomEventData::Create(x) => {
                                    let mut success = false;
                                    //println!("rid: {}", &get_rid_by_id(&x.id, &TotalUsers));
                                    if !TotalRoom.contains_key(&get_rid_by_id(&x.id, &TotalUsers)) {
                                        room_id += 1;
                                        
                                        let mut new_room = RoomData {
                                            rid: room_id,
                                            users: vec![],
                                            master: x.id.clone(),
                                            last_master: "".to_owned(),
                                            avg_ng: 0,
                                            avg_rk: 0,
                                            ready: 0,
                                            queue_cnt: 1,
                                        };
                                        let mut u = TotalUsers.get(&x.id);
                                        if let Some(u) = u {
                                            new_room.add_user(Rc::clone(&u));
                                            let rid = new_room.rid;
                                            let r = Rc::new(RefCell::new(new_room));
                                            r.borrow().publish_update(&msgtx, x.id.clone())?;
                                            TotalRoom.insert(
                                                rid,
                                                Rc::clone(&r),
                                            );
                                            success = true;
                                        }
                                    }
                                    if success {
                                        mqttmsg = MqttMsg{topic:format!("room/{}/res/create", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"ok"}}"#)};
                                        //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/create", x.id.clone()), 
                                        //    msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                    } else {
                                        mqttmsg = MqttMsg{topic:format!("room/{}/res/create", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"fail"}}"#)};
                                        //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/create", x.id.clone()), 
                                        //    msg: format!(r#"{{"msg":"fail"}}"#)})?;
                                    }
                                },
                                RoomEventData::Close(x) => {
                                    let mut success = false;
                                    if let Some(y) =  TotalRoom.remove(&get_rid_by_id(&x.id, &TotalUsers)) {
                                        let data = TotalRoom.remove(&get_rid_by_id(&x.id, &TotalUsers));
                                        y.borrow_mut().leave_room();
                                        match data {
                                            Some(_) => {
                                                //QueueRoom.remove(&get_rid_by_id(&x.id, &TotalUsers));
                                                println!("Totalroom rid: {}", &get_rid_by_id(&x.id, &TotalUsers));
                                                QueueSender.send(QueueData::RemoveRoom(RemoveRoomData{rid: get_rid_by_id(&x.id, &TotalUsers)}));
                                                success = true;
                                            },
                                            _ => {}
                                        }
                                    }
                                    if success {
                                        mqttmsg = MqttMsg{topic:format!("room/{}/res/cancel_queue", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"ok"}}"#)};
                                        //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/cancel_queue", x.id.clone()), 
                                        //    msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                    } else {
                                        mqttmsg = MqttMsg{topic:format!("room/{}/res/cancel_queue", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"fail"}}"#)};
                                        //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/cancel_queue", x.id.clone()), 
                                        //    msg: format!(r#"{{"msg":"fail"}}"#)})?;
                                    }
                                },
                                RoomEventData::MainServerDead(x) => {
                                    isServerLive = false;
                                    isBackup = false;
                                    for msg in LossSend.clone() {
                                        msgtx.try_send(msg.clone())?;
                                    }
                                }
                            }
                        }
                        //println!("isBackup: {}, isServerLive: {}", isBackup, isServerLive);
                        if mqttmsg.topic != "" {
                            LossSend.push(mqttmsg.clone());
                            if !isBackup || (isBackup && isServerLive == false) {
                                //println!("send");
                                msgtx.try_send(mqttmsg.clone())?;
                            }
                        }
                        if msgtx.is_full() {
                            println!("FULL!!");
                            //thread::sleep(Duration::from_millis(5000));
                            
                        }
                        Ok(())
                    };
                    if let Err(msg) = handle() {
                        
                        println!("Error msgtx len: {}", msgtx.len());
                        println!("Error rx len: {}", rx.len());
                        println!("init {:?}", msg);
                        continue;
                        
                        panic!("Error found");
                        
                        
                    }
                    
                }
            }
        }
    });

    Ok((tx, tx1))
}

pub fn create(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: CreateRoomData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::Create(CreateRoomData{id: data.id.clone()}));
    Ok(())
}

pub fn close(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: CloseRoomData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::Close(CloseRoomData{id: data.id.clone()}));
    Ok(())
}

pub fn start_queue(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: StartQueueData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::StartQueue(StartQueueData{id: data.id.clone(), action: data.action.clone()}));
    Ok(())
}

pub fn cancel_queue(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: CancelQueueData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::CancelQueue(data));
    Ok(())
}

pub fn prestart(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: PreStartData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::PreStart(data));
    Ok(())
}

pub fn prestart_get(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: PreStartGetData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::PreStartGet(data));
    Ok(())
}

pub fn join(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: JoinRoomData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::Join(data));
    Ok(())
}

pub fn choose_ng_hero(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: UserNGHeroData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::ChooseNGHero(data));
    Ok(())
}

pub fn invite(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: InviteRoomData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::Invite(data));
    Ok(())
}

pub fn leave(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: LeaveData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::Leave(data));
    Ok(())
}


pub fn start_game(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: StartGameData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::StartGame(data));
    Ok(())
}

pub fn game_over(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: GameOverData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::GameOver(data));
    Ok(())
}

pub fn game_info(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: GameInfoData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::GameInfo(data));
    Ok(())
}


pub fn game_close(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: GameCloseData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::GameClose(data));
    Ok(())
}

pub fn status(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: StatusData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::Status(data));
    Ok(())
}

pub fn reconnect(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: ReconnectData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::Reconnect(data));
    Ok(())
}

pub fn server_dead(id: String, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    sender.try_send(RoomEventData::MainServerDead(DeadData{ServerDead: id}));
    Ok(())
}