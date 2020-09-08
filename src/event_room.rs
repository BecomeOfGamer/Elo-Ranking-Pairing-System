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
use std::fs::File;
use std::io::prelude::*;
use uuid::Uuid;
use failure::Error;
use rust_decimal::Decimal;
use rust_decimal_macros::*;

extern crate toml;
use crate::room::*;
use crate::msg::*;
use crate::elo::*;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct GameSetting {
    SCORE_INTERVAL: Option<i16>,
    HONOR_THRESHOLD: Option<i32>,
    BLOCK_RECENT_PLAYER_OF_GAMES: Option<usize>,
    HERO: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct GameMode {
    MODE: Option<String>,
    TEAM_SIZE: Option<i16>,
    MATCH_SIZE: Option<usize>,
    SCORE_INTERVAL: Option<i16>,
    BLOCK_RECENT_PLAYER_OF_GAMES: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Config {
    game_setting: Option<GameSetting>,
    game_mode: Option<Vec<GameMode>>,
}

const TEAM_SIZE: i16 = 5;
const MATCH_SIZE: usize = 2;
const SCORE_INTERVAL: i16 = 100;
const BLOCK_RECENT_PLAYER_OF_GAMES: usize = 2;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameModeCfg {
    pub mode: String,
    pub team_size: i16,
    pub match_size: usize,
}

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
pub struct GameServerLoginData {
    pub name: String,
    pub address: String,
    pub max_server: u32,
    pub max_user: u32,
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
    pub mode: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CancelQueueData {
    pub id: String,
    pub action: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreGameData {
    pub rid: Vec<Vec<u32>>,
    pub mode: String,
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
    pub Talent: UserGift,
    pub equ: Vec<Equit>,
    pub effect: Effect
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Equit {
    pub name: String,
    pub total_weight: f32,
    pub positive: String,
    pub pvalue: f32,
    pub negative: String,
    pub nvalue: f32,
    pub option: Vec<EquOpt>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EquOpt {
    pub name: String,
    pub weight: f32,
    pub value: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Effect {
    pub lifesteal: Vec<f32>,
    pub maxhp: Vec<f32>,
    pub maxmp: Vec<f32>,
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
pub struct GameInfoRes {
    pub game: u32,
    pub users: Vec<UserInfoRes>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserInfoRes {
    pub steamid: String,
    pub hero: String,
    pub TotalCurrency: u32,
    pub Currency: u32,
    pub WinCount: u32,
    pub LoseCount: u32,
    pub HeroMasteryLevel: u32,
    pub LastHeroMastery: u32,
    pub HeroMastery: u32,
    pub PlayerLevel: u32,
    pub PlayerExperience: u32,
    pub LastPlayerExperience: u32,
    pub Rank: String,
    pub RankLevel: u16,
    pub LastRankScore: u32,
    pub RankScore: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GetRPData {
    pub id: String,
    pub game: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UploadData {
    pub game: u32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UploadRes {
    pub game: u32,
    pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserInfoData {
    pub steamid: String,
    pub hero: String,
    // pub level: u16,
    // pub equ: Vec<String>,
    pub damage: u16,
    pub be_damage: u16,
    pub K: u16,
    pub D: u16,
    pub A: u16,
    pub Talent: UserGift,
    pub BattleScore: String,
    pub Currency: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserGift {
    pub A: u16,
    pub B: u16,
    pub C: u16,
    pub D: u16,
    pub E: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RankBanData {
    pub gameid: u32,
    pub ban: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RankChooseData {
    pub gameid: u32,
    pub member: String,
    pub choose: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TestData {
    pub Test: Vec<TestInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TestInfo {
    pub equ_id: u32,
    pub rank: u8,
    pub Lv: u8,
    pub Lv5: u8,
    pub option1: u32,
    pub option2: u32,
    pub option3: u32,
    pub option1Lv: u8,
    pub option2Lv: u8,
    pub option3Lv: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EquInfo {
    pub userid: String,
    pub equ_id: u32,
    pub rank: u8,
    pub Lv: u8,
    pub Lv5: u8,
    pub option1: u32,
    pub option2: u32,
    pub option3: u32,
    pub option1Lv: u8,
    pub option2Lv: u8,
    pub option3Lv: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserDeleteEquip {
    pub userid: String,
    pub equ_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct NewEquip {
    pub equ_id: u32,
    pub equ_name: String,
    pub positive: String,
    pub negative: String,
    pub PLv1: f32,
    pub PLv2: f32,
    pub PLv3: f32,
    pub PLv4: f32,
    pub PLv5v1: f32,
    pub PLv5v2: f32,
    pub PLv5v3: f32,
    pub PLv5v4: f32,
    pub PLv5v5: f32,
    pub NLv1: f32,
    pub NLv2: f32,
    pub NLv3: f32,
    pub NLv4: f32,
    pub NLv5v1: f32,
    pub NLv5v2: f32,
    pub NLv5v3: f32,
    pub NLv5v4: f32,
    pub NLv5v5: f32,
    pub Option1: u32,
    pub Option2: u32,
    pub Option3: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DeleteEquip {
    pub equ_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Black {
    pub id: String,
    pub black: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QueryBlack {
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct NewOption {
    pub option_id: u32,
    pub option_name: String,
    pub special: bool,
    pub exception: bool,
    pub set_id: u32,
    pub set_amount: u8,
    pub option_weight: f32,
    pub effect_v1: f32,
    pub effect_v2: f32,
    pub effect_v3: f32,
    pub effect_v4: f32,
    pub effect_v5: f32, 
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DeleteOption {
    pub option_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Msg {
    pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RankGameStatusData {
    pub gameid: u32,
    pub msg: String,
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
    RankBan(RankBanData),
    RankChoose(RankChooseData),
    RankGameStatus(RankGameStatusData),
    GetRP(GetRPData),
    UploadRP(UploadData),
    UploadRes(UploadRes),
    InsertEqu(EquInfo),
    ModifyUserEqu(EquInfo),
    DeleteUserEqu(UserDeleteEquip),
    ModifyEqu(NewEquip),
    CreateEqu(NewEquip),
    DeleteEqu(DeleteEquip),
    NewOption(NewOption),
    ModifyOption(NewOption),
    DeleteOption(DeleteOption),
    AddBlackList(Black),
    QueryBlackList(QueryBlack),
    RemoveBlackList(Black),
    TestEquip(TestData),
    Status(StatusData),
    Reconnect(ReconnectData),
    GameServerLogin(GameServerLoginData),
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
    pub mode: String,
    pub Res: bool,
}

#[derive(Clone, Debug)]
pub struct SqlReplayData {
    pub gameid: u32,
    pub name: String,
    pub url: String,
    pub address: String,
}

#[derive(Clone, Debug)]
pub struct SqlGameData {
    pub gameid: u32,
    pub userid: String,
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SqlGameInfoData {
    pub game : u32,
    pub id: String,
    pub hero: String,
    pub equ: String,
    pub damage: u16, 
    pub be_damage: u16,
    pub Res: bool,
    pub afk: bool, 
    pub K: u16,
    pub D: u16,
    pub A: u16,
    pub BattleScore: u8,
    pub Currency: u32,
    pub Talent: UserGift,
}

#[derive(Clone, Debug)]
pub struct SqlHeroname {
    pub hero_type: Vec<String>,
}

pub enum SqlData {
    Login(SqlLoginData),
    UpdateScore(SqlScoreData),
    UpdateGameInfo(SqlGameInfoData),
    UpdateReplay(SqlReplayData),
    UpdateExistingRP(SqlReplayData),
    AddBlack(Black),
    RemoveBlack(Black),
    HeroNum(SqlHeroname),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QueueRoomData {
    pub user_name: Vec<String>,
    pub hero: Vec<String>,
    pub rid: u32,
    pub gid: u32,
    pub user_len: i16,
    pub avg_ng1v1: i16,
    pub avg_rk1v1: i16,
    pub avg_ng5v5: i16,
    pub avg_rk5v5: i16,
    pub honor: bool,
    pub mode: String,
    pub ready: i8,
    pub allow_same_hero: bool,
    pub queue_cnt: i16,
    pub block: Vec<String>,
    pub blacklist: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReadyGroupData {
    pub user_name: Vec<String>,
    pub hero: Vec<String>,
    pub gid: u32,
    pub rid: Vec<u32>,
    pub max_room_len: i16,
    pub user_len: i16,
    pub avg_ng1v1: i16,
    pub avg_rk1v1: i16,
    pub avg_ng5v5: i16,
    pub avg_rk5v5: i16,
    pub honor: bool,
    pub allow_same_hero: bool,
    pub game_status: u16,
    pub queue_cnt: i16,
    pub block: Vec<String>,
    pub blacklist: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReadyGameData {
    pub user_name: Vec<String>,
    pub hero: Vec<String>,
    pub gid: Vec<u32>,
    pub honor: bool,
    pub allow_same_hero: bool,
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

fn SendGameList(game: &Rc<RefCell<FightGame>>, msgtx: &Sender<MqttMsg>, conn: &mut mysql::PooledConn , TotalEquip: &BTreeMap<u32, Rc<RefCell<Equipment>>>, TotalEquOption: &BTreeMap<u32, Rc<RefCell<EquOption>>>)
    -> Result<(), Error> {
    let mut res: StartGameSendData = Default::default();
    res.game = game.borrow().game_id;
    for (i, t) in game.borrow().teams.iter().enumerate() {
        let ids = t.borrow().get_users_id_hero();
        for (id, name, hero, equip) in &ids {
            let mut h: HeroCell = HeroCell {id:id.clone(), team: (i+1) as u16, name:name.clone(), hero:hero.clone(), ..Default::default() };
            h.Talent.A = 1;
            h.Talent.B = 3;
            h.Talent.C = 3;
            h.Talent.D = 3;
            h.Talent.E = 3;

            println!("equip: {:?}", equip);

            for e in equip {
                let equ = TotalEquip.get(&e.equ_id.clone());
                if let Some(equ) = equ {
                    let mut e1 = Equit {
                        name: equ.borrow().equ_name.clone(),
                        positive: equ.borrow().positive.clone(),
                        pvalue: (equ.borrow().pvalue[(e.lv-1) as usize].clone() * 1000.0).round()/ 1000 as f32,
                        negative: equ.borrow().negative.clone(),
                        nvalue: (equ.borrow().nvalue[(e.lv-1) as usize].clone() * 1000.0).round()/ 1000 as f32,
                        ..Default::default()
                    };
                    let mut eff = Effect {..Default::default()};
                    let mut total_weight = 0.0;
                    if e.rank == 1 {
                        total_weight = 1.0;
                    } else if e.rank == 2 {
                        total_weight = 1.5;
                    } else if e.rank == 3 {
                        total_weight = 2.0;
                    } else if e.rank == 4 {
                        total_weight = 2.5;
                    } else if e.rank == 5 {
                        total_weight = 3.0;
                    }
                    for i in equ.borrow().option_id.clone() {
                        let opt = TotalEquOption.get(&i);
                        if let Some(opt) = opt {
                            let mut eo = EquOpt {
                                name: opt.borrow().option_name.clone(),
                                weight: (opt.borrow().option_weight.clone() * 1000.0).round() / 1000 as f32,
                                value: (opt.borrow().effect[(e.lv5-1) as usize].clone() * 1000.0).round() / 1000 as f32,
                            };
                            total_weight += eo.weight;
                            if eo.name == "lifesteal" {
                                eff.lifesteal.push((1000.0 * eo.value).round()  / 1000 as f32 );
                            } else if eo.name == "maxhp" {
                                eff.maxhp.push((1000.0 * eo.value).round() / 1000 as f32);
                            } else if eo.name == "maxmp" {
                                eff.maxmp.push((1000.0 * eo.value).round() / 1000 as f32);
                            }
                            // if let Some(eff1) = h.effect.clone().into_iter().find(|s| s.name == eo.name) {
                            //     ;
                            // } else {
                            //     let mut eff = Effect {
                            //         name: eo.name.clone(),
                            //         v: eo.value.clone(),
                            //     };
                            //     h.effect.push(eff);
                            // }
                            e1.option.push(eo);
                        }
                    }
                    e1.total_weight = (total_weight * 1000.0).round()/1000 as f32; 
                    // positive
                    if e1.positive == "lifesteal" {
                        eff.lifesteal.push((1000.0*e1.pvalue*e1.total_weight).round() / 1000 as f32);
                    } else if e1.positive == "maxhp" {
                        eff.maxhp.push((1000.0*e1.pvalue*e1.total_weight).round() / 1000 as f32);
                    } else if e1.positive == "maxmp" {
                        eff.maxmp.push((1000.0*e1.pvalue*e1.total_weight).round() / 1000 as f32);
                    }
                    //h.effect.push(eff);
                    // negative
                    if e1.negative == "lifesteal" {
                        eff.lifesteal.push((1000.0*e1.nvalue*e1.total_weight).round() / 1000 as f32);
                    } else if e1.negative == "maxhp" {
                        eff.maxhp.push((1000.0*e1.nvalue*e1.total_weight).round() / 1000 as f32);
                    } else if e1.negative == "maxmp" {
                        eff.maxmp.push((1000.0*e1.nvalue*e1.total_weight).round() / 1000 as f32);
                    }
                    h.effect = eff.clone();
                    
                    h.equ.push(e1);
                }
                
                
            }

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

fn user_score(u: &Rc<RefCell<User>>, value: i16, msgtx: &Sender<MqttMsg>, sender: &Sender<SqlData>, conn: &mut mysql::PooledConn, mode: String, Win: bool) -> Result<(), Error> {
    
    msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", u.borrow().id), 
        msg: format!(r#"{{"msg":"ok" }}"#)})?;
    //println!("Update!");
    if mode == "ng1p2t" {
        u.borrow_mut().ng1v1.score += value;
        if Win == true {
            u.borrow_mut().ng1v1.WinCount += 1;
        } else {
            u.borrow_mut().ng1v1.LoseCount += 1;
        }
        sender.send(SqlData::UpdateScore(SqlScoreData {id: u.borrow().id.clone(), score: u.borrow().ng1v1.score.clone(), mode: mode.clone(), Res: Win.clone()}));
    
    } else if mode == "ng5p2t" {
        u.borrow_mut().ng5v5.score += value;
        if Win == true {
            u.borrow_mut().ng5v5.WinCount += 1;
        } else {
            u.borrow_mut().ng5v5.LoseCount += 1;
        }
        sender.send(SqlData::UpdateScore(SqlScoreData {id: u.borrow().id.clone(), score: u.borrow().ng5v5.score.clone(), mode: mode.clone(), Res: Win.clone()}));
    
    } else if mode == "rk1p2t" {
        u.borrow_mut().rk1v1.score += value;
        if Win == true {
            u.borrow_mut().rk1v1.WinCount += 1;
        } else {
            u.borrow_mut().rk1v1.LoseCount += 1;
        }
        sender.send(SqlData::UpdateScore(SqlScoreData {id: u.borrow().id.clone(), score: u.borrow().rk1v1.score.clone(), mode: mode.clone(), Res: Win.clone()}));
    
    } else if mode == "rk5p2t" {
        u.borrow_mut().rk5v5.score += value;
        if Win == true {
            u.borrow_mut().rk5v5.WinCount += 1;
        } else {
            u.borrow_mut().rk5v5.LoseCount += 1;
        }
        sender.send(SqlData::UpdateScore(SqlScoreData {id: u.borrow().id.clone(), score: u.borrow().rk5v5.score.clone(), mode: mode.clone(), Res: Win.clone()}));
    
    }
        //let sql = format!("UPDATE user_ng as a JOIN user as b ON a.id=b.id SET score={} WHERE b.userid='{}';", u.borrow().ng, u.borrow().id);
    //println!("sql: {}", sql);
    //let qres = conn.query(sql.clone())?;
    Ok(())
}

fn get_ng(team : &Vec<Rc<RefCell<User>>>, mode: String) -> Vec<i32> {
    let mut res: Vec<i32> = vec![];
    if mode == "ng1p2t" {
        for u in team {
            res.push(u.borrow().ng1v1.score.into());
        }
    }
    else if mode == "ng5p2t" {
        for u in team {
            res.push(u.borrow().ng5v5.score.into());
        }
    }
    res
}

fn get_rk(team : &Vec<Rc<RefCell<User>>>, mode: String) -> Vec<i32> {
    let mut res: Vec<i32> = vec![];
    if mode == "rk1p2t"{
        for u in team {
            res.push(u.borrow().rk1v1.score.into());
        }
    }
    else if mode == "rk5p2t" {
        for u in team {
            res.push(u.borrow().rk5v5.score.into());
        }
    }
    res
}

fn settlement_ng_score(win: &Vec<Rc<RefCell<User>>>, lose: &Vec<Rc<RefCell<User>>>, msgtx: &Sender<MqttMsg>, sender: &Sender<SqlData>, conn: &mut mysql::PooledConn, mode: String) {
    if win.len() == 0 || lose.len() == 0 {
        return;
    }
    let mut win_score: Vec<i32> = vec![];
    let mut lose_score: Vec<i32> = vec![];
    if mode == "ng1p2t" || mode  == "ng5p2t" {
        win_score = get_ng(win, mode.clone());
        lose_score = get_ng(lose, mode.clone());
    }
    else {
        win_score = get_rk(win, mode.clone());
        lose_score = get_rk(lose, mode.clone());
    }
    let elo = EloRank {k:20.0};
    let (rw, rl) = elo.compute_elo_team(&win_score, &lose_score);
    println!("{} Game Over", mode);
    for (i, u) in win.iter().enumerate() {
        user_score(u, (rw[i]-win_score[i]) as i16, msgtx, sender, conn, mode.clone(), true);
    }
    for (i, u) in lose.iter().enumerate() {
        user_score(u, (rl[i]-lose_score[i]) as i16, msgtx, sender, conn, mode.clone(), false);
    }
}

pub fn HandleSqlRequest(pool: mysql::Pool)
    -> Result<Sender<SqlData>, Error> {
        #[cfg(target_os = "linux")]
        let (tx1, rx1): (Sender<SqlData>, Receiver<SqlData>) = bounded(10000);
    
        #[cfg(not(target_os = "linux"))]
        let (tx1, rx1): (Sender<SqlData>, Receiver<SqlData>) = crossbeam::unbounded();
        let start = Instant::now();
        let mut hero: Vec<String> = Vec::new();
        let mut NewUsers: Vec<String> = Vec::new();
        let mut len = 0;
        let mut UpdateInfo: Vec<SqlGameInfoData> = Vec::new();
        let mut info_len = 0; 
        let mut UpdateReplay: Vec<SqlReplayData> = Vec::new();
        let mut replay_len = 0;
        let mut ExistRP: Vec<SqlReplayData> = Vec::new();
        let mut exist_len = 0;
        // let mut UpdateGame: Vec<SqlGameData> = Vec::new();
        // let mut game_len = 0;
        let mut AddBlack: Vec<Black> = Vec::new();
        let mut add_black_len = 0;
        let mut RemoveBlack: Vec<Black> = Vec::new();
        let mut remove_black_len = 0;

        #[cfg(target_os = "linux")]
        let update1000ms = tick(Duration::from_millis(1000));

        #[cfg(not(target_os = "linux"))]
        let (txxx, update1000ms) = crossbeam_channel::unbounded();
        #[cfg(not(target_os = "linux"))]
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                txxx.try_send(std::time::Instant::now()).unwrap();
                //println!("update200ms: rx len: {}, tx len: {}", rx.len(), txxx.len());
            }
        });

        thread::spawn(move || -> Result<(), Error> {
            let mut conn = pool.get_conn()?;
            loop{
                select! {

                    recv(update1000ms) -> _ => {
                        if len > 0 {
                            // insert new user in NewUsers
                            let mut insert_str: String = "insert into user (userid, name, status, Level, Exp, Money, Currency) values".to_string();
                            for (i, u) in NewUsers.iter().enumerate() {
                                let mut new_user = format!(" ('{}', 'default name', 'online', 1, 0, 0, 100)", u);
                                insert_str += &new_user;
                                
                                if i < len-1 {
                                    insert_str += ",";
                                }
                                
                            }
                            insert_str += ";";
                            
                            println!("{}", insert_str);
                            conn.query(insert_str.clone())?;
                            

                            

                            let sql = format!("select id from user where userid='{}';", NewUsers[0]);
                            //println!("sql: {}", sql);
                            let qres = conn.query(sql.clone())?;
                            let mut id = 0;
                            let mut name: String = "".to_owned();
                            for row in qres {
                                let a = row?.clone();
                                id = mysql::from_value(a.get("id").unwrap());
                            }

                            let mut insert_hero: String = "insert into user_hero (id, Hero, HeroLv, HeroMastery) values".to_string();
                            for i in 0..len {
                                for (j, h) in hero.iter().enumerate() {
                                    let mut new_hero = format!(" ('{}', '{}', 1, 0)", id+i, h);
                                    insert_hero += &new_hero;
                                    if j < hero.len()-1 {
                                        insert_hero += ",";
                                    }
                                }
                                if i < len-1 {
                                    insert_str += ",";
                                    insert_hero += ",";
                                }
                            }
                            insert_hero += ";";
                            conn.query(insert_hero.clone())?;
                            

                            let mut insert_rk: String = "insert into user_rk1v1 (id, score, Win, Lose) values".to_string();
                            for i in 0..len {
                                let mut new_user = format!(" ({}, 1000, 0, 0)", id+i);
                                insert_rk += & new_user;
                                if i < len-1 {
                                    insert_rk += ",";
                                }
                            }
                            insert_rk += ";";
                            //println!("{}", insert_rk);
                            conn.query(insert_rk.clone())?;
                            

                            let mut insert_rk1: String = "insert into user_rk5v5 (id, score, Win, Lose) values".to_string();
                            for i in 0..len {
                                let mut new_user = format!(" ({}, 1000, 0, 0)", id+i);
                                insert_rk1 += & new_user;
                                if i < len-1 {
                                    insert_rk1 += ",";
                                }
                            }
                            insert_rk1 += ";";
                            //println!("{}", insert_rk1);
                            conn.query(insert_rk1.clone())?;
                            


                            let mut insert_ng: String = "insert into user_ng1v1 (id, score, Win, Lose) values".to_string();
                            for i in 0..len {
                                let mut new_user = format!(" ({}, 1000, 0, 0)", id+i);
                                insert_ng += &new_user;
                                if i < len-1 {
                                    insert_ng += ",";
                                }
                            }
                            insert_ng += ";";
                            //println!("{}", insert_ng);
                            conn.query(insert_ng.clone())?;
                            
                            
                            let mut insert_ng1: String = "insert into user_ng5v5 (id, score, Win, Lose) values".to_string();
                            for i in 0..len {
                                let mut new_user = format!(" ({}, 1000, 0, 0)", id+i);
                                insert_ng1 += &new_user;
                                if i < len-1 {
                                    insert_ng1 += ",";
                                }
                            }
                            insert_ng1 += ";";
                            //println!("{}", insert_ng1);
                            conn.query(insert_ng1.clone())?;
                            

                            let mut insert_honor: String = "insert into user_honor (id, honor) values".to_string();
                            for i in 0..len {
                                let mut new_user = format!(" ({}, 50)", id+i);
                                insert_honor += &new_user;
                                if i < len-1 {
                                    insert_honor += ",";
                                }
                            }
                            insert_honor += ";";
                            //println!("{}", insert_honor);
                            conn.query(insert_honor.clone())?;
                            


                            len = 0;
                            NewUsers.clear();
                        }

                        if info_len > 0 {
                            println!("SQL Update game_info");
                            let mut insert_info: String = "insert into game_info (id, gameid, hero, damage, be_damage, K, D, A, Res, afk, BattleScore, Currency, equ, Talent_A, Talent_B, Talent_C, Talent_D, Talent_E) values".to_string();
                            for (i, info) in UpdateInfo.iter().enumerate() {
                                let mut new_user = format!(r#" ((select id from user where userid="{}"), {}, '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}', {}, {}, {}, {}, {})"#, info.id, info.game, info.hero, info.damage, info.be_damage, info.K, info.D, info.A, info.Res, info.afk, info.BattleScore, info.Currency, info.equ, info.Talent.A, info.Talent.B, info.Talent.C, info.Talent.D, info.Talent.E);
                                insert_info += &new_user;
                                if i < info_len-1 {
                                    insert_info += ",";
                                }
                            }
                            insert_info += ";";
                            println!("{}", insert_info);
                            {
                                conn.query(insert_info.clone())?;
                            }
                            

                            info_len = 0;
                            UpdateInfo.clear();
                        }

                        if replay_len > 0 {
                            let mut insert_replay: String = "insert into replay (gameid, replay, url, address) values".to_string();
                            for (i, replay) in UpdateReplay.iter().enumerate() {
                                let mut new_replay = format!(r#" ({}, "{}", "{}", "{}")"#, replay.gameid, replay.name, replay.url, replay.address);
                                insert_replay += &new_replay;
                                if i < replay_len-1 {
                                    insert_replay += ",";
                                }
                            }
                            insert_replay += ";";
                            println!("{}", insert_replay);
                            {
                                conn.query(insert_replay.clone())?;
                            }
                            replay_len = 0;
                            UpdateReplay.clear();
                        }

                        if exist_len > 0 {
                            for (i, replay) in ExistRP.iter().enumerate() {
                                let mut new_replay = format!(r#"UPDATE replay SET replay='{}', url='{}', address='{}' WHERE gameid={}"#,  replay.name, replay.url, replay.address, replay.gameid);
                                
                            }
                            exist_len = 0;
                            ExistRP.clear();
                        }

                        if add_black_len > 0 {
                            let mut insert_black: String = "insert into user_blacklist (id, black_id) values".to_string();
                            for (i, black) in AddBlack.iter().enumerate() {
                                let mut new_black = format!(r#" ((select id from user where userid="{}"), "{}")"#, black.id, black.black);
                                insert_black += &new_black;
                                if i < add_black_len-1 {
                                    insert_black += ",";
                                }
                            }
                            insert_black += ";";
                            println!("{}", insert_black);
                            {
                                conn.query(insert_black.clone())?;
                            }
                            add_black_len = 0;
                            AddBlack.clear();
                        }

                        if remove_black_len > 0 {
                            let mut delete_black: String = "DELETE FROM user_blacklist where".to_string();
                            for (i, black) in RemoveBlack.iter().enumerate() {
                                let mut new_black = format!(r#" ((select id from user where userid="{}") and "{}")"#, black.id, black.black);
                                delete_black += &new_black;
                                if i < remove_black_len-1 {
                                    delete_black += " or";
                                }
                            }
                            delete_black += ";";
                            println!("{}", delete_black);
                            {
                                conn.query(delete_black.clone())?;
                            }
                            remove_black_len = 0;
                            RemoveBlack.clear();
                        }

                    }

                    recv(rx1) -> d => {
                        let handle = || -> Result<(), Error> {
                            if let Ok(d) = d {
                                match d {
                                    
                                    SqlData::Login(x) => {     
                                        //println!("SQL Update Login");                                  
                                        NewUsers.push(x.id.clone());
                                        println!("New Users: {:?}", NewUsers);
                                        len+=1;                                        
                                    }
                                    SqlData::UpdateScore(x) => {
                                        //println!("SQL Update score");
                                        if x.mode == "ng1p2t" {
                                            if x.Res == true {
                                                let sql = format!("UPDATE user_ng1v1 as a JOIN user as b ON a.id=b.id SET score={}, Win=Win+1 WHERE b.userid='{}';", x.score, x.id);
                                                let qres = conn.query(sql.clone())?;
                                            } else {
                                                let sql = format!("UPDATE user_ng1v1 as a JOIN user as b ON a.id=b.id SET score={}, Lose=Lose+1 WHERE b.userid='{}';", x.score, x.id);
                                                let qres = conn.query(sql.clone())?;
                                            }
                                            //println!("sql: {}", sql);
                                            

                                        } else if x.mode == "ng5p2t" {
                                            if x.Res == true {
                                                let sql = format!("UPDATE user_ng5v5 as a JOIN user as b ON a.id=b.id SET score={}, Win=Win+1 WHERE b.userid='{}';", x.score, x.id);
                                                let qres = conn.query(sql.clone())?;
                                            } else {
                                                let sql = format!("UPDATE user_ng5v5 as a JOIN user as b ON a.id=b.id SET score={}, Lose=Lose+1 WHERE b.userid='{}';", x.score, x.id);
                                                let qres = conn.query(sql.clone())?;
                                            }
                                           

                                        } else if x.mode == "rk1p2t" {
                                            if x.Res == true {
                                                let sql = format!("UPDATE user_rk1v1 as a JOIN user as b ON a.id=b.id SET score={}, Win=Win+1 WHERE b.userid='{}';", x.score, x.id);
                                                let qres = conn.query(sql.clone())?;
                                            } else {
                                                let sql = format!("UPDATE user_rk1v1 as a JOIN user as b ON a.id=b.id SET score={}, Lose=Lose+1 WHERE b.userid='{}';", x.score, x.id);
                                                let qres = conn.query(sql.clone())?;
                                            }
                                            
                                        } else if x.mode == "rk5p2t" {
                                            if x.Res == true {
                                                let sql = format!("UPDATE user_rk5v5 as a JOIN user as b ON a.id=b.id SET score={}, Win=Win+1 WHERE b.userid='{}';", x.score, x.id);
                                                let qres = conn.query(sql.clone())?;
                                            } else {
                                                let sql = format!("UPDATE user_rk5v5 as a JOIN user as b ON a.id=b.id SET score={}, Lose=Lose+1 WHERE b.userid='{}';", x.score, x.id);
                                                let qres = conn.query(sql.clone())?;
                                            }
                                            

                                        }
                                        
                                    }
                                    SqlData::UpdateGameInfo(x) => {
                                        //println!("SQL Update game_info");
                                        UpdateInfo.push(x.clone());
                                        info_len += 1;
                                    }
                                    SqlData::UpdateReplay(x) => {
                                        //println!("SQL Update replay");
                                        UpdateReplay.push(x.clone());
                                        replay_len += 1;
                                    }
                                    SqlData::UpdateExistingRP(x) => {
                                        //println!("SQL Update replay");
                                        ExistRP.push(x.clone());
                                        exist_len += 1;
                                    }
                                    SqlData::AddBlack(x) => {
                                        println!("AddBlack");
                                        AddBlack.push(x.clone());
                                        add_black_len += 1;
                                    },
                                    SqlData::RemoveBlack(x) => {
                                        println!("RemoveBlack");
                                        RemoveBlack.push(x.clone());
                                        remove_black_len += 1;
                                    },
                                    SqlData::HeroNum(x) => {
                                        hero = x.hero_type.clone();
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

pub fn HandleQueueRequest(msgtx: Sender<MqttMsg>, sender: Sender<RoomEventData>, mode: String, team_size: i16, match_size: usize, score_interval: i16)
    -> Result<Sender<QueueData>, Error> {
    #[cfg(target_os = "linux")]
    let (tx, rx):(Sender<QueueData>, Receiver<QueueData>) = bounded(10000);
    
    #[cfg(not(target_os = "linux"))]
    let (tx, rx):(Sender<QueueData>, Receiver<QueueData>) = crossbeam::unbounded();
    let start = Instant::now();

    #[cfg(target_os = "linux")]
    let update = tick(Duration::from_millis(1000));    

    #[cfg(not(target_os = "linux"))]
    let (txxx, update) = crossbeam_channel::unbounded();
    #[cfg(not(target_os = "linux"))]
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            txxx.try_send(std::time::Instant::now()).unwrap();
            //println!("update200ms: rx len: {}, tx len: {}", rx.len(), txxx.len());
        }
    });
    //let mut team_size: i16 = 1;

    // if mode == "ng1p2t" {
    //     team_size = 1;
    // } else if mode == "rk1p2t" {
    //     team_size = 1;
    // } else if mode == "ng5p2t" {
    //     team_size = 5;
    // } else if mode == "rk5p2t" {
    //     team_size = 5;
    // }

    thread::spawn(move || -> Result<(), Error> {
        let mut QueueRoom: BTreeMap<u32, Rc<RefCell<QueueRoomData>>> = BTreeMap::new();
        let mut ReadyGroups: BTreeMap<u32, Rc<RefCell<ReadyGroupData>>> = BTreeMap::new();
        
        let mut group_id = 0;
        loop {
            select! {
                recv(update) -> _ => {
                    
                    let mut new_now = Instant::now();
                    
                    if QueueRoom.len() >= match_size {
                        let mut g: ReadyGroupData = Default::default();
                        let mut tq: Vec<Rc<RefCell<QueueRoomData>>> = vec![];
                        let mut id: Vec<u32> = vec![];
                        let mut new_now = Instant::now();
                        tq = QueueRoom.iter().map(|x|Rc::clone(x.1)).collect();
                        //tq.sort_by_key(|x| x.borrow().avg_rk);
                        //println!("Collect Time: {:?}",Instant::now().duration_since(new_now));
                        
                        
                        let mut new_now = Instant::now();
                        if mode == "ng1p2t" {
                            tq.sort_by_key(|x| x.borrow().avg_ng1v1);
                        } else if mode == "rk1p2t" {
                            tq.sort_by_key(|x| x.borrow().avg_rk1v1);
                        } else if mode == "ng5p2t" {
                            tq.sort_by_key(|x| x.borrow().avg_ng5v5);
                        } else if mode == "rk5p2t" {
                            tq.sort_by_key(|x| x.borrow().avg_rk5v5);
                        }
                        //println!("Sort Time: {:?}",Instant::now().duration_since(new_now));
                        let mut new_now1 = Instant::now();
                        for i in 1..team_size+1 {
                            for (k, v) in &mut QueueRoom {
                                if v.borrow().user_len > i {
                                    continue
                                }
                                let mut block: bool = false;
                                
<<<<<<< HEAD
                                let Difference: i16 = i16::abs(v.borrow().avg_ng - g.avg_ng);
                                if g.avg_ng == 0 || Difference <= SCORE_INTERVAL * v.borrow().queue_cnt {
                                    g.rid.push(v.borrow().rid);
                                    let mut ng ;
                                    if (g.user_len + v.borrow().user_len > 0){
                                        ng = (g.avg_ng * g.user_len + v.borrow().avg_ng * v.borrow().user_len) as i16 / (g.user_len + v.borrow().user_len) as i16;
                                    } else {
                                        g = Default::default();
                                        continue;
                                    }
                                    g.avg_ng = ng;
                                    g.user_len += v.borrow().user_len;
                                    v.borrow_mut().ready = 1;
                                    v.borrow_mut().gid = group_id + 1;
=======
                                for u in &v.borrow_mut().user_name {
                                    if g.blacklist.contains(u) || g.block.contains(u){
                                        block = true;
                                        break;
                                    }
>>>>>>> ff381ae6d0750df3b7222f81e6dc00f911081b93
                                }
                                for u in &g.user_name {
                                    if v.borrow().blacklist.contains(&u) || v.borrow().block.contains(&u){
                                        block = true;
                                        break;
                                    }
                                }
                                if !g.allow_same_hero {
                                    for h in &g.hero {
                                        if v.borrow().hero.contains(&h) {
                                            block = true;
                                            break;
                                        }
                                    }
                                }
                                if (g.user_len != 0 && g.honor != v.borrow().honor) {
                                     block = true;
                                }
                                if block {
                                    continue;
                                }
                                let mut group_score = 0;
                                if mode == "ng1p2t" {
                                    group_score = g.avg_ng1v1;
                                } else if mode == "rk1p2t" {
                                    group_score = g.avg_rk1v1;
                                } else if mode == "ng5p2t" {
                                    group_score = g.avg_ng5v5;
                                } else if mode == "rk5p2t" {
                                    group_score = g.avg_rk5v5;
                                }

                                let mut room_score = 0;
                                if mode == "ng1p2t" {
                                    room_score = v.borrow().avg_ng1v1;
                                } else if mode == "rk1p2t" {
                                    room_score = v.borrow().avg_rk1v1;
                                } else if mode == "ng5p2t" {
                                    room_score = v.borrow().avg_ng5v5;
                                } else if mode == "rk5p2t" {
                                    room_score = v.borrow().avg_rk5v5;
                                }

                                if g.user_len > 0 && g.user_len < team_size && (group_score + v.borrow().queue_cnt*score_interval) < room_score {
                                    for r in g.rid {
                                        id.push(r);
                                    }
                                    g = Default::default();
                                    g.rid.push(v.borrow().rid);
                                    g.max_room_len = v.borrow().user_len.clone();
                                    g.allow_same_hero = v.borrow().allow_same_hero.clone();
                                    g.hero = [g.hero.as_slice(), v.borrow().hero.clone().as_slice()].concat();
                                    g.block = [g.block.as_slice(), v.borrow().block.clone().as_slice()].concat();
                                    g.blacklist = [g.blacklist.as_slice(), v.borrow().blacklist.clone().as_slice()].concat();
                                    g.user_name = [g.user_name.as_slice(), v.borrow().user_name.as_slice()].concat();
                                    g.honor = v.borrow().honor.clone();
                                    let mut ng = (group_score * g.user_len + room_score * v.borrow().user_len) as i16 / (g.user_len + v.borrow().user_len) as i16;
                                    if mode == "ng1p2t" {
                                        g.avg_ng1v1 = ng;
                                    } else if mode == "rk1p2t" {
                                        g.avg_rk1v1 = ng;
                                    } else if mode == "ng5p2t" {
                                        g.avg_ng5v5 = ng;
                                    } else if mode == "rk5p2t" {
                                        g.avg_rk5v5 = ng;
                                    }
                                    
                                    g.user_len += v.borrow().user_len;
                                    v.borrow_mut().ready = 1;
                                    v.borrow_mut().gid = group_id + 1;
                                    v.borrow_mut().queue_cnt += 1;
                                    continue;
                                }

                                if v.borrow().ready == 0 &&
                                    v.borrow().user_len as i16 + g.user_len <= team_size {
                                    if g.user_len == 0 {
                                        g.allow_same_hero = v.borrow().allow_same_hero.clone();
                                    }
                                    let Difference: i16 = i16::abs(room_score - group_score);
                                    if group_score == 0 || Difference <= score_interval * v.borrow().queue_cnt {
                                        g.rid.push(v.borrow().rid);
                                        g.hero = [g.hero.as_slice(), v.borrow().hero.clone().as_slice()].concat();
                                        g.block = [g.block.as_slice(), v.borrow().block.clone().as_slice()].concat();
                                        g.user_name = [g.user_name.as_slice(), v.borrow().user_name.as_slice()].concat();
                                        if g.user_len == 0 {
                                            g.honor = v.borrow().honor.clone();
                                        }
                                        let mut score = (group_score * g.user_len + room_score * v.borrow().user_len) as i16 / (g.user_len + v.borrow().user_len) as i16;
                                        if mode == "ng1p2t" {
                                            g.avg_ng1v1 = score;
                                        } else if mode == "rk1p2t" {
                                            g.avg_rk1v1 = score;
                                        } else if mode == "ng5p2t" {
                                            g.avg_ng5v5 = score;
                                        } else if mode == "rk5p2t" {
                                            g.avg_rk5v5 = score;
                                        }
                                        if (g.max_room_len < v.borrow().user_len) {
                                            g.max_room_len = v.borrow().user_len.clone()
                                        }
                                        g.user_len += v.borrow().user_len;
                                        v.borrow_mut().ready = 1;
                                        v.borrow_mut().gid = group_id + 1;
                                    }
                                    else {
                                        v.borrow_mut().queue_cnt += 1;
                                    }
                                }
                                if g.user_len == team_size {
                                   
                                    //println!("match team_size!");
                                    group_id += 1;
                                    //info!("new group_id: {}", group_id);
                                    g.gid = group_id;
                                    
                                    g.queue_cnt = 1;
                                    ReadyGroups.insert(group_id, Rc::new(RefCell::new(g.clone())));
                                    
                                    g = Default::default();
                                }

                            }
                        }
                        //println!("Time 2: {:?}",Instant::now().duration_since(new_now1));
                        if g.user_len < team_size {
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
                    
                    if ReadyGroups.len() >= match_size {
                        let mut fg: ReadyGameData = Default::default();
                        //let mut prestart = false;
                        let mut total_score: i16 = 0;
                        let mut rm_ids: Vec<u32> = vec![];
                        let mut tq: Vec<Rc<RefCell<ReadyGroupData>>> = vec![];
                        tq = ReadyGroups.iter().map(|x|Rc::clone(x.1)).collect();
                        //println!("ReadyGroup!! {}", ReadyGroups.len());
                        let mut new_now2 = Instant::now();
                        tq.sort_by_key(|x| x.borrow().max_room_len);
                        tq.iter().rev();
                        
                        for (id, rg) in &mut ReadyGroups {
                            
                            let mut block: bool = false;
                            
                            for u in &rg.borrow_mut().user_name {
                                if fg.block.contains(u) {
                                    block = true;
                                    break;
                                }                                
                            }
                            for u in &fg.user_name {
                                if rg.borrow().block.contains(&u) {
                                    block = true;
                                    break;
                                }   
                            }
                            if (fg.team_len != 0 && fg.honor != rg.borrow().honor) {
                                block = true;
                            }
                            if !fg.allow_same_hero {
                                for h in &fg.hero {
                                    if rg.borrow().hero.contains(&h) {
                                        block = true;
                                        break;
                                    }
                                }
                            }
                            if block {
                                continue;
                            }
                            
                            let mut group_score = 0;
                            if mode == "ng1p2t" {
                                group_score = rg.borrow().avg_ng1v1;
                            } else if mode == "rk1p2t" {
                                group_score = rg.borrow().avg_rk1v1;
                            } else if mode == "ng5p2t" {
                                group_score = rg.borrow().avg_ng5v5;
                            } else if mode == "rk5p2t" {
                                group_score = rg.borrow().avg_rk5v5;
                            }

                            
                            if rg.borrow().game_status == 0 && fg.team_len < match_size {
                                if total_score == 0 {
                                    total_score += group_score as i16;
                                    fg.group.push(rg.borrow().rid.clone());
                                    fg.honor = rg.borrow().honor;
                                    fg.gid.push(*id);
                                    fg.team_len += 1;
                                    fg.block = [fg.block.as_slice(), rg.borrow().block.clone().as_slice()].concat();
                                    fg.hero = [fg.hero.as_slice(), rg.borrow().hero.clone().as_slice()].concat();
                                    fg.allow_same_hero = rg.borrow().allow_same_hero.clone();
                                    continue;
                                }
                                
                                let mut difference = 0;
                                if fg.team_len > 0 {
                                    difference = i16::abs(group_score as i16 - total_score/fg.team_len as i16);
                                }
                                if difference <= score_interval * rg.borrow().queue_cnt {
                                    total_score += group_score as i16;
                                    fg.group.push(rg.borrow().rid.clone());
                                    fg.block = [fg.block.as_slice(), rg.borrow().block.clone().as_slice()].concat();
                                    fg.user_name = [fg.user_name.as_slice(), rg.borrow().user_name.clone().as_slice()].concat();
                                    fg.hero = [fg.hero.as_slice(), rg.borrow().hero.clone().as_slice()].concat();
                                    fg.team_len += 1;
                                    fg.gid.push(*id);
                                }
                                else {
                                    rg.borrow_mut().queue_cnt += 1;
                                }
                            }
                            if fg.team_len == match_size {
                                println!("{} UpdateGame", mode);
                                //std::thread::sleep(std::time::Duration::from_millis(5000));
                                sender.send(RoomEventData::UpdateGame(PreGameData{rid: fg.group.clone(), mode: mode.clone()}));
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
                                    println!("mode: {}, rid: {}, block: {:?}, honor: {}",mode, x.rid, x.block, x.honor);
                                    QueueRoom.insert(x.rid.clone(), Rc::new(RefCell::new(x.clone())));
                                }
                                QueueData::RemoveRoom(x) => {
                                    //println!("Remove Room!!! mode, {}, x.rid: {}",mode, &x.rid);
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



pub fn init(msgtx: Sender<MqttMsg>, sender: Sender<SqlData>, pool: mysql::Pool, server_addr: String, isBackup: bool) 
    -> Result<Sender<RoomEventData>, Error> {
    let mut ModeCfg: BTreeMap<String, GameModeCfg> = BTreeMap::new();
    #[cfg(target_os = "linux")]
    let (tx, rx):(Sender<RoomEventData>, Receiver<RoomEventData>) = bounded(10000);
    
    #[cfg(not(target_os = "linux"))]
    let (tx, rx):(Sender<RoomEventData>, Receiver<RoomEventData>) = crossbeam::unbounded();
    let mut QueueSender: BTreeMap<String, Sender<QueueData>> = BTreeMap::new();
    
    let file_path = "src/config.toml";
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
    let config: Config = toml::from_str(&str_val).unwrap();
    
    let score_interval = config.game_setting.clone().unwrap().SCORE_INTERVAL.unwrap();
    let honor_threshold = config.game_setting.clone().unwrap().HONOR_THRESHOLD.unwrap();
    let block_recent_player_of_games = config.game_setting.clone().unwrap().BLOCK_RECENT_PLAYER_OF_GAMES.unwrap();
    
    for x in config.game_mode.unwrap() {
        let gmc = GameModeCfg {
            mode: x.MODE.clone().unwrap(),
            team_size: x.TEAM_SIZE.unwrap(),
            match_size: x.MATCH_SIZE.unwrap()
        };
        let mut tx1 = HandleQueueRequest(msgtx.clone(), tx.clone(), x.MODE.clone().unwrap(), x.TEAM_SIZE.unwrap(), x.MATCH_SIZE.unwrap(), score_interval)?;
        QueueSender.insert(x.MODE.clone().unwrap(), tx1.clone());
        ModeCfg.insert(x.MODE.clone().unwrap(), gmc);
    }
    
    let hero = config.game_setting.clone().unwrap().HERO.unwrap();
    sender.try_send(SqlData::HeroNum(SqlHeroname {hero_type: hero.clone()}));
    // let mut tx1 = HandleQueueRequest(msgtx.clone(), tx.clone(), "ng1p2t".to_string())?;
    // let mut tx2 = HandleQueueRequest(msgtx.clone(), tx.clone(), "rk1p2t".to_string())?;
    // let mut tx3 = HandleQueueRequest(msgtx.clone(), tx.clone(), "ng5p2t".to_string())?;
    // let mut tx4 = HandleQueueRequest(msgtx.clone(), tx.clone(), "rk5p2t".to_string())?;
    
    // QueueSender.insert("ng1p2t".to_string(), tx1.clone());
    // QueueSender.insert("rk1p2t".to_string(), tx2.clone());
    // QueueSender.insert("ng5p2t".to_string(), tx3.clone());
    // QueueSender.insert("rk5p2t".to_string(), tx4.clone());
    // let mut tx1: Sender<QueueData>;
    // match QueueSender1.clone() {
    //     Some(s) => {
    //         tx1 = s;
    //         println!("in");
    //     },
    //     None => {
    //         tx1 = HandleQueueRequest(msgtx.clone(), tx.clone())?;
    //         println!("2 in");
    //     },
    // }
    #[cfg(target_os = "linux")]
    let update5000ms = tick(Duration::from_millis(5000));
    #[cfg(target_os = "linux")]
    let update200ms = tick(Duration::from_millis(200));
    #[cfg(target_os = "linux")]
    let update20000ms = tick(Duration::from_millis(20000));

    #[cfg(not(target_os = "linux"))]
    let (txxx, update5000ms) = crossbeam_channel::unbounded();
    #[cfg(not(target_os = "linux"))]
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(5000));
            txxx.try_send(std::time::Instant::now()).unwrap();
            //println!("update5000ms: rx len: {}, tx len: {}", rx.len(), txxx.len());
        }
    });
    #[cfg(not(target_os = "linux"))]
    let (txxx, update200ms) = crossbeam_channel::unbounded();
    #[cfg(not(target_os = "linux"))]
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(200));
            txxx.try_send(std::time::Instant::now()).unwrap();
            //println!("update200ms: rx len: {}, tx len: {}", rx.len(), txxx.len());
        }
    });

    #[cfg(not(target_os = "linux"))]
    let (txxx, update20000ms) = crossbeam_channel::unbounded();
    #[cfg(not(target_os = "linux"))]
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(20000));
            txxx.try_send(std::time::Instant::now()).unwrap();
            //println!("update200ms: rx len: {}, tx len: {}", rx.len(), txxx.len());
        }
    });
    
    let start = Instant::now();
    //let QueueSender = tx1.clone();
    // let t = SystemTime::now();
    // println!("start: {:?}", start);
    // println!("system time: {:?}", t);

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
        let mut TotalReplay: BTreeMap<u32, Rc<RefCell<Replay>>> = BTreeMap::new();
        let mut TotalGameServer: Vec<Rc<RefCell<GameServer>>> = vec![];
        let mut LossSend: Vec<MqttMsg> = vec![];
        let mut room_id: u32 = 0;
        let mut group_id: u32 = 0;
        let mut game_id: u32 = 0;
        let mut game_port: u16 = 7777;

        // Equipment
        let mut TotalEquip: BTreeMap<u32, Rc<RefCell<Equipment>>> = BTreeMap::new();
        let mut TotalEquOption: BTreeMap<u32, Rc<RefCell<EquOption>>> = BTreeMap::new();
        
        // let sql = format!(r#"select userid, a.score as ng1v1, b.score as rk1v1, d.score as ng5v5, e.score as rk5v5, f.honor as honor, g.Level, g.Exp, h.Rank, h.RankLevel, h.Score, i.Win, i.Lose, j.Total_Currency, name from user as c 
        //                     join user_ng1v1 as a on a.id=c.id 
        //                     join user_rk1v1 as b on b.id=c.id
        //                     join user_ng5v5 as d on d.id=c.id
        //                     join user_rk5v5 as e on e.id=c.id
        //                     join user_honor as f on f.id=c.id
        //                     join user_level as g on g.id=c.id
        //                     join user_rank as h on h.id=c.id
        //                     join user_battle as i on i.id=c.id
        //                     join user_currency as j on j.id=c.id;"#);
        let sql = format!(r#"select userid, name, Level, Exp, Money, Currency, a.score as rk1v1Score, a.Win as rk1v1Win, a.Lose as rk1v1Lose, b.score as ng1v1Score, b.Win as ng1v1Win, b.Lose as ng1v1Lose, c.score as rk5v5Score, c.Win as rk5v5Win, c.Lose as rk5v5Lose, d.score as ng5v5Score, d.Win as ng5v5Win, d.Lose as ng5v5Lose, e.honor as honor from user as f 
                            join user_rk1v1 as a on a.id=f.id
                            join user_ng1v1 as b on b.id=f.id
                            join user_rk5v5 as c on c.id=f.id
                            join user_ng5v5 as d on d.id=f.id
                            join user_honor as e on e.id=f.id;"#);
        let qres2: mysql::QueryResult = conn.query(sql.clone())?;
        let mut userid: String = "".to_owned();
        let mut ng: i16 = 0;
        let mut rk: i16 = 0;
        let mut name: String = "".to_owned();
        let id = 0;
        
        
        for row in qres2 {
            let a = row?.clone();
            let id: String =mysql::from_value(a.get("userid").unwrap());           
            let mut user = User {
                id: id.clone(),
                hero: mysql::from_value(a.get("name").unwrap()),
                online: false,
                honor: mysql::from_value(a.get("honor").unwrap()),
                ..Default::default()
            };

            

            let ng1v1Info = ScoreInfo {
                score: mysql::from_value(a.get("ng1v1Score").unwrap()),
                WinCount: mysql::from_value(a.get("ng1v1Win").unwrap()),
                LoseCount: mysql::from_value(a.get("ng1v1Lose").unwrap()),
            };
            let rk1v1Info = ScoreInfo {
                score: mysql::from_value(a.get("rk1v1Score").unwrap()),
                WinCount: mysql::from_value(a.get("rk1v1Win").unwrap()),
                LoseCount: mysql::from_value(a.get("rk1v1Lose").unwrap()),
            };
            let ng5v5Info = ScoreInfo {
                score: mysql::from_value(a.get("ng5v5Score").unwrap()),
                WinCount: mysql::from_value(a.get("ng5v5Win").unwrap()),
                LoseCount: mysql::from_value(a.get("ng5v5Lose").unwrap()),
            };
            let rk5v5Info = ScoreInfo {
                score: mysql::from_value(a.get("rk5v5Score").unwrap()),
                WinCount: mysql::from_value(a.get("rk5v5Win").unwrap()),
                LoseCount: mysql::from_value(a.get("rk5v5Lose").unwrap()),
            };

            user.ng1v1 = ng1v1Info;
            user.ng5v5 = ng5v5Info;
            user.rk1v1 = rk1v1Info;
            user.rk5v5 = rk5v5Info;

            user.info.PlayerLv =  mysql::from_value(a.get("Level").unwrap());
            user.info.PlayerExp =  mysql::from_value(a.get("Exp").unwrap());
            user.info.TotalCurrency = mysql::from_value(a.get("Currency").unwrap());
            
            // let equ = UserEquInfo {
            //     equ_id: 1,
            //     rank: 1,
            //     lv: 3,
            //     lv5: 2,
            //     option1lv: 0,
            //     option2lv: 0,
            //     option3lv: 0,
            //     ..Default::default()
            // };
            // user.info.TotalEquip.push(equ);
            //println!("{:?}", user);
            TotalUsers.insert(id, Rc::new(RefCell::new(user.clone())));
        }




        let es = format!(r#"select a.userid, equ_id, Rank, Lv, Lv5, Option1, Option2, Option3, Option1Lv, Option2Lv, Option3Lv from equ_info as b join user as a on a.id=b.id;"#);
        let eq = conn.query(es.clone())?;
            
        for r in eq {
            let a = r?.clone();

            

            let mut tmp: Vec<String> = vec![];
            userid = mysql::from_value(a.get("userid").unwrap());
            let mut u = TotalUsers.get(&userid);
            if let Some(u) = u {
                let equ = UserEquInfo {
                    equ_id: mysql::from_value(a.get("equ_id").unwrap()),
                    rank: mysql::from_value(a.get("Rank").unwrap()),
                    lv: mysql::from_value(a.get("Lv").unwrap()),
                    lv5: mysql::from_value(a.get("Lv5").unwrap()),
                    option1: mysql::from_value(a.get("Option1").unwrap()),
                    option2: mysql::from_value(a.get("Option2").unwrap()),
                    option3: mysql::from_value(a.get("Option3").unwrap()),
                    option1lv: mysql::from_value(a.get("Option1Lv").unwrap()),
                    option2lv: mysql::from_value(a.get("Option2Lv").unwrap()),
                    option3lv: mysql::from_value(a.get("Option3Lv").unwrap()),
                    ..Default::default()
                };
                u.borrow_mut().info.TotalEquip.push(equ);

                // u.borrow_mut().blacklist.push(mysql::from_value(a.get("black_id").unwrap()));
                // println!("userid: {}, blacklist: {:?}", u.borrow().id, u.borrow().blacklist);
            }
            
        }


        let s = format!(r#"select a.userid, black_id from user_blacklist as b join user as a on a.id=b.id;"#);
        let q = conn.query(s.clone())?;
            
        for r in q {
            let a = r?.clone();
            let mut tmp: Vec<String> = vec![];
            userid = mysql::from_value(a.get("userid").unwrap());
            let mut u = TotalUsers.get(&userid);
            if let Some(u) = u {
                u.borrow_mut().blacklist.push(mysql::from_value(a.get("black_id").unwrap()));
                println!("userid: {}, blacklist: {:?}", u.borrow().id, u.borrow().blacklist);
            }
            
        }
        
        let s = format!(r#"select a.userid, Hero, HeroLv, HeroMastery from user_hero as b join user as a on a.id=b.id;"#);
        let q = conn.query(s.clone())?;
            
        for r in q {
            let a = r?.clone();
            let mut tmp: Vec<String> = vec![];
            userid = mysql::from_value(a.get("userid").unwrap());
            let mut u = TotalUsers.get(&userid);
            if let Some(u) = u {
                let mut hero = Hero {
                    Hero_name: mysql::from_value(a.get("Hero").unwrap()),
                    Level: mysql::from_value(a.get("HeroLv").unwrap()),
                    Exp: mysql::from_value(a.get("HeroMastery").unwrap()),
                };
                u.borrow_mut().info.HeroExp.insert(hero.Hero_name.clone(), hero);
                //println!("userid: {}, hero: {:?}", u.borrow().id, u.borrow().info.HeroExp);
            }
            
        }

        println!("userid: {}", userid);
        //ng = mysql::from_value(a.get("ng").unwrap());
        //rk = mysql::from_value(a.get("rk").unwrap());
        //name = mysql::from_value(a.get("name").unwrap());
            
        let Equip_sql = format!("select * from equipment;");
        let Equip_qres2: mysql::QueryResult = conn.query(Equip_sql.clone())?;
        for row in Equip_qres2 {
            let a = row?.clone();
            let mut equ = Equipment {
                equ_id: mysql::from_value(a.get("equ_id").unwrap()),
                equ_name: mysql::from_value(a.get("equ_name").unwrap()),
                positive: mysql::from_value(a.get("Positive").unwrap()),
                negative: mysql::from_value(a.get("Negative").unwrap()),
                ..Default::default()
            };
            equ.pvalue.push(mysql::from_value(a.get("PLv1").unwrap()));
            equ.pvalue.push(mysql::from_value(a.get("PLv2").unwrap()));
            equ.pvalue.push(mysql::from_value(a.get("PLv3").unwrap()));
            equ.pvalue.push(mysql::from_value(a.get("PLv4").unwrap()));
            equ.pvalue.push(mysql::from_value(a.get("PLv5v1").unwrap()));
            equ.pvalue.push(mysql::from_value(a.get("PLv5v2").unwrap()));
            equ.pvalue.push(mysql::from_value(a.get("PLv5v3").unwrap()));
            equ.pvalue.push(mysql::from_value(a.get("PLv5v4").unwrap()));
            equ.pvalue.push(mysql::from_value(a.get("PLv5v5").unwrap()));

            equ.nvalue.push(mysql::from_value(a.get("NLv1").unwrap()));
            equ.nvalue.push(mysql::from_value(a.get("NLv2").unwrap()));
            equ.nvalue.push(mysql::from_value(a.get("NLv3").unwrap()));
            equ.nvalue.push(mysql::from_value(a.get("NLv4").unwrap()));
            equ.nvalue.push(mysql::from_value(a.get("NLv5v1").unwrap()));
            equ.nvalue.push(mysql::from_value(a.get("NLv5v2").unwrap()));
            equ.nvalue.push(mysql::from_value(a.get("NLv5v3").unwrap()));
            equ.nvalue.push(mysql::from_value(a.get("NLv5v4").unwrap()));
            equ.nvalue.push(mysql::from_value(a.get("NLv5v5").unwrap()));

            equ.option_id.push(mysql::from_value(a.get("Option1").unwrap()));
            equ.option_id.push(mysql::from_value(a.get("Option2").unwrap()));
            equ.option_id.push(mysql::from_value(a.get("Option3").unwrap()));
            println!("equ: {:?}", equ);
            TotalEquip.insert(equ.equ_id, Rc::new(RefCell::new(equ.clone())));
            
        }

        let EquOption_sql = format!("select * from equ_option;");
        let EquOption_qres2: mysql::QueryResult = conn.query(EquOption_sql.clone())?;
        for row in EquOption_qres2 {
            let a = row?.clone();
            let mut opt = EquOption {
                option_id: mysql::from_value(a.get("option_id").unwrap()),
                option_name: mysql::from_value(a.get("option_name").unwrap()),
                special: mysql::from_value(a.get("special").unwrap()),
                exception: mysql::from_value(a.get("exception").unwrap()),
                set_id: mysql::from_value(a.get("set_id").unwrap()),
                set_amount: mysql::from_value(a.get("set_amount").unwrap()),
                option_weight: mysql::from_value(a.get("option_weight").unwrap()),
                ..Default::default()
            };
            opt.effect.push(mysql::from_value(a.get("effect_v1").unwrap()));
            opt.effect.push(mysql::from_value(a.get("effect_v2").unwrap()));
            opt.effect.push(mysql::from_value(a.get("effect_v3").unwrap()));
            opt.effect.push(mysql::from_value(a.get("effect_v4").unwrap()));
            opt.effect.push(mysql::from_value(a.get("effect_v5").unwrap()));
            TotalEquOption.insert(opt.option_id, Rc::new(RefCell::new(opt.clone())));
        }

        let replay_sql = format!(r#"select gameid, replay, url, address from replay;"#);
        let replay_qres2: mysql::QueryResult = conn.query(replay_sql.clone())?;
        
        
        for row in replay_qres2 {
            let a = row?.clone();
            let gameid: u32 = mysql::from_value(a.get("gameid").unwrap());           
            let replay = Replay {
                gameid: gameid.clone(),
                name: mysql::from_value(a.get("replay").unwrap()),
                url: mysql::from_value(a.get("url").unwrap()),
                address: mysql::from_value(a.get("address").unwrap()),
                ..Default::default()
            };
            //println!("{:?}", replay);
            TotalReplay.insert(gameid, Rc::new(RefCell::new(replay.clone())));
        }
        //println!("{:?}", TotalReplay);
        
        let get_game_id = format!("select MAX(gameid) from game_info;");
        let qres3: mysql::QueryResult = conn.query(get_game_id.clone())?;
        
        for row in qres3 {
            let a = row?.clone();
            let q = mysql::from_value_opt(a.get("MAX(gameid)").unwrap());
            let q = match q {
                Ok(q) => {
                    game_id = q;
                },
                Err(err) => {
                    game_id = 0;
                },
            };

            
            
            //println!("game id: {}", game_id);
        }
        
        println!("game id: {}", game_id);
        

        msgtx.try_send(MqttMsg{topic:format!("server/0/res/check"), msg: r#"{"msg":"ok"}"#.to_string()})?;

        loop {
            crossbeam_channel::select! {
                
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
                        let mode = group.borrow().mode.clone();
                        
                        match res {
                            
                            PrestartStatus::Ready => {

                                if (mode == "rk1p2t" || mode == "rk5p2t") && !group.borrow().done.clone() {
                                    let mut t1 = group.borrow().teams[0].borrow_mut().get_group_order(group.borrow().mode.clone()).clone();
                                    let mut t2 = group.borrow().teams[1].borrow_mut().get_group_order(group.borrow().mode.clone()).clone();
                                    //let send = group.borrow().send;
                                    if !group.borrow().send.clone() {
                                        if group.borrow().ban.len() == 0 {
                                            // Start
                                            msgtx.try_send(MqttMsg{topic:format!("game/{}/res/rankgame_start", group.borrow().game_id), 
                                                        msg: format!(r#"{{"team1":{:?}, "team2":{:?}}}"#,t1, t2)})?;
                                            //group.borrow_mut().send = true;
                                        }
                                        // Ban the hero
                                        if group.borrow().order.clone() == 0 && group.borrow().ban.len() < 2 {
                                            
                                            if group.borrow().ban.len()%2 == 0 {
                                                // Team 1 ban a hero
                                                msgtx.try_send(MqttMsg{topic:format!("game/{}/res/rank_choose", group.borrow().game_id), 
                                                            msg: format!(r#"{{"msg":"ban", "member":"{}"}}"#, t1[0])})?;
                                                //group.borrow_mut().send = true;
                                            } else {
                                                // Team 2 ban a hero
                                                msgtx.try_send(MqttMsg{topic:format!("game/{}/res/rank_choose", group.borrow().game_id), 
                                                            msg: format!(r#"{{"msg":"ban", "member":"{}"}}"#, t2[0])})?;
                                                //group.borrow_mut().send = true;
                                            }
                                            
                                        }
                                        else {
                                            if group.borrow().order.clone() == 0 {
                                                msgtx.try_send(MqttMsg{topic:format!("game/{}/res/rank_choose", group.borrow().game_id), 
                                                            msg: format!(r#"{{"msg":"choose", "member":"{}"}}"#, t1[0])})?;
                                               // group.borrow_mut().send = true;
                                            } else if group.borrow().order.clone() < group.borrow().user_names.len() as u8{
                                                let od = group.borrow().order.clone()-1;
                                                if (od/2)%2 == 1 {
                                                    //Team 1 choose
                                                    msgtx.try_send(MqttMsg{topic:format!("game/{}/res/rank_choose", group.borrow().game_id), 
                                                            msg: format!(r#"{{"msg":"choose", "member":"{}"}}"#, t1[((od/2)+(od%2))as usize])})?;
                                                    //group.borrow_mut().send = true;
                                                } else if (od/2)%2 == 0 {
                                                    // Team 2 choose
                                                    msgtx.try_send(MqttMsg{topic:format!("game/{}/res/rank_choose", group.borrow().game_id), 
                                                            msg: format!(r#"{{"msg":"choose", "member":"{}"}}"#, t2[((od/2)+(od%2)) as usize])})?;
                                                    //group.borrow_mut().send = true;
                                                }
                                            }
                                            
                                        }
                                    }
                                    group.borrow_mut().send = true;
                                    // Test
                                    // group.borrow_mut().send = false;
                                    // if group.borrow().ban.len() < 2 {
                                    //     group.borrow_mut().ban.push("freyja".to_string());
                                    // } else {
                                    //     group.borrow_mut().order += 1;
                                    // }
                                    // if group.borrow().order.clone() >= group.borrow().user_names.len() as u8 {
                                    //     group.borrow_mut().done = true;
                                    // }

                                }
<<<<<<< HEAD
                                group.borrow_mut().ready();
                                group.borrow_mut().update_names();
                                group.borrow_mut().game_port = game_port;
                                
                                GameingGroups.remove(&group.borrow().game_id);
                                //PreStartGroups.remove(&group.borrow().game_id);
                                GameingGroups.insert(group.borrow().game_id.clone(), group.clone());
                                //info!("game_port: {}", game_port);
                                //info!("game id {}", group.borrow().game_id);
                                
                                let cmd = Command::new("/root/LinuxNoEditor/CF1/Binaries/Linux/CF1Server")
                                        .arg(format!("-Port={}", game_port))
                                        .arg(format!("-gameid {}", group.borrow().game_id))
                                        .arg("-NOSTEAM")
                                        .spawn();
                                        match cmd {
                                            Ok(_) => {},
                                            Err(_) => {warn!("Fail open CF1Server")},
=======
                                else {
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
                                    
                                    //info!("game_port: {}", game_port);
                                    //info!("game id {}", group.borrow().game_id);
                                    
                                    

                                    TotalGameServer.sort_by(|a, b| a.borrow().utilization.cmp(&b.borrow().utilization));
                                    
                                    for gs in TotalGameServer.clone() {
                                        if gs.borrow().now_user + group.borrow().user_names.len() as u32 > gs.borrow().max_user {
                                            continue;
>>>>>>> ff381ae6d0750df3b7222f81e6dc00f911081b93
                                        }
                                        println!("user: {}", group.borrow().user_names.len());
                                        msgtx.try_send(MqttMsg{topic:format!("server/{}/res/start_game", TotalGameServer[0].borrow().name.clone()), 
                                            msg: format!(r#"{{"game":{}, "port":"{}", "user":{}}}"#, group.borrow().game_id, game_port, group.borrow().user_names.len())})?;

                                        group.borrow_mut().server_name = TotalGameServer[0].borrow().name.clone();
                                        group.borrow_mut().server_notify = 1;
                                        group.borrow_mut().game_start = false;
                                        group.borrow_mut().update_names();
                                        GameingGroups.insert(group.borrow().game_id.clone(), group.clone());
                                        
                                        TotalGameServer[0].borrow_mut().now_server += 1;
                                        TotalGameServer[0].borrow_mut().update();
                                        break;
                                    }
                                    for gs in TotalGameServer.clone() {
                                        println!("GS afford: {}", gs.borrow().utilization);
                                    }
                                    // let cmd = Command::new(r#"/home/damody/LinuxNoEditor/CF1/Binaries/Linux/CF1Server"#)
                                    //         .arg(format!("-Port={}", game_port))
                                    //         .arg(format!("-gameid {}", group.borrow().game_id))
                                    //         .arg("-NOSTEAM")
                                    //         .spawn();
                                    //         match cmd {
                                    //             Ok(_) => {},
                                    //             Err(_) => {warn!("Fail open CF1Server")},
                                    //         }
                                    // if !isBackup || (isBackup && isServerLive == false){
                                    //     msgtx.try_send(MqttMsg{topic:format!("game/{}/res/game_signal", group.borrow().game_id), 
                                    //         msg: format!(r#"{{"game":{}}}"#, group.borrow().game_id)})?;
                                    //     LossSend.push(MqttMsg{topic:format!("game/{}/res/game_signal", group.borrow().game_id), 
                                    //         msg: format!(r#"{{"game":{}}}"#, group.borrow().game_id)});
                                    // }
                                }
                            },
                            PrestartStatus::Cancel => {
                                group.borrow_mut().update_names();
                                group.borrow_mut().clear_queue();
                                group.borrow_mut().game_status = 0;
                                let mut rm_rid: Vec<u32> = vec![]; 
                                let mut users: Vec<String> = vec![];
                                let mut heros: Vec<String> = vec![];
                                let mut block: Vec<String> = vec![];
                                let mut blacklist: Vec<String> = vec![];
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
                                            heros.push(u.borrow().hero.clone());
                                            for index in &u.borrow().recent_users {
                                                block = [block.as_slice(), index.clone().as_slice()].concat();
                                            }
                                            blacklist = [blacklist.as_slice(), u.borrow().blacklist.as_slice()].concat();
                                            
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
                                            let mut h: bool;
                                            
                                            if r.borrow().avg_honor.clone() < honor_threshold {
                                                h = false;
                                            } else {
                                                h = true;
                                            }
                                            let mut data = QueueRoomData {
                                                user_name: users.clone(),
                                                hero: heros.clone(),
                                                rid: r.borrow().rid.clone(),
                                                gid: 0,
                                                user_len: r.borrow().users.len().clone() as i16,
                                                avg_ng1v1: r.borrow().avg_ng1v1.clone(),
                                                avg_rk1v1: r.borrow().avg_rk1v1.clone(),
                                                avg_ng5v5: r.borrow().avg_ng5v5.clone(),
                                                avg_rk5v5: r.borrow().avg_rk5v5.clone(),
                                                honor: h,
                                                allow_same_hero: true,
                                                mode: group.borrow().mode.clone(),
                                                ready: 0,
                                                queue_cnt: 1,
                                                block: block.clone(),
                                                blacklist: blacklist.clone(),
                                            };
                                            if data.mode == "rk1p2t" || data.mode == "rk5p2t" {
                                                data.allow_same_hero = true;
                                            }
                                            let t1 = QueueSender.get(&group.borrow().mode.clone());
                                            if let Some(t1) = t1{
                                                t1.send(QueueData::UpdateRoom(data));
                                            }
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
                // recv(update1000ms) -> _ => {
                //     if !isBackup || (isBackup && isServerLive == false) {
                //         //msgtx.try_send(MqttMsg{topic:format!("server/0/res/heartbeat"), 
                //         //                    msg: format!(r#"{{"msg":"live"}}"#)})?;
                //     }
                // }

                recv(update5000ms) -> _ => {
                    let mut rm_ids: Vec<u32> = vec![];
                    // Rankchoose cnt
                    for (id, group) in &mut PreStartGroups{
                        if group.borrow().times > 7 {
                            println!("time up");
                            msgtx.try_send(MqttMsg{topic:format!("game/{}/res/rankgame_status", group.borrow().game_id.clone()), 
                                msg: format!(r#"{{"msg":"timeout"}}"#)})?;
                            for r in &group.borrow().room_names {
                                
                                msgtx.try_send(MqttMsg{topic:format!("room/{}/res/prestart", r), 
                                    msg: format!(r#"{{"msg":"stop queue"}}"#)})?;
                                
                            }
                            rm_ids.push(*id);
                        }
                        if group.borrow().send.clone() {
                            group.borrow_mut().times += 1;
                        }
                    }
                    for k in rm_ids {   
                        PreStartGroups.remove(&k);
                    }
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
                
                recv(update20000ms) -> _ => {
                    for (id, group) in &mut GameingGroups {
                        if group.borrow().game_start == false {
                            if group.borrow().server_notify > 5 {
                                // let s = TotalGameServer.iter().find(|&x| x.borrow().name == group.borrow().server_name);
                                // if let Some(s) = s {
                                //     s.borrow_mut().now_server -= 1;
                                //     s.borrow_mut().update();
                                   
                                // }
                                let index = TotalGameServer.iter().position(|x| *x.borrow().name == group.borrow().server_name).unwrap();
                                TotalGameServer.remove(index);
                                TotalGameServer.sort_by(|a, b| a.borrow().utilization.cmp(&b.borrow().utilization));
                                // if TotalGameServer[0].borrow().name == group.borrow().server_name {
                                //     // TotalGameServer.sort_by(|a, b| a.borrow().utilization.cmp(&b.borrow().utilization));
                                //     msgtx.try_send(MqttMsg{topic:format!("server/{}/res/start_game", TotalGameServer[1].borrow().name.clone()), 
                                //         msg: format!(r#"{{"game":{}, "port":"{}"}}"#, group.borrow().game_id, group.borrow().game_port)})?;

                                //     group.borrow_mut().server_name = TotalGameServer[1].borrow().name.clone();
                                //     group.borrow_mut().server_notify = 1;
                                //     group.borrow_mut().game_start = false;
                                //     // GameingGroups.insert(group.borrow().game_id.clone(), group.clone());
                                    
                                //     TotalGameServer[1].borrow_mut().now_server += 1;
                                //     TotalGameServer[1].borrow_mut().update();
                                // } else {
                                if TotalGameServer.len() > 0 {
                                    msgtx.try_send(MqttMsg{topic:format!("server/{}/res/start_game", TotalGameServer[0].borrow().name.clone()), 
                                    msg: format!(r#"{{"game":{}, "port":"{}"}}"#, group.borrow().game_id, group.borrow().game_port)})?;

                                    group.borrow_mut().server_name = TotalGameServer[0].borrow().name.clone();
                                    group.borrow_mut().server_notify = 1;
                                    group.borrow_mut().game_start = false;
                                    // GameingGroups.insert(group.borrow().game_id.clone(), group.clone());
                                    
                                    TotalGameServer[0].borrow_mut().now_server += 1;
                                    TotalGameServer[0].borrow_mut().update();
                                }
                            } else {
                                
                                msgtx.try_send(MqttMsg{topic:format!("server/{}/res/start_game", group.borrow().server_name.clone()), 
                                    msg: format!(r#"{{"game":{}, "port":"{}"}}"#, group.borrow().game_id, group.borrow().game_port)})?;
                                group.borrow_mut().server_notify += 1;
                                
                            }
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
                                        if u.borrow().online == false {
                                            mqttmsg = MqttMsg{topic:format!("member/{}/res/status", x.id), 
                                                msg: format!(r#"{{"msg":"offline"}}"#)};
                                        }
                                        else {
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
<<<<<<< HEAD
                                                msg: format!(r#"{{"server":"172.104.78.55:{}"}}"#, g.borrow().game_port)};
                                            //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/reconnect", x.id), 
                                            //    msg: format!(r#"{{"server":"172.104.78.55:{}"}}"#, g.borrow().game_port)})?;
=======
                                                msg: format!(r#"{{"server":"{}:{}"}}"#, server_addr, g.borrow().game_port)};
                                            //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/reconnect", x.id), 
                                            //    msg: format!(r#"{{"server":"114.32.129.195:{}"}}"#, g.borrow().game_port)})?;
>>>>>>> ff381ae6d0750df3b7222f81e6dc00f911081b93
                                        }
                                    }
                                },
                                RoomEventData::GameClose(x) => {
                                    //let p = PreStartGroups.remove(&x.game);
                                    //    let cmd = Command::new(r#"/home/damody/LinuxNoEditor/CF1/Binaries/Linux/CF1Server"#)
                                    //         .arg(format!("-Port={}", game_port))
                                    //         .arg(format!("-gameid {}", group.borrow().game_id))
                                    //         .arg("-NOSTEAM")
                                    //         .spawn();
                                    //         match cmd {
                                    //             Ok(_) => {},
                                    //             Err(_) => {warn!("Fail open CF1Server")},
                                    //         }
                                    let g = GameingGroups.get(&x.game);
                                    if let Some(g) = g {
                                        let s = TotalGameServer.iter().find(|&x| x.borrow().name == g.borrow().server_name);
                                        if let Some(s) = s {
                                            s.borrow_mut().now_server -= 1;
                                            s.borrow_mut().update();
                                            msgtx.try_send(MqttMsg{topic:format!("server/{}/res/game_close", s.borrow().name.clone()), 
                                            msg: format!(r#"{{"game":{}, "user":{}}}"#, x.game, g.borrow().user_names.len())})?;
                                        }
                                        
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
                                    let g1 = GameingGroups.get(&x.game);
                                    if let Some(g1) = g1 {
                                        g1.borrow_mut().winteam = x.win;
                                        g1.borrow_mut().loseteam = x.lose;
                                        settlement_ng_score(&win, &lose, &msgtx, &sender, &mut conn, g1.borrow().mode.clone());
                                    }
                                    // remove game
                                    // let g = GameingGroups.remove(&x.game);
                                    // let mut users: Vec<String> = Vec::new();
                                    // match g {
                                    //     Some(g) => {
                                            
                                    //         for u in &g.borrow().user_names {
                                    //             let u = get_user(&u, &TotalUsers);
                                    //             if let Some(u) = u {
                                    //                 users.push(u.borrow().id.clone())
                                    //             }
                                    //         }
                                    //         for room in & g.borrow().room_names {
                                    //             let r = TotalRoom.get(&get_rid_by_id(&room, &TotalUsers));
                                    //             if let Some(r) = r {
                                    //                 println!("room id: {}, users: {}", r.borrow().rid, r.borrow().users.len());
                                    //                 r.borrow_mut().ready = 0;
                                    //             }
                                    //         }
                                    //     for u in &g.borrow().user_names {
                                    //         let u = get_user(&u, &TotalUsers);
                                    //         match u {
                                    //             Some(u) => {
                                    //                 // add recent users
                                    //                 if u.borrow().recent_users.len() != 0 && u.borrow().recent_users.len() >= block_recent_player_of_games {
                                    //                     u.borrow_mut().recent_users.remove(0);                                                        
                                    //                 }
                                    //                 if block_recent_player_of_games > 0 {
                                    //                     let mut tmp_users = users.clone();
                                    //                     let index = tmp_users.iter().position(|x| *x == u.borrow().id).unwrap();
                                    //                     tmp_users.remove(index);
                                    //                     u.borrow_mut().recent_users.push(tmp_users.clone());
                                    //                 }
                                    //                 // remove room
                                    //                 let r = TotalRoom.get(&u.borrow().rid);
                                    //                 let mut is_null = false;
                                    //                 if let Some(r) = r {
                                    //                     r.borrow_mut().rm_user(&u.borrow().id);
                                    //                     if r.borrow().users.len() == 0 {
                                    //                         is_null = true;
                                    //                         //info!("remove success {}", u.borrow().id);
                                    //                     }
                                    //                 }
                                    //                 else {
                                    //                     //info!("remove fail {}", u.borrow().id);
                                    //                 }
                                    //                 if is_null {
                                    //                     TotalRoom.remove(&u.borrow().rid);
                                    //                     //QueueRoom.remove(&u.borrow().rid);
                                    //                 }
                                    //                 // remove group
                                    //                 //ReadyGroups.remove(&u.borrow().gid);
                                    //                 // remove game
                                    //                 PreStartGroups.remove(&u.borrow().game_id);
                                    //                 GameingGroups.remove(&u.borrow().game_id);

                                    //                 u.borrow_mut().rid = 0;
                                    //                 u.borrow_mut().gid = 0;
                                    //                 u.borrow_mut().game_id = 0;
                                    //             },
                                    //             None => {
                                    //                 error!("remove fail ");
                                    //             }
                                    //         }
                                    //     }
                                    //     //g.borrow_mut().leave_room();
                                    //     for u in &g.borrow().user_names {
                                    //         let u = get_user(&u, &TotalUsers);
                                    //         match u {
                                    //             Some(u) => {
                                    //                 //mqttmsg = MqttMsg{topic:format!("member/{}/res/status", u.borrow().id), 
                                    //                 //    msg: format!(r#"{{"msg":"game_id = {}"}}"#, u.borrow().game_id)};
                                    //                 if !isBackup || (isBackup && isServerLive == false) {
                                    //                     msgtx.try_send(MqttMsg{topic:format!("member/{}/res/status", u.borrow().id), 
                                    //                         msg: format!(r#"{{"msg":"game_id = {}"}}"#, u.borrow().game_id)})?;
                                    //                     LossSend.push(MqttMsg{topic:format!("member/{}/res/status", u.borrow().id), 
                                    //                         msg: format!(r#"{{"msg":"game_id = {}"}}"#, u.borrow().game_id)});
                                    //                 }
                                    //             },
                                    //             None => {

                                    //             }
                                    //         }
                                    //     }
                                    //     //info!("Remove game_id!");
                                    //     }
                                    //     ,
                                    //     None => {
                                    //         //info!("remove game fail {}", x.game);
                                    //     }
                                    // }
                                },
                                RoomEventData::GameInfo(x) => {
                                    //println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                                    let mut data1: GameInfoRes = Default::default();
                                    data1.game = x.game.clone();
                                    let g = GameingGroups.get(&x.game);
                                    let mut users: Vec<String> = Vec::new();
                                    match g {
                                        Some(g) => {
                                            println!("Game Info");
                                            
                                            for u in &x.users {
                                                let mut userinfo: UserInfoRes = Default::default();
                                                let mut update_info: SqlGameInfoData = Default::default();
                                                let mut hero = "".to_string();
                                                let u1 = get_user(&u.steamid, &TotalUsers);
                                                if let Some(u1) = u1 {
                                                    hero = u1.borrow().hero.clone();
                                                
                                                    // SQL update
                                                    update_info.game = x.game.clone();
                                                    update_info.id = u.steamid.clone();
                                                    update_info.hero = u.hero.clone();
                                                    if g.borrow().winteam.contains(&u.steamid) {
                                                        update_info.Res = true;
                                                    } 
                                                    if g.borrow().loseteam.contains(&u.steamid) {
                                                        update_info.Res = false;
                                                    }
                                                    update_info.afk = false;
                                                    // for (i, e) in u.equ.iter().enumerate() {
                                                    //     update_info.equ += e;
                                                    //     if i < u.equ.len()-1 {
                                                    //         update_info.equ += ", ";
                                                    //     }
                                                    // }
                                                    update_info.damage = u.damage.clone();
                                                    update_info.be_damage = u.be_damage.clone();
                                                    update_info.K = u.K.clone();
                                                    update_info.D = u.D.clone();
                                                    update_info.A = u.A.clone();
                                                    update_info.BattleScore = 1;
                                                    update_info.Talent = u.Talent.clone();
                                                    //println!("Update Info: {:?}", update_info);
                                                    sender.try_send(SqlData::UpdateGameInfo(update_info));
            
                                                    // Ready for Response
                                                    userinfo.steamid = u.steamid.clone();
                                                    userinfo.hero = hero.clone();
                                                    userinfo.TotalCurrency = u1.borrow().info.TotalCurrency + u.Currency.clone();
                                                    u1.borrow_mut().info.TotalCurrency += u.Currency.clone();
                                                    userinfo.Currency = 20;
                                                    userinfo.WinCount = 15;
                                                    userinfo.LoseCount = 15;
                                                    //let mut h = u1.borrow_mut().info.HeroExp.get(&u.hero.clone());
                                                    if let Some(h) = u1.borrow_mut().info.HeroExp.get_mut(&u.hero.clone()) {
                                                        userinfo.LastHeroMastery = h.Exp;
                                                        userinfo.HeroMasteryLevel = h.Level;
                                                        userinfo.HeroMastery = h.Exp + 10;
                                                        h.Exp += 10;
                                                    }
                                                    
                                                    userinfo.LastPlayerExperience = u1.borrow().info.PlayerExp;
                                                    u1.borrow_mut().info.PlayerExp += 10;
                                                    userinfo.PlayerExperience = 20;
                                                    userinfo.PlayerLevel = u1.borrow().info.PlayerLv;
                                                    
                                                    userinfo.Rank = "Gold".to_string();
                                                    userinfo.LastRankScore = 25;
                                                    userinfo.RankScore = 40;
                                                    // if g.borrow().winteam.contains(&u.steamid) {
                                                    //     userinfo.RankBattleCount = 15;
                                                    // } else {
                                                    //     userinfo.RankBattleCount = -15;
                                                    // }
                                                }
                                                data1.users.push(userinfo);
                                            }
                                            msgtx.try_send(MqttMsg{topic:format!("game/{}/res/game_info", x.game), 
                                                        msg: json!(data1).to_string()})?;
                                            }
                                            None => {
                                                //info!("remove game fail {}", x.game);
                                            }
                                        }
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
                                            for room in & g.borrow().room_names {
                                                let r = TotalRoom.get(&get_rid_by_id(&room, &TotalUsers));
                                                if let Some(r) = r {
                                                    println!("room id: {}, users: {}", r.borrow().rid, r.borrow().users.len());
                                                    r.borrow_mut().ready = 0;
                                                }
                                            }
                                        for u in &g.borrow().user_names {
                                            let u = get_user(&u, &TotalUsers);
                                            match u {
                                                Some(u) => {
                                                    // add recent users
                                                    if u.borrow().recent_users.len() != 0 && u.borrow().recent_users.len() >= block_recent_player_of_games {
                                                        u.borrow_mut().recent_users.remove(0);                                                        
                                                    }
                                                    if block_recent_player_of_games > 0 {
                                                        let mut tmp_users = users.clone();
                                                        let index = tmp_users.iter().position(|x| *x == u.borrow().id).unwrap();
                                                        tmp_users.remove(index);
                                                        u.borrow_mut().recent_users.push(tmp_users.clone());
                                                    }
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
                                    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                                    
                                
                                },
                                RoomEventData::StartGame(x) => {
                                    let g = GameingGroups.get(&x.game);
                                    if let Some(g) = g {
<<<<<<< HEAD
                                        SendGameList(&g, &msgtx, &mut conn);
                                        for r in &g.borrow().room_names {
                                            if !isBackup || (isBackup && isServerLive == false) {
                                                msgtx.try_send(MqttMsg{topic:format!("room/{}/res/start", r), 
                                                    msg: format!(r#"{{"room":"{}","msg":"start","server":"172.104.78.55:{}","game":{}}}"#, 
                                                        r, g.borrow().game_port, g.borrow().game_id)})?;
=======
                                        if g.borrow().game_start == false {
                                            SendGameList(&g, &msgtx, &mut conn, &TotalEquip, &TotalEquOption);
                                            g.borrow_mut().game_start = true;
                                            for r in &g.borrow().room_names {
                                                if !isBackup || (isBackup && isServerLive == false) {
                                                    let s = TotalGameServer.iter().find(|&x| x.borrow().name == g.borrow().server_name);
                                                    if let Some(s) = s {
                                                    msgtx.try_send(MqttMsg{topic:format!("room/{}/res/start", r), 
                                                        msg: format!(r#"{{"room":"{}","msg":"start","server":"{}:{}","game":{}}}"#, 
                                                            r, s.borrow().address, g.borrow().game_port, g.borrow().game_id)})?;
                                                    }
                                                }
>>>>>>> ff381ae6d0750df3b7222f81e6dc00f911081b93
                                            }
                                            // for u in &g.borrow().user_names {
                                            //     sender.send(SqlData::UpdateGame(SqlGameData {gameid: x.game.clone(), userid: u.clone()}));
                                            // }
                                        }
                                    }
                                    
                                },
                                RoomEventData::Leave(x) => {
                                    let u = TotalUsers.get(&x.id);
                                    if let Some(u) = u {
                                        u.borrow_mut().start_prestart = false;
                                            let r = TotalRoom.get(&u.borrow().rid);
                                            //println!("id: {}, rid: {}", u.borrow().id, &get_rid_by_id(&u.borrow().id, &TotalUsers));
                                            let mut is_null = false;
                                            if let Some(r) = r {
                                                let m = r.borrow().master.clone();
                                                r.borrow_mut().rm_user(&x.id);
                                                if r.borrow().users.len() > 0 {
                                                    r.borrow().publish_update(&msgtx, m.clone())?;
                                                }
                                                else {
                                                    is_null = true;
                                                }
                                                
                                                //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/leave", x.id), 
                                                //    msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                                if is_null {
                                                    println!("Leave!!");
                                                    if r.borrow().mode != "" {
                                                        msgtx.try_send(MqttMsg{topic:format!("room/{}/res/cancel_queue", m.clone()), 
                                                                msg: format!(r#"{{"msg":"ok"}}"#)});
                                                        let t1 = QueueSender.get(&r.borrow().mode);
                                                        if let Some(t1) = t1 {
                                                            t1.send(QueueData::RemoveRoom(RemoveRoomData{rid: u.borrow().rid}));
                                                        }
                                                    }
                                                        //QueueRoom.remove(&u.borrow().rid);
                                                }
                                                mqttmsg = MqttMsg{topic:format!("room/{}/res/leave", x.id), 
                                                    msg: format!(r#"{{"msg":"ok", "id": "{}"}}"#, x.id)};
                                            }
                                            if is_null {
                                                TotalRoom.remove(&u.borrow().rid);
                                                //println!("Totalroom rid: {}", &u.borrow().rid);
                                                
                                                //QueueSender<>.send(QueueData::RemoveRoom(RemoveRoomData{rid: u.borrow().rid}));
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
                                        println!("gid: {}, id: {}", gid, u.borrow().id);
                                        if u.borrow().prestart_get == true {
                                            
                                            if gid != 0 {
                                                let g = ReadyGroups.get(&gid);
                                                if let Some(gr) = g {
                                                    if x.accept == true {
                                                        gr.borrow_mut().user_ready(&x.id);
                                                        //info!("PreStart user_ready");
                                                        mqttmsg = MqttMsg{topic:format!("room/{}/res/start_get", u.borrow().id), 
                                                            msg: format!(r#"{{"msg":"start", "room":"{}"}}"#, &x.room)};
                                                        //msgtx.try_send(MqttMsg{topic:format!("room/{}/res/start_get", u.borrow().id), 
                                                        //        msg: format!(r#"{{"msg":"start"}}"#)})?;
                                                    } else {
                                                        println!("accept false!");
                                                        gr.borrow_mut().user_cancel(&x.id);
                                                        let mut users: Vec<String> = vec![];
                                                        let mut heros: Vec<String> = vec![];
                                                        let mut block: Vec<String> = vec![];
                                                        let mut blacklist: Vec<String> = vec![];
                                                        for room in &gr.borrow().rooms {
                                                            for u in &room.borrow().users {
                                                                
                                                                users.push(u.borrow().id.clone());
                                                                heros.push(u.borrow().hero.clone());
                                                                for index in &u.borrow().recent_users {
                                                                    block = [block.as_slice(), index.clone().as_slice()].concat();
                                                                }
                                                                blacklist = [blacklist.as_slice(), u.borrow().blacklist.as_slice()].concat();
                                            
                                                            }
                                                        }
                                                        for r in &gr.borrow().rooms {
                                                            //println!("r_rid: {}, u_rid: {}", r.borrow().rid, u.borrow().rid);
                                                            if r.borrow().rid != u.borrow().rid {
                                                                let mut h: bool;
                                                                println!("PRESTART room avg honor: {}",r.borrow().avg_honor.clone());
                                                                if r.borrow().avg_honor.clone() < honor_threshold {
                                                                    h = false;
                                                                } else {
                                                                    h = true;
                                                                }
                                                                let mut data = QueueRoomData {
                                                                    user_name: users.clone(),
                                                                    hero: heros.clone(),
                                                                    rid: r.borrow().rid.clone(),
                                                                    gid: 0,
                                                                    user_len: r.borrow().users.len().clone() as i16,
                                                                    avg_ng1v1: r.borrow().avg_ng1v1.clone(),
                                                                    avg_rk1v1: r.borrow().avg_rk1v1.clone(),
                                                                    avg_ng5v5: r.borrow().avg_ng5v5.clone(),
                                                                    avg_rk5v5: r.borrow().avg_rk5v5.clone(),
                                                                    honor: h,
                                                                    allow_same_hero: true,
                                                                    mode: gr.borrow().mode.clone(),
                                                                    ready: 0,
                                                                    queue_cnt: 1,
                                                                    block: block.clone(),
                                                                    blacklist: blacklist.clone(),
                                                                };
                                                                if data.mode == "rk1p2t" || data.mode == "rk5p2t" {
                                                                    data.allow_same_hero = true;
                                                                }
                                                                let t1 = QueueSender.get(&r.borrow().mode);
                                                                if let Some(t1) = t1 {
                                                                    t1.send(QueueData::UpdateRoom(data));
                                                                }
                                                            } else {
                                                                r.borrow_mut().ready = 0;
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
                                    println!("Update Game!");
                                    let mut fg: FightGame = Default::default();
                                    let mut cancel_queue = false;
                                    for r in &x.rid {
                                        let mut g: FightGroup = Default::default();
                                        for rid in r {
                                            let room = TotalRoom.get(&rid);
                                            if let Some(room) = room {
                                                g.add_room(Rc::clone(&room));
                                                if room.borrow().ready == 0 {
                                                    cancel_queue = true;
                                                }
                                            }
                                        }
                                        
                                        //g.update_avg();
                                        //g.prestart();
                                        group_id += 1;
                                        g.set_group_id(group_id);
                                        g.game_status = 1;
                                        g.mode = x.mode.clone();
                                        ReadyGroups.insert(group_id, Rc::new(RefCell::new(g.clone())));
                                        let mut rg = ReadyGroups.get(&group_id);
                                        if let Some(rg) = rg {
                                            fg.teams.push(Rc::clone(rg));
                                        }
                                    }
                                    
                                    fg.update_names();
                                    let gm = ModeCfg.get(&x.mode);
                                    if let Some(gm) = gm {
                                        println!("FG users: {}, gmode_size: {}", fg.user_names.len(), gm.team_size as usize*gm.match_size);

                                        if fg.user_names.len() != (gm.team_size as usize*gm.match_size) || cancel_queue  {
                                            println!("in");
                                            for gr in fg.teams {
                                                let mut users: Vec<String> = vec![];
                                                let mut heros: Vec<String> = vec![];
                                                let mut block: Vec<String> = vec![];
                                                let mut blacklist: Vec<String> = vec![];
                                                for room in &gr.borrow().rooms {
                                                    for u in &room.borrow().users {
                                                        
                                                        users.push(u.borrow().id.clone());
                                                        heros.push(u.borrow().hero.clone());
                                                        for index in &u.borrow().recent_users {
                                                            block = [block.as_slice(), index.clone().as_slice()].concat();
                                                        }
                                                        blacklist = [blacklist.as_slice(), u.borrow().blacklist.as_slice()].concat();
                                    
                                                    }
                                                }
                                                for r in &gr.borrow().rooms {
                                                    //println!("r_rid: {}, u_rid: {}", r.borrow().rid, u.borrow().rid);
                                                    
                                                    let mut h: bool;
                                                    println!("PRESTART room avg honor: {}",r.borrow().avg_honor.clone());
                                                    if r.borrow().avg_honor.clone() < honor_threshold {
                                                        h = false;
                                                    } else {
                                                        h = true;
                                                    }
                                                    let mut data = QueueRoomData {
                                                        user_name: users.clone(),
                                                        hero: heros.clone(),
                                                        rid: r.borrow().rid.clone(),
                                                        gid: 0,
                                                        user_len: r.borrow().users.len().clone() as i16,
                                                        avg_ng1v1: r.borrow().avg_ng1v1.clone(),
                                                        avg_rk1v1: r.borrow().avg_rk1v1.clone(),
                                                        avg_ng5v5: r.borrow().avg_ng5v5.clone(),
                                                        avg_rk5v5: r.borrow().avg_rk5v5.clone(),
                                                        honor: h,
                                                        allow_same_hero: true,
                                                        mode: gr.borrow().mode.clone(),
                                                        ready: 0,
                                                        queue_cnt: 1,
                                                        block: block.clone(),
                                                        blacklist: blacklist.clone(),
                                                    };
                                                    if data.mode == "rk1p2t" || data.mode == "rk5p2t" {
                                                        data.allow_same_hero = true;
                                                    }
                                                    let t1 = QueueSender.get(&r.borrow().mode);
                                                    if let Some(t1) = t1 {
                                                        t1.send(QueueData::UpdateRoom(data));
                                                    }
                                                }
                                                
                                                
                                            }
                                            ReadyGroups.remove(&group_id);
                                            ReadyGroups.remove(&(group_id-1));
                                        }
                                        else {
                                            println!("Prestart");
                                            for gr in &fg.teams {
                                                gr.borrow_mut().prestart();
                                            }
                                            for r in &fg.room_names {
                                                //thread::sleep_ms(100);
                                                if !isBackup || (isBackup && isServerLive == false) {
                                                    msgtx.try_send(MqttMsg{topic:format!("room/{}/res/prestart", r), msg: r#"{"msg":"prestart"}"#.to_string()})?;
                                                }
                                            }
                                            fg.send = false;
                                            fg.done = false;
                                            fg.mode = x.mode.clone();
                                            game_id += 1;
                                            fg.set_game_id(game_id);
                                            PreStartGroups.insert(game_id, Rc::new(RefCell::new(fg)));
                                        }
                                    }
                                    
                                    

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
                                    let mut ban = false;
                                    let mut ban_t: u64 = 0;
                                    let u = TotalUsers.get(&x.id);
                                    let mut rid = 0;
                                    if let Some(u) = u {
                                        if u.borrow().rid != 0 {
                                            hasRoom = true;
                                            rid = u.borrow().rid;
                                        }
                                        let n = Instant::now();
                                        let dur = n.duration_since(u.borrow().ban.from);
                                        if dur < u.borrow().ban.long {
                                            println!("ban: {}", (u.borrow().ban.long.clone()-dur).as_secs());
                                            ban_t = (u.borrow().ban.long.clone()-dur).as_secs();
                                            // mqttmsg = MqttMsg{topic:format!("room/{}/res/start_queue", y.borrow().master.clone()), 
                                            //     msg: format!(r#"{{"msg":"ban", "time":{}}}"#, (u.borrow().ban.long.clone()-dur).as_secs())};
                                            ban = true;
                                        }
                                    }
                                    if hasRoom {
                                        let r = TotalRoom.get(&rid);
                                        if let Some(y) = r {
                                            if (x.mode == "ng1p2t" || x.mode == "rk1p2t") && y.borrow_mut().users.len() > 1 {
                                                    mqttmsg = MqttMsg{topic:format!("room/{}/res/start_queue", y.borrow().master.clone()), 
                                                    msg: format!(r#"{{"msg":"fail"}}"#)};
                                            }
                                            else {
                                                let mut users: Vec<String> = vec![];
                                                let mut heros: Vec<String> = vec![];
                                                let mut block: Vec<String> = vec![];
                                                let mut blacklist: Vec<String> = vec![];
                                                for u in &y.borrow().users {                  
                                                    users.push(u.borrow().id.clone());
                                                    heros.push(u.borrow().hero.clone());
                                                    for index in &u.borrow().recent_users {
                                                        block = [block.as_slice(), index.clone().as_slice()].concat();
                                                    }
                                                    blacklist = [blacklist.as_slice(), u.borrow().blacklist.as_slice()].concat();
                                                    let n = Instant::now();
                                                    let dur = n.duration_since(u.borrow().ban.from);
                                                    if dur < u.borrow().ban.long {
                                                        println!("ban: {}", (u.borrow().ban.long.clone()-dur).as_secs());
                                                        ban_t = (u.borrow().ban.long.clone()-dur).as_secs();
                                                        
                                                        ban = true;
                                                        break;
                                                    }
                                                }

                                                if ban {
                                                    mqttmsg = MqttMsg{topic:format!("room/{}/res/start_queue", y.borrow().master.clone()), 
                                                            msg: format!(r#"{{"msg":"ban", "time":{}}}"#, ban_t)};
                                                }
                                                else {
                                                    y.borrow_mut().mode = x.mode.clone();
                                                    y.borrow_mut().ready = 1;
                                                    y.borrow_mut().update_avg();
                                                    let mut h: bool;
                                                    println!("STARTQUEUE room avg honor: {}",y.borrow().avg_honor.clone());
                                                    if y.borrow().avg_honor.clone() < honor_threshold {
                                                        h = false;
                                                    } else {
                                                        h = true;
                                                    }
                                                    let mut data = QueueRoomData {
                                                        user_name: users.clone(),
                                                        hero: heros.clone(),
                                                        rid: y.borrow().rid.clone(),
                                                        gid: 0,
                                                        user_len: y.borrow().users.len().clone() as i16,
                                                        avg_ng1v1: y.borrow().avg_ng1v1.clone(),
                                                        avg_rk1v1: y.borrow().avg_rk1v1.clone(),
                                                        avg_ng5v5: y.borrow().avg_ng5v5.clone(),
                                                        avg_rk5v5: y.borrow().avg_rk5v5.clone(),
                                                        honor: h,
                                                        allow_same_hero: true,
                                                        mode: x.mode.clone(),
                                                        ready: 0,
                                                        queue_cnt: 1,
                                                        block: block.clone(),
                                                        blacklist: blacklist.clone(),
                                                    };
                                                    if data.mode == "rk1p2t" || data.mode == "rk5p2t" {
                                                        data.allow_same_hero = true;
                                                    }
                                                    //println!("Totalroom rid: {}", rid);
                                                    let t1 = QueueSender.get(&x.mode.clone());
                                                    if let Some(t1) = t1 {
                                                        t1.send(QueueData::UpdateRoom(data));
                                                    }
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
                                            g.borrow_mut().user_cancel(&x.id);
                                            for r in &g.borrow().rooms {
                                                
                                                r.borrow_mut().ready = 0;
                                            }
                                        }
                                        
                                        let gr = ReadyGroups.remove(&u.borrow().gid);
                                        let r = TotalRoom.get(&u.borrow().rid);
                                        //println!("Totalroom rid: {}", &u.borrow().rid);
                                        if let Some(r) = r {
                                            println!("room master: {}", r.borrow().master);
                                            r.borrow_mut().ready = 0;
                                            let t1 = QueueSender.get(&r.borrow().mode);
                                            //println!("mode: {}", &r.borrow().mode);
                                            if let Some(t1) = t1 {
                                                t1.send(QueueData::RemoveRoom(RemoveRoomData{rid: u.borrow().rid}));
                                            }
                                        }
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
                                            if u2.borrow().online == true{
                                                // Logout
                                                let mut success1 = false;
                                                let mut is_null = false;
                                                if u2.borrow().game_id == 0 {
                                                    let gid = u2.borrow().gid;
                                                    let rid = u2.borrow().rid;
                                                    let r = TotalRoom.get(&u2.borrow().rid);
                                                    
                                                    if let Some(r) = r {
                                                        let m = r.borrow().master.clone();
                                                        r.borrow_mut().rm_user(&x.u.id);
                                                        if r.borrow().users.len() > 0 {
                                                            r.borrow().publish_update(&msgtx, m)?;
                                                        }
                                                        else {
                                                            is_null = true;
                                                        }
                                                        //mqttmsg = MqttMsg{topic:format!("room/{}/res/leave", x.id), 
                                                        //    msg: format!(r#"{{"msg":"ok"}}"#)};
                                                        if !isBackup || (isBackup && isServerLive == false) {
                                                            msgtx.try_send(MqttMsg{topic:format!("room/{}/res/leave", x.u.id), 
                                                                msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                                            LossSend.push(MqttMsg{topic:format!("room/{}/res/leave", x.u.id), 
                                                                msg: format!(r#"{{"msg":"ok"}}"#)});
                                                        }
                                                    }
                                                    
                                                    if gid != 0 {
                                                        let g = ReadyGroups.get(&gid);
                                                        if let Some(gr) = g {
                                                            gr.borrow_mut().user_cancel(&x.u.id);
                                                            ReadyGroups.remove(&gid);
                                                            let r = TotalRoom.get(&rid);
                                                            //println!("Totalroom rid: {}", &u.borrow().rid);
                                                            
                                                            if let Some(r) = r {
                                                                let t1 = QueueSender.get(&r.borrow().mode);
                                                                if let Some(t1) = t1 {
                                                                    t1.send(QueueData::RemoveRoom(RemoveRoomData{rid: rid}));
                                                                }
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
                                                    success1 = true;
                                                } else {
                                                    success1 = true;
                                                }
                                                if is_null {
                                                    TotalRoom.remove(&u2.borrow().rid);
                                                    //QueueRoom.remove(&u.borrow().rid);
                                                }
                                                if success1 {
                                                    msgtx.try_send(MqttMsg{topic:format!("member/{}/res/relogin", x.u.id.clone()), 
                                                        msg: format!(r#"{{"msg":"relogin"}}"#)})?;
                                                    //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/logout", x.id.clone()), 
                                                    //    msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                                } else {
                                                    msgtx.try_send(MqttMsg{topic:format!("member/{}/res/relogin", x.u.id.clone()), 
                                                        msg: format!(r#"{{"msg":"relogin fail"}}"#)})?;
                                                    //msgtx.try_send(MqttMsg{topic:format!("member/{}/res/logout", x.id.clone()), 
                                                    //    msg: format!(r#"{{"msg":"fail"}}"#)})?;
                                                }

                                                // re-Login
                                                msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", u2.borrow().id.clone()), 
                                                        msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                            } else {
                                            u2.borrow_mut().online = true;
                                                if !isBackup || (isBackup && isServerLive == false) {
                                                    msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", u2.borrow().id.clone()), 
                                                        msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                                }
                                                mqttmsg = MqttMsg{topic:format!("member/{}/res/score", u2.borrow().id.clone()), 
                                                    msg: format!(r#"{{"ng1p2t":{}, "ng5p2t":{}, "rk1p2t":{}, "rk5p2t":{}}}"#, 
                                                u2.borrow().ng1v1.score.clone(), u2.borrow().ng5v5.score.clone(), u2.borrow().rk1v1.score.clone(), u2.borrow().rk1v1.score.clone())};
                                            }
                                        }
                                        
                                    }
                                    else {
                                        let mut user = x.u.clone();
                                        user.info.PlayerExp = 0;
                                        user.info.PlayerLv = 1;
                                        for h in hero.clone() {
                                            let hero = Hero {
                                                Hero_name: h.clone(),
                                                Level: 1,
                                                Exp: 0,
                                            };
                                            user.info.HeroExp.insert(h.clone(), hero);
                                        }
                                        // let equ = UserEquInfo {
                                        //     equ_id: 1,
                                        //     rank: 2,
                                        //     lv: 3,
                                        //     lv5: 2,
                                        //     option1lv: 0,
                                        //     option2lv: 0,
                                        //     option3lv: 0,
                                        //     ..Default::default()
                                        // };
                                        // user.info.TotalEquip.push(equ);
                                        TotalUsers.insert(x.u.id.clone(), Rc::new(RefCell::new(user.clone())));
                                        //thread::sleep(Duration::from_millis(50));
                                        sender.send(SqlData::Login(SqlLoginData {id: x.dataid.clone(), name: name.clone()}));
                                        if !isBackup || (isBackup && isServerLive == false){
                                            msgtx.try_send(MqttMsg{topic:format!("member/{}/res/login", x.u.id.clone()), 
                                                msg: format!(r#"{{"msg":"ok"}}"#)})?;
                                        }
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/score", x.u.id.clone()), 
                                                msg: format!(r#"{{"ng1p2t":{}, "ng5p2t":{}, "rk1p2t":{}, "rk5p2t":{}}}"#, 
                                                x.u.ng1v1.score, x.u.ng5v5.score, x.u.rk1v1.score, x.u.rk5v5.score)};
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
                                                    
                                                    if let Some(r) = r {
                                                        let t1 = QueueSender.get(&r.borrow().mode);
                                                        if let Some(t1) = t1 {
                                                            t1.send(QueueData::RemoveRoom(RemoveRoomData{rid: rid}));
                                                        }
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
                                            //println!("Totalroom rid: {}", &u.borrow().rid);
                                            QueueSender.send(QueueData::RemoveRoom(RemoveRoomData{rid: u.borrow().rid}));
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
                                        
                                        
                                        let mut u = TotalUsers.get(&x.id);
                                        if let Some(u) = u {
                                            let mut new_room = RoomData {
                                                rid: room_id,
                                                users: vec![],
                                                master: x.id.clone(),
                                                last_master: "".to_owned(),
                                                mode: "".to_owned(),
                                                avg_ng1v1: 0,
                                                avg_rk1v1: 0,
                                                avg_ng5v5: 0,
                                                avg_rk5v5: 0,
                                                avg_honor: u.borrow().honor,
                                                ready: 0,
                                                queue_cnt: 1,
                                            };
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
                                                if y.borrow().mode != "" {
                                                    let t1 = QueueSender.get(&y.borrow().mode);
                                                    if let Some(t1) = t1 {
                                                        t1.send(QueueData::RemoveRoom(RemoveRoomData{rid: get_rid_by_id(&x.id, &TotalUsers)}));
                                                    }
                                                }
                                                //QueueSender.send(QueueData::RemoveRoom(RemoveRoomData{rid: get_rid_by_id(&x.id, &TotalUsers)}));
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
                                RoomEventData::RankBan(x) => {
                                    let group = PreStartGroups.get(&x.gameid);
                                    if let Some(group) = group {
                                        if group.borrow().ban.contains(&x.ban) {
                                            if group.borrow().ban.len() == 0 {
                                                let mut t1 = group.borrow().teams[0].borrow_mut().get_group_order(group.borrow().mode.clone()).clone();
                                                mqttmsg = MqttMsg{topic:format!("game/{}/res/ban", x.gameid.clone()), 
                                                    msg: format!(r#"{{"msg":"fail", "member":"{}"}}"#, t1[0].clone())};
                                            } else {
                                                let mut t2 = group.borrow().teams[1].borrow_mut().get_group_order(group.borrow().mode.clone()).clone();
                                                mqttmsg = MqttMsg{topic:format!("game/{}/res/ban", x.gameid.clone()), 
                                                    msg: format!(r#"{{"msg":"fail", "member":"{}"}}"#, t2[0].clone())};
                                            }
                                        }
                                        else {
                                            if group.borrow().ban.len() == 0 {
                                                let mut t1 = group.borrow().teams[0].borrow_mut().get_group_order(group.borrow().mode.clone()).clone();
                                                mqttmsg = MqttMsg{topic:format!("game/{}/res/ban", x.gameid.clone()), 
                                                    msg: format!(r#"{{"msg":"ok", "member":"{}"}}"#, t1[0].clone())};
                                            } else {
                                                let mut t2 = group.borrow().teams[1].borrow_mut().get_group_order(group.borrow().mode.clone()).clone();
                                                mqttmsg = MqttMsg{topic:format!("game/{}/res/ban", x.gameid.clone()), 
                                                    msg: format!(r#"{{"msg":"ok", "member":"{}"}}"#, t2[0].clone())};
                                            }
                                            group.borrow_mut().ban.push(x.ban);
                                            // group.borrow_mut().order += 1;
                                            group.borrow_mut().send = false;
                                            group.borrow_mut().times = 0;
                                            mqttmsg = MqttMsg{topic:format!("game/{}/res/update_char", x.gameid.clone()), 
                                                msg: format!(r#"{{"ban":{:?}, "team1":{:?}, "team2":{:?}}}"#, group.borrow().ban.clone(), group.borrow().team1.clone(), group.borrow().team2.clone())};
                                        }
                                    }
                                },
                                RoomEventData::RankChoose(x) => {
                                    let group = PreStartGroups.get(&x.gameid);
                                    if let Some(group) = group {
                                        if group.borrow().ban.contains(&x.choose) || group.borrow().team1.contains(&x.choose) || group.borrow().team2.contains(&x.choose) {
                                            println!("Choose Fail.");
                                            mqttmsg = MqttMsg{topic:format!("game/{}/res/choose", x.gameid.clone()), 
                                                msg: format!(r#"{{"msg":"fail", "member":"{}"}}"#, x.member.clone())};
                                        }
                                        else {
                                            let u = TotalUsers.get(&x.member);
                                            if let Some(u) = u {
                                                u.borrow_mut().hero = x.choose.clone();
                                            }
                                            if group.borrow().order == 0 {
                                                group.borrow_mut().team1.push(x.choose);
                                            }
                                            else {
                                                let od = group.borrow().order.clone()-1;
                                                if (od/2)%2 == 1 {
                                                    group.borrow_mut().team1.push(x.choose);
                                                } else {
                                                    group.borrow_mut().team2.push(x.choose);
                                                }
                                            }
                                            // group.borrow_mut().ban.push(x.ban);
                                            group.borrow_mut().order += 1;

                                            if group.borrow().order.clone() >= group.borrow().user_names.len() as u8 {
                                                group.borrow_mut().done = true;
                                            }
                                            group.borrow_mut().send = false;
                                            group.borrow_mut().times = 0;
                                            mqttmsg = MqttMsg{topic:format!("game/{}/res/choose", x.gameid.clone()), 
                                                msg: format!(r#"{{"msg":"ok", "member":"{}"}}"#, x.member.clone())};
                                            mqttmsg = MqttMsg{topic:format!("game/{}/res/update_char", x.gameid.clone()), 
                                            msg: format!(r#"{{"ban":{:?}, "team1":{:?}, "team2":{:?}}}"#, group.borrow().ban.clone(), group.borrow().team1.clone(), group.borrow().team2.clone())};
                                        }
                                    }
                                },
                                RoomEventData::RankGameStatus(x) => {
                                    if x.msg == "timeout" {
                                        let group = PreStartGroups.remove(&x.gameid);
                                        if let Some(group) = group {
                                            let mut t1 = group.borrow().teams[0].borrow_mut().get_group_order(group.borrow().mode.clone()).clone();
                                            let mut t2 = group.borrow().teams[1].borrow_mut().get_group_order(group.borrow().mode.clone()).clone();
                                    
                                            msgtx.try_send(MqttMsg{topic:format!("game/{}/res/rankgame_status", group.borrow().game_id.clone()), 
                                                msg: format!(r#"{{"msg":"timeout"}}"#)})?;
                                            for r in &group.borrow().room_names {
                                                
                                                msgtx.try_send(MqttMsg{topic:format!("room/{}/res/prestart", r), 
                                                    msg: format!(r#"{{"msg":"stop queue"}}"#)})?;
                                                
                                            }

                                            if group.borrow().order.clone() == 0 {
                                                let u = TotalUsers.get(&t1[0]);
                                                if let Some(u) = u {
                                                    let n = Instant::now();
                                                    let dur = n - u.borrow().ban.from;
                                                    if dur > u.borrow().ban.long {
                                                        u.borrow_mut().ban.from = Instant::now();
                                                        u.borrow_mut().ban.long = Duration::new(60*5, 0);
                                                    } else {
                                                        u.borrow_mut().ban.long += Duration::new(60*5, 0);
                                                    }
                                                }
                                            } else if group.borrow().order.clone() < group.borrow().user_names.len() as u8{
                                                let od = group.borrow().order.clone()-1;
                                                if (od/2)%2 == 1 {
                                                    //Team 1 choose
                                                    let u = TotalUsers.get(&t1[((od/2)+(od%2))as usize]);
                                                    if let Some(u) = u {
                                                        let n = Instant::now();
                                                        let dur = n - u.borrow().ban.from;
                                                        if dur > u.borrow().ban.long {
                                                            u.borrow_mut().ban.from = Instant::now();
                                                            u.borrow_mut().ban.long = Duration::new(60*5, 0);
                                                        } else {
                                                            u.borrow_mut().ban.long += Duration::new(60*5, 0);
                                                        }
                                                    }
                                                    
                                                } else if (od/2)%2 == 0 {
                                                    // Team 2 choose
                                                    let u = TotalUsers.get(&t2[((od/2)+(od%2)) as usize]);
                                                    if let Some(u) = u {
                                                        let n = Instant::now();
                                                        let dur = n - u.borrow().ban.from;
                                                        if dur > u.borrow().ban.long {
                                                            u.borrow_mut().ban.from = Instant::now();
                                                            u.borrow_mut().ban.long = Duration::new(60*5, 0);
                                                        } else {
                                                            u.borrow_mut().ban.long += Duration::new(60*5, 0);
                                                        }
                                                    }
                                                }
                                            }
                                            for uid in &group.borrow().user_names {
                                                let u = TotalUsers.get(uid);
                                                if let Some(u) = u {
                                                    u.borrow_mut().gid = 0;
                                                }
                                                
                                                
                                            }
                                        }
                                    }
                                },
                                RoomEventData::GetRP(x) => {
                                    println!("Get Replay");
                                    let replay = TotalReplay.get(&x.game);
                                    if let Some(replay) = replay {
                                        println!("{:?}", replay);
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/replay", x.id.clone()), 
                                        msg: format!(r#"{{"name":"{}", "url":"{}", "server":"{}"}}"#, replay.borrow().name, replay.borrow().url, replay.borrow().address)};
                                    }
                                },
                                RoomEventData::UploadRP(x) => {
                                    //println!("Upload Replay");
                                    let s = format!("{}", Uuid::new_v4());
                                    sender.send(SqlData::UpdateReplay(SqlReplayData {gameid: x.game.clone(), name: x.name.clone(), url: s.clone(), address: "114.32.129.195".to_string() }));
                                    mqttmsg = MqttMsg{topic:format!("game/{}/res/upload", x.game.clone()), 
                                        msg: format!(r#"{{"server":"114.32.129.195", "game":{}, "id":"{}"}}"#, x.game.clone(), s)};
                                    let replay = Replay {
                                        gameid: x.game.clone(),
                                        name: x.name.clone(),
                                        url: s.clone(),
                                        address: "114.32.129.195".to_string(),
                                        count: 1,
                                        ..Default::default()
                                    };
                                    TotalReplay.insert(x.game.clone(), Rc::new(RefCell::new(replay.clone())));
                                },
                                RoomEventData::UploadRes(x) => {
                                    //println!("Upload Res");
                                    if x.msg == "fail" {
                                        let rp = TotalReplay.get(&x.game);
                                        if let Some(rp) = rp {
                                            if rp.borrow().count < 8 {
                                                rp.borrow_mut().address = "167.179.96.153".to_string();
                                                rp.borrow_mut().count += 1;
                                                mqttmsg = MqttMsg{topic:format!("game/{}/res/upload", x.game.clone()), 
                                                    msg: format!(r#"{{"server":"167.179.96.153", "game":{}, "id":"{}"}}"#, x.game.clone(), rp.borrow().url.clone())};
                                                sender.send(SqlData::UpdateExistingRP(SqlReplayData {gameid: rp.borrow().gameid.clone(), name: rp.borrow().name.clone(), url: rp.borrow().url.clone(), address: "167.179.96.153".to_string() }));
                                            }
                                        }
                                    }
                                    
                                },
                                RoomEventData::InsertEqu(x) => {
                                    println!("Insert Equ");
                                    let u2 = TotalUsers.get(&x.userid);
                                    if let Some(u2) = u2 {
                                        println!("{} Add Equip", x.userid);
                                        let info = UserEquInfo {
                                            equ_id: x.equ_id.clone(),
                                            rank: x.rank.clone(),
                                            lv: x.Lv.clone(),
                                            lv5: x.Lv5.clone(),
                                            option1: x.option1.clone(),
                                            option2: x.option1.clone(),
                                            option3: x.option1.clone(),
                                            option1lv: x.option1Lv.clone(),
                                            option2lv: x.option2Lv.clone(),
                                            option3lv: x.option3Lv.clone(),
                                            ..Default::default()
                                        };
                                        u2.borrow_mut().info.TotalEquip.push(info);
                                    }
                                },
                                RoomEventData::ModifyUserEqu(x) => {
                                    println!("Modify User Equip");
                                    let u2 = TotalUsers.get(&x.userid);
                                    if let Some(u2) = u2 {
                                        println!("{:?}", u2.borrow().info.TotalEquip);
                                        let index = u2.borrow().info.TotalEquip.iter().position(|r| r.equ_id == x.equ_id).unwrap();
                                        println!("index: {}", index);
                                        u2.borrow_mut().info.TotalEquip[index].rank = x.rank.clone();
                                        u2.borrow_mut().info.TotalEquip[index].lv = x.Lv.clone();
                                        u2.borrow_mut().info.TotalEquip[index].lv5 = x.Lv5.clone();
                                        u2.borrow_mut().info.TotalEquip[index].option1 = x.option1.clone();
                                        u2.borrow_mut().info.TotalEquip[index].option2 = x.option2.clone();
                                        u2.borrow_mut().info.TotalEquip[index].option3 = x.option3.clone();
                                        u2.borrow_mut().info.TotalEquip[index].option1lv = x.option1Lv.clone();
                                        u2.borrow_mut().info.TotalEquip[index].option2lv = x.option2Lv.clone();
                                        u2.borrow_mut().info.TotalEquip[index].option3lv = x.option3Lv.clone();
                                    }
                                },
                                RoomEventData::DeleteUserEqu(x) => {
                                    println!("Delete User Equip");
                                    let u2 = TotalUsers.get(&x.userid);
                                    if let Some(u2) = u2 {
                                        println!("{:?}", u2.borrow().info.TotalEquip);
                                        let index = u2.borrow().info.TotalEquip.iter().position(|r| r.equ_id == x.equ_id).unwrap();
                                        println!("index: {}", index);
                                        u2.borrow_mut().info.TotalEquip.remove(index);
                                    }
                                },
                                RoomEventData::ModifyEqu(x) => {
                                    let e = TotalEquip.get(&x.equ_id);
                                    if let Some(e) = e {
                                        e.borrow_mut().equ_name = x.equ_name.clone();
                                        e.borrow_mut().positive = x.positive.clone();
                                        e.borrow_mut().negative = x.negative.clone();
                                        e.borrow_mut().pvalue = vec![];
                                        e.borrow_mut().nvalue = vec![];
                                        e.borrow_mut().option_id = vec![];

                                        e.borrow_mut().pvalue.push(x.PLv1.clone());
                                        e.borrow_mut().pvalue.push(x.PLv2.clone());
                                        e.borrow_mut().pvalue.push(x.PLv3.clone());
                                        e.borrow_mut().pvalue.push(x.PLv4.clone());
                                        e.borrow_mut().pvalue.push(x.PLv5v1.clone());
                                        e.borrow_mut().pvalue.push(x.PLv5v2.clone());
                                        e.borrow_mut().pvalue.push(x.PLv5v3.clone());
                                        e.borrow_mut().pvalue.push(x.PLv5v4.clone());
                                        e.borrow_mut().pvalue.push(x.PLv5v5.clone());

                                        e.borrow_mut().nvalue.push(x.NLv1.clone());
                                        e.borrow_mut().nvalue.push(x.NLv2.clone());
                                        e.borrow_mut().nvalue.push(x.NLv3.clone());
                                        e.borrow_mut().nvalue.push(x.NLv4.clone());
                                        e.borrow_mut().nvalue.push(x.NLv5v1.clone());
                                        e.borrow_mut().nvalue.push(x.NLv5v2.clone());
                                        e.borrow_mut().nvalue.push(x.NLv5v3.clone());
                                        e.borrow_mut().nvalue.push(x.NLv5v4.clone());
                                        e.borrow_mut().nvalue.push(x.NLv5v5.clone());

                                        if x.Option1 != 0 {
                                            e.borrow_mut().option_id.push(x.Option1.clone());
                                        }
                                        if x.Option2 != 0 {
                                            e.borrow_mut().option_id.push(x.Option2.clone());
                                        }
                                        if x.Option3 != 0 {
                                            e.borrow_mut().option_id.push(x.Option3.clone());
                                        }
                                        println!("Modify {:?}", e);
                                    }
                                },
                                RoomEventData::CreateEqu(x) => {
                                    println!("CreateEqu");
                                    let mut equ = Equipment {
                                        equ_id: x.equ_id.clone(),
                                        equ_name: x.equ_name.clone(),
                                        positive: x.positive.clone(),
                                        negative: x.negative.clone(),
                                        ..Default::default()
                                    };
                                    equ.pvalue.push(x.PLv1.clone());
                                    equ.pvalue.push(x.PLv2.clone());
                                    equ.pvalue.push(x.PLv3.clone());
                                    equ.pvalue.push(x.PLv4.clone());
                                    equ.pvalue.push(x.PLv5v1.clone());
                                    equ.pvalue.push(x.PLv5v2.clone());
                                    equ.pvalue.push(x.PLv5v3.clone());
                                    equ.pvalue.push(x.PLv5v4.clone());
                                    equ.pvalue.push(x.PLv5v5.clone());

                                    equ.nvalue.push(x.NLv1.clone());
                                    equ.nvalue.push(x.NLv2.clone());
                                    equ.nvalue.push(x.NLv3.clone());
                                    equ.nvalue.push(x.NLv4.clone());
                                    equ.nvalue.push(x.NLv5v1.clone());
                                    equ.nvalue.push(x.NLv5v2.clone());
                                    equ.nvalue.push(x.NLv5v3.clone());
                                    equ.nvalue.push(x.NLv5v4.clone());
                                    equ.nvalue.push(x.NLv5v5.clone());

                                    if x.Option1 != 0 {
                                        equ.option_id.push(x.Option1.clone());
                                    }
                                    if x.Option2 != 0 {
                                        equ.option_id.push(x.Option2.clone());
                                    }
                                    if x.Option3 != 0 {
                                        equ.option_id.push(x.Option3.clone());
                                    }
                                    println!("Create {:?}", equ);
                                    TotalEquip.insert(equ.equ_id, Rc::new(RefCell::new(equ.clone())));
                                },
                                RoomEventData::DeleteEqu(x) => {
                                    //println!("Delete");
                                    TotalEquip.remove(&x.equ_id);
                                },
                                RoomEventData::NewOption(x) => {
                                    println!("New Option");
                                    //TotalEquip.remove(&x.equ_id);
                                    let mut opt = EquOption {
                                        option_id: x.option_id.clone(),
                                        option_name: x.option_name.clone(),
                                        special: x.special.clone(),
                                        exception: x.exception.clone(),
                                        set_id: x.set_id.clone(),
                                        set_amount: x.set_amount.clone(),
                                        option_weight: x.option_weight.clone(),
                                        ..Default::default()
                                    };
                                    opt.effect.push(x.effect_v1);
                                    opt.effect.push(x.effect_v2);
                                    opt.effect.push(x.effect_v3);
                                    opt.effect.push(x.effect_v4);
                                    opt.effect.push(x.effect_v5);
                                    TotalEquOption.insert(opt.option_id, Rc::new(RefCell::new(opt.clone())));
                                    println!("{:?}", TotalEquOption);
                                },
                                RoomEventData::ModifyOption(x) => {
                                    println!("Modify Option");
                                    let opt = TotalEquOption.get(&x.option_id);
                                    if let Some(opt) = opt {
                                        opt.borrow_mut().option_name = x.option_name.clone();
                                        opt.borrow_mut().special = x.special.clone();
                                        opt.borrow_mut().exception = x.exception.clone();
                                        opt.borrow_mut().set_id = x.set_id.clone();
                                        opt.borrow_mut().set_amount = x.set_amount.clone();
                                        opt.borrow_mut().option_weight = x.option_weight.clone();
                                        
                                        opt.borrow_mut().effect.clear();
                                        opt.borrow_mut().effect.push(x.effect_v1);
                                        opt.borrow_mut().effect.push(x.effect_v2);
                                        opt.borrow_mut().effect.push(x.effect_v3);
                                        opt.borrow_mut().effect.push(x.effect_v4);
                                        opt.borrow_mut().effect.push(x.effect_v5);

                                    }
                                    println!("{:?}", TotalEquOption);
                                },
                                RoomEventData::DeleteOption(x) => {
                                    println!("Delete Option");
                                    TotalEquOption.remove(&x.option_id);
                                    println!("{:?}", TotalEquOption);
                                },
                                RoomEventData::AddBlackList(x) => {
                                    //println!("Add Black List");
                                    let mut success = false;
                                    let u = TotalUsers.get(&x.id);
                                    if let Some(u) = u {
                                        if TotalUsers.contains_key(&x.black) {
                                            if !u.borrow_mut().blacklist.contains(&x.black) {
                                                u.borrow_mut().blacklist.push(x.black.clone());
                                                success = true;
                                                sender.try_send(SqlData::AddBlack(x.clone()));
    
                                            }
                                            
                                        }
                                    }
                                    if success {
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/add_black_list", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"ok"}}"#)};
                                    } else {
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/add_black_list", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"fail"}}"#)};
                                    }
                                },
                                RoomEventData::QueryBlackList(x) => {
                                    //println!("Query Black List");
                                    let u = TotalUsers.get(&x.id);
                                    if let Some(u) = u {
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/query_black_list", x.id.clone()), 
                                            msg: format!(r#"{{"list":{:?}}}"#, u.borrow().blacklist.clone())};
                                    }
                                },
                                RoomEventData::RemoveBlackList(x) => {
                                    //println!("Remove Black List");
                                    let mut success = false;
                                    let u = TotalUsers.get(&x.id);
                                    if let Some(u) = u {
                                        if u.borrow_mut().blacklist.contains(&x.black) {
                                            let index = u.borrow().blacklist.iter().position(|a| *a == x.black).unwrap();
                                            u.borrow_mut().blacklist.remove(index);
                                            success = true;
                                            sender.try_send(SqlData::RemoveBlack(x.clone()));
                                        }
                                    }
                                    if success {
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/add_black_list", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"ok"}}"#)};
                                    } else {
                                        mqttmsg = MqttMsg{topic:format!("member/{}/res/add_black_list", x.id.clone()), 
                                            msg: format!(r#"{{"msg":"fail"}}"#)};
                                    }
                                },
                                RoomEventData::TestEquip(x) => {
                                    println!("Equip Data");
                                    let mut eff = Effect {..Default::default()};
                                    let mut EffMap: HashMap<String, Vec<f32>> = HashMap::new(); 
                                    for e in x.Test {
                                        let equ = TotalEquip.get(&e.equ_id.clone());
                                        if let Some(equ) = equ {
                                            let mut e1 = Equit {
                                                name: equ.borrow().equ_name.clone(),
                                                positive: equ.borrow().positive.clone(),
                                                pvalue: (equ.borrow().pvalue[(e.Lv-1) as usize].clone() * 1000.0).round()/ 1000 as f32,
                                                negative: equ.borrow().negative.clone(),
                                                nvalue: (equ.borrow().nvalue[(e.Lv-1) as usize].clone() * 1000.0).round()/ 1000 as f32,
                                                ..Default::default()
                                            };
                                            let mut total_weight = 0.0;
                                            if e.rank == 1 {
                                                total_weight = 1.0;
                                            } else if e.rank == 2 {
                                                total_weight = 1.5;
                                            } else if e.rank == 3 {
                                                total_weight = 2.0;
                                            } else if e.rank == 4 {
                                                total_weight = 2.5;
                                            } else if e.rank == 5 {
                                                total_weight = 3.0;
                                            }
                                            for i in equ.borrow().option_id.clone() {
                                                let opt = TotalEquOption.get(&i);
                                                if let Some(opt) = opt {
                                                    let mut eo = EquOpt {
                                                        name: opt.borrow().option_name.clone(),
                                                        weight: (opt.borrow().option_weight.clone() * 1000.0).round() / 1000 as f32,
                                                        value: (opt.borrow().effect[(e.Lv5-1) as usize].clone() * 1000.0).round() / 1000 as f32,
                                                    };
                                                    total_weight += eo.weight;
                                                    if eo.name == "lifesteal" {
                                                        eff.lifesteal.push((1000.0 * eo.value).round()  / 1000 as f32 );
                                                    } else if eo.name == "maxhp" {
                                                        eff.maxhp.push((1000.0 * eo.value).round() / 1000 as f32);
                                                    } else if eo.name == "maxmp" {
                                                        eff.maxmp.push((1000.0 * eo.value).round() / 1000 as f32);
                                                    }
                                                    if !EffMap.contains_key(&eo.name) {
                                                        EffMap.insert(eo.name.clone(), vec![]);
                                                        
                                                    }
                                                    (*EffMap.get_mut(&eo.name).unwrap()).push((1000.0 * eo.value).round()  / 1000 as f32 );
                                                    // if let Some(eff1) = h.effect.clone().into_iter().find(|s| s.name == eo.name) {
                                                    //     ;
                                                    // } else {
                                                    //     let mut eff = Effect {
                                                    //         name: eo.name.clone(),
                                                    //         v: eo.value.clone(),
                                                    //     };
                                                    //     h.effect.push(eff);
                                                    // }
                                                    e1.option.push(eo);
                                                }
                                            }
                                            e1.total_weight = (total_weight * 1000.0).round()/1000 as f32; 
                                            // positive
                                            if e1.positive == "lifesteal" {
                                                eff.lifesteal.push((1000.0*e1.pvalue*e1.total_weight).round() / 1000 as f32);
                                            } else if e1.positive == "maxhp" {
                                                eff.maxhp.push((1000.0*e1.pvalue*e1.total_weight).round() / 1000 as f32);
                                            } else if e1.positive == "maxmp" {
                                                eff.maxmp.push((1000.0*e1.pvalue*e1.total_weight).round() / 1000 as f32);
                                            }
                                            if !EffMap.contains_key(&e1.positive) {
                                                EffMap.insert(e1.positive.clone(), vec![]);
                                                
                                            }
                                            (*EffMap.get_mut(&e1.positive).unwrap()).push((1000.0*e1.pvalue*e1.total_weight).round() / 1000 as f32);
                                            //h.effect.push(eff);
                                            // negative
                                            if e1.negative == "lifesteal" {
                                                eff.lifesteal.push((1000.0*e1.nvalue*e1.total_weight).round() / 1000 as f32);
                                            } else if e1.negative == "maxhp" {
                                                eff.maxhp.push((1000.0*e1.nvalue*e1.total_weight).round() / 1000 as f32);
                                            } else if e1.negative == "maxmp" {
                                                eff.maxmp.push((1000.0*e1.nvalue*e1.total_weight).round() / 1000 as f32);
                                            }
                                            
                                            if !EffMap.contains_key(&e1.negative) {
                                                EffMap.insert(e1.negative.clone(), vec![]);
                                                
                                            }
                                            (*EffMap.get_mut(&e1.negative).unwrap()).push((1000.0*e1.nvalue*e1.total_weight).round() / 1000 as f32 );
                                            
                                        }
                                    }
                                    println!("{:?}", EffMap);
                                    mqttmsg = MqttMsg{topic:format!("manager/0/res/equ_test"), 
                                            msg: format!(r#"{:?}"#, EffMap)};
                                },
                                RoomEventData::GameServerLogin(x) => {
                                    let gameserver = GameServer {
                                        name: x.name.clone(),
                                        address: x.address.clone(),
                                        max_server: x.max_server.clone(),
                                        now_server: 0,
                                        max_user: x.max_user.clone(),
                                        now_user: 0,
                                        utilization: 0,
                                    };
                                    TotalGameServer.push(Rc::new(RefCell::new(gameserver)));
                                    mqttmsg = MqttMsg{topic:format!("server/{}/res/login", x.name.clone()), 
                                            msg: format!(r#"{{"msg":"ok"}}"#)};
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

    Ok(tx)
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
    sender.try_send(RoomEventData::StartQueue(StartQueueData{id: data.id.clone(), action: data.action.clone(), mode: data.mode.clone()}));
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

pub fn ban(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: RankBanData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::RankBan(data));
    Ok(())
}

pub fn choose(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: RankChooseData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::RankChoose(data));
    Ok(())
}

pub fn rankgame_status(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: Msg = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::RankGameStatus(RankGameStatusData{gameid: id.clone().parse::<u32>().unwrap(), msg: data.msg}));
    Ok(())
}

pub fn getRP(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: GameCloseData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::GetRP(GetRPData{ id: id.clone(), game: data.game}));
    Ok(())
}

pub fn uploadRP(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: UploadData = serde_json::from_value(v)?;
    //sender.try_send(RoomEventData::UploadRP(data));
    Ok(())
}

pub fn uploadRPRes(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: UploadRes = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::UploadRes(data));
    Ok(())
}

pub fn insert_equ(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: EquInfo = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::InsertEqu(data));
    Ok(())
}

pub fn modify_userequ(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: EquInfo = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::ModifyUserEqu(data));
    Ok(())
}

pub fn delete_userequ(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: UserDeleteEquip = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::DeleteUserEqu(data));
    Ok(())
}

pub fn modify_equ(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: NewEquip = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::ModifyEqu(data));
    Ok(())
}

pub fn create_equ(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: NewEquip = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::CreateEqu(data));
    Ok(())
}

pub fn delete_equ(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: DeleteEquip = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::DeleteEqu(data));
    Ok(())
}

pub fn new_option(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: NewOption = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::NewOption(data));
    Ok(())
}

pub fn modify_option(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: NewOption = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::ModifyOption(data));
    Ok(())
}

pub fn delete_option(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: DeleteOption = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::DeleteOption(data));
    Ok(())
}

pub fn add_black_list(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: Black = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::AddBlackList(data));
    Ok(())
}

pub fn query_black_list(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: QueryBlack = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::QueryBlackList(data));
    Ok(())
}

pub fn remove_black_list(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: Black = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::RemoveBlackList(data));
    Ok(())
}

pub fn equ_test(id: String, v: Value, sender: Sender<RoomEventData>)
-> std::result::Result<(), Error>
{
   let data: TestData = serde_json::from_value(v)?;
   sender.try_send(RoomEventData::TestEquip(data));
   Ok(())
}

pub fn server_login(id: String, v: Value, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    let data: GameServerLoginData = serde_json::from_value(v)?;
    sender.try_send(RoomEventData::GameServerLogin(data));
    Ok(())
}

pub fn server_dead(id: String, sender: Sender<RoomEventData>)
 -> std::result::Result<(), Error>
{
    sender.try_send(RoomEventData::MainServerDead(DeadData{ServerDead: id}));
    Ok(())
}