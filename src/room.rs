use serde_derive::{Serialize, Deserialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashMap, BTreeMap};
use std::time::{Duration, Instant};
use crate::msg::*;
use crossbeam_channel::{bounded, tick, Sender, Receiver, select};
use failure::Error;
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GameServer {
    pub name: String,
    pub address: String,
    pub max_user: u32,
    pub now_user: u32,
    pub max_server: u32,
    pub now_server: u32,
    pub utilization: i16,
}

impl GameServer{
    pub fn update(&mut self) {
        self.utilization = 100*self.now_server as i16/self.max_server as i16;
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Replay {
    pub gameid: u32,
    pub name: String,
    pub url: String,
    pub address: String,
    pub count: u8,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Equipment {
    pub equ_id: u32,
    pub equ_name: String,
    pub positive: String,
    pub negative: String,
    pub pvalue: Vec<f32>,
    pub nvalue: Vec<f32>,
    pub option_id: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct EquOption {
    pub option_id: u32,
    pub option_name: String,
    pub special: bool,
    pub exception: bool,
    pub set_id: u32,
    pub set_amount: u8,
    pub option_weight: f32,
    pub effect: Vec<f32>, 
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UserEquInfo {
    pub equ_id: u32,
    pub rank: u8,
    pub lv: u8,
    pub lv5: u8,
    pub option1: u32,
    pub option2: u32,
    pub option3: u32,
    pub option1lv: u8,
    pub option2lv: u8,
    pub option3lv: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BanTime {
    pub from: Instant,
    pub long: Duration,
}

impl Default for BanTime {
    fn default() -> BanTime{
        BanTime{
            from: Instant::now(),
            long: Duration::new(0, 0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct User {
    pub id: String,
    pub name: String,
    pub hero: String,
    pub honor: i32,
    pub ban: BanTime,
    pub info: PlayerInfo,
    pub ng1v1: ScoreInfo,
    pub ng5v5: ScoreInfo,
    pub rk1v1: ScoreInfo,
    pub rk5v5: ScoreInfo,
    pub rid: u32,
    pub gid: u32,
    pub game_id: u32,
    pub online: bool,
    pub start_prestart: bool,
    pub prestart_get: bool,
    pub recent_users: Vec<Vec<String>>,
    pub blacklist: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PlayerInfo {
    pub PlayerExp: u32,
    pub PlayerLv: u32,
    pub HeroExp: BTreeMap<String, Hero>,
    pub Money: u32,
    pub TotalCurrency: u32,
    pub TotalEquip: Vec<UserEquInfo>,
    pub HiddenRank: u32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ScoreInfo {
    pub score: i16,
    pub WinCount: u32,
    pub LoseCount: u32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Hero {
    pub Hero_name: String,
    pub Level: u32,
    pub Exp: u32,
}

#[derive(Clone, Debug)]
pub struct RoomData {
    pub rid: u32,
    pub users: Vec<Rc<RefCell<User>>>,
    pub master: String,
    pub last_master: String,
    pub mode: String,
    pub avg_ng1v1: i16,
    pub avg_rk1v1: i16,
    pub avg_ng5v5: i16,
    pub avg_rk5v5: i16,
    pub avg_honor: i32,
    pub ready: i8,
    pub queue_cnt: i16,
}

impl RoomData {
    pub fn update_avg(&mut self) {
        let mut sum_ng1v1 = 0;
        let mut sum_rk1v1 = 0;
        let mut sum_ng5v5 = 0;
        let mut sum_rk5v5 = 0;
        let mut sum_honor = 0;
        
        for user in &self.users {
            sum_ng1v1 += user.borrow().ng1v1.score;
            sum_rk1v1 += user.borrow().rk1v1.score;
            sum_ng5v5 += user.borrow().ng5v5.score;
            sum_rk5v5 += user.borrow().rk5v5.score;
            sum_honor += user.borrow().honor;
        }
        if self.users.len() > 0 {
            self.avg_ng1v1 = sum_ng1v1/self.users.len() as i16;
            self.avg_rk1v1 = sum_rk1v1/self.users.len() as i16;
            self.avg_ng5v5 = sum_ng5v5/self.users.len() as i16 + 50 * (self.users.len() as i16 - 1);
            self.avg_rk5v5 = sum_rk5v5/self.users.len() as i16 + 50 * (self.users.len() as i16 - 1);
            self.avg_honor = sum_honor/self.users.len() as i32;
        }
    }

    pub fn add_user(&mut self, user: Rc<RefCell<User>>) {
        user.borrow_mut().rid = self.rid;
        self.users.push(Rc::clone(&user));
        self.update_avg();
    }

    pub fn leave_room(&mut self) {
        for user in &self.users {
            user.borrow_mut().rid = 0;
            user.borrow_mut().gid = 0;
            user.borrow_mut().game_id = 0;
        }
        self.ready = 0;
    }

    pub fn user_prestart(&mut self) {
        for user in &self.users {
            user.borrow_mut().start_prestart = true;
            user.borrow_mut().prestart_get = false;
        }
    }

    pub fn check_prestart_get(&mut self) -> bool{
        let mut res = false;
        for user in &self.users {
            //println!("User: {}, Prestart_get: {}", user.borrow().id, user.borrow().prestart_get);
            if user.borrow().prestart_get == true {
                res = true;
            }
            else {
                res = false;
                break;
            }
        }
        res
    }

    pub fn clear_queue(&mut self) {
        for user in &self.users {
            user.borrow_mut().gid = 0;
            user.borrow_mut().game_id = 0;
        }
        self.ready = 0;
    }

    pub fn publish_update(&self, msgtx: &Sender<MqttMsg>, r: String) -> Result<(), Error>{
        #[derive(Serialize, Deserialize)]
        pub struct teamCell {
            pub room: String,  
            pub team: Vec<String>,
        }
        let mut t = teamCell {room: self.master.clone(), team: vec![]};
        
        for user in &self.users {
            t.team.push(user.borrow().id.clone());
        }
        msgtx.try_send(MqttMsg{topic:format!("room/{}/res/update", r), msg: serde_json::to_string(&t).unwrap()})?;
        Ok(())
    }

    pub fn rm_user(&mut self, id: &String) {
        let mut i = 0;
        while i != self.users.len() {
            let id2 =  self.users[i].borrow().id.clone();
            if id2 == *id {
                self.users.remove(i);
            } else {
                i += 1;
            }
        }
        if self.master == *id && self.users.len() > 0 {
            self.last_master = self.master.clone();
            self.master = self.users[0].borrow().id.clone();
        }
        self.update_avg();
    }
}

#[derive(Clone, Debug, Default)]
pub struct FightCheck {
    pub id: String,
    pub check: i8,
}

#[derive(Clone, Debug, Default)]
pub struct FightGroup {
    pub rooms: Vec<Rc<RefCell<RoomData>>>,
    pub user_order: Vec<Rc<RefCell<User>>>,
    pub user_count: i16,
    pub avg_ng1v1: i16,
    pub avg_rk1v1: i16,
    pub avg_ng5v5: i16,
    pub avg_rk5v5: i16,
    pub avg_honor: i32,
    pub mode: String,
    pub checks: Vec<FightCheck>,
    pub rids: Vec<u32>,
    pub game_status: u16,
    pub queue_cnt: i16,
}

impl FightGroup {
    pub fn user_ready(&mut self, id: &String) -> bool {
        for c in &mut self.checks {
            //println!("c: {}, id: {}", c.id, *id);
            if c.id == *id {
                //println!("check");
                c.check = 1;
                self.check_prestart();
                return true;
            }
        }
        false
    }

    pub fn get_users_id_hero(&self) -> Vec<(String, String, String, Vec<UserEquInfo>)> {
        let mut res: Vec<(String, String, String, Vec<UserEquInfo>)> = vec![];
        for r in &self.rooms {
            for u in &r.borrow().users {
                res.push((u.borrow().id.clone(), u.borrow().name.clone(), u.borrow().hero.clone(), u.borrow().info.TotalEquip.clone()));
            }
        }
        res
    }
    pub fn user_cancel(&mut self, id: &String) -> bool {
        for c in &mut self.checks {
            if c.id == *id {
                c.check = -1;
                return true;
            }
        }
        false
    }
    pub fn update_group_order(&mut self, mode: String) {
        self.user_order.clear();
        for r in &self.rooms {
            for u in &r.borrow().users {
                self.user_order.push(u.clone());
            }
        }
        if mode == "rk1p2t" {
            self.user_order.sort_by_key(|x| x.borrow().rk1v1.score);
        } else if mode == "rk5p2t" {
            self.user_order.sort_by_key(|x| x.borrow().rk5v5.score);
        }
    }

    pub fn get_group_order(&mut self, mode: String) -> Vec<String> {
        self.update_group_order(mode);
        let mut res: Vec<String> = vec![];
        for u in &self.user_order {
            res.push(u.borrow().id.clone());
        }
        res
    }

    pub fn leave_room(&mut self) -> bool {
        for r in &mut self.rooms {
            r.borrow_mut().leave_room();
        }
        false
    }
    pub fn clear_queue(&mut self) -> bool {
        for r in &mut self.rooms {
            r.borrow_mut().clear_queue();
        }
        false
    }

    pub fn check_has_room(&self, id: &String) -> bool {
        for room in &self.rooms {
            if room.borrow().master == *id {
                return true;
            }
        }
        false
    }

    pub fn update_avg(&mut self) {
        let mut sum_ng1v1: i32 = 0;
        let mut sum_rk1v1: i32 = 0;
        let mut sum_ng5v5: i32 = 0;
        let mut sum_rk5v5: i32 = 0;
        let mut sum_honor: i32 = 0;
        
        self.user_count = 0;
        for room in &self.rooms {
            sum_ng1v1 += room.borrow().avg_ng1v1 as i32 * room.borrow().users.len() as i32;
            sum_rk1v1 += room.borrow().avg_rk1v1 as i32 * room.borrow().users.len() as i32;
            sum_ng5v5 += room.borrow().avg_ng5v5 as i32 * room.borrow().users.len() as i32;
            sum_rk5v5 += room.borrow().avg_rk5v5 as i32 * room.borrow().users.len() as i32;
            sum_honor += room.borrow().avg_honor as i32 * room.borrow().users.len() as i32;
            
            self.user_count += room.borrow().users.len() as i16;
        }
        if self.user_count > 0 {
            self.avg_ng1v1 = (sum_ng1v1/self.user_count as i32) as i16;
            self.avg_rk1v1 = (sum_rk1v1/self.user_count as i32) as i16;
            self.avg_ng5v5 = (sum_ng5v5/self.user_count as i32) as i16;
            self.avg_rk5v5 = (sum_rk5v5/self.user_count as i32) as i16;
            self.avg_honor = (sum_honor/self.user_count as i32);
        }
    }

    pub fn add_room(&mut self, room: Rc<RefCell<RoomData>>) {
        self.rooms.push(Rc::clone(&room));
        self.rids.push(room.borrow().rid);
        self.update_avg();
    }

    pub fn rm_room_by_master(&mut self, id: &String) {
        let pos = self.rooms.iter().position(|x| x.borrow().master == *id);
        if let Some(x) = pos {
            let rid = self.rooms[x].borrow().rid;
            self.rids.retain(|&x| x != rid);
            self.rooms.remove(x);
        }
        self.update_avg();
    }

    pub fn rm_room_by_rid(&mut self, id: u32) {
        let mut i = 0;
        self.rids.retain(|&x| x != id);
        self.rooms.retain(|x| x.borrow().rid != id);
        self.update_avg();
    }

    pub fn ready(&mut self) {
        self.checks.clear();
        for room in &self.rooms {
            room.borrow_mut().ready = 3;
        }
    }

    pub fn set_group_id(&mut self, gid: u32) {
        for room in &self.rooms {
            room.borrow_mut().ready = 1;
            for u in &room.borrow_mut().users {
                u.borrow_mut().gid = gid;
            }
        }
    }
    
    pub fn prestart(&mut self) {
        self.checks.clear();
        for room in &self.rooms {
            room.borrow_mut().ready = 1;
            room.borrow_mut().user_prestart();
            for u in &room.borrow().users {
                self.checks.push(FightCheck{id: u.borrow().id.clone(), check: 0});
            }
        }
    }

    pub fn check_prestart(&self) -> PrestartStatus {
        let mut res = PrestartStatus::Ready;
        for c in &self.checks {
            //println!("check: '{:?}'", c);
            if c.check < 0 {
                return PrestartStatus::Cancel;
            } else if c.check != 1 {
                res= PrestartStatus::Wait;
            }
        }
        res
    }
}


#[derive(Clone, Debug, Default)]
pub struct FightGame {
    pub teams: Vec<Rc<RefCell<FightGroup>>>,
    pub room_names: Vec<String>,
    pub user_names: Vec<String>,
    pub game_id: u32,
    pub mode: String,
    pub times: u8,
    pub order: u8,
    pub send: bool,
    pub ban: Vec<String>,
    pub team1: Vec<String>,
    pub team2: Vec<String>,
    pub done: bool,
    pub user_count: u16,
    pub winteam: Vec<String>,
    pub loseteam: Vec<String>,
    pub game_status: u16,
    pub game_port: u16,
    pub server_name: String,
    pub game_start: bool,
    pub server_notify: i8,
}

#[derive(PartialEq)]
pub enum PrestartStatus {
    Wait,
    Ready,
    Cancel
}

impl FightGame {

    pub fn update_names(&mut self) {
        self.room_names.clear();
        self.user_names.clear();
        for t in &self.teams {
            for r in &t.borrow().rooms {
                self.room_names.push(r.borrow().master.clone());
                for u in &r.borrow().users {
                    self.user_names.push(u.borrow().id.clone());
                }
            }
        }
    }

    pub fn check_prestart_get(&self) -> bool {
        let mut res = false;
        for c in &self.teams {
            for r in &c.borrow().rooms {
                res = r.borrow_mut().check_prestart_get();
                if res == false {
                    break;
                }
            }
            if res == false {
                break;
            }
        }
        res
    }

    pub fn check_prestart(&self) -> PrestartStatus {
        let mut res = PrestartStatus::Ready;
        for c in &self.teams {
            let v = c.borrow().check_prestart();
            if v == PrestartStatus::Cancel {
                return PrestartStatus::Cancel;
            } else if v == PrestartStatus::Wait {
                res = PrestartStatus::Wait;
            }
        }
        res
    }

    pub fn set_game_id(&mut self, gid: u32) {
        for t in &mut self.teams {
            for room in &mut t.borrow_mut().rooms {
                room.borrow_mut().ready = 1;
                for u in &room.borrow_mut().users {
                    u.borrow_mut().game_id = gid;
                }
            }
        }
        self.game_id = gid;
    }
    pub fn leave_room(&mut self) -> bool {
        for r in &mut self.teams {
            r.borrow_mut().leave_room();
        }
        false
    }
    pub fn clear_queue(&mut self) -> bool {
        for r in &mut self.teams {
            r.borrow_mut().clear_queue();
        }
        false
    }

    pub fn ready(&mut self) {
        for g in &mut self.teams {
            g.borrow_mut().ready();
        }
    }
}