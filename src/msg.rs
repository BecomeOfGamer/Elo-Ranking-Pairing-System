use serde_derive::{Serialize, Deserialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MqttMsg {
    pub topic: String,
    pub msg: String,
    pub time: SystemTime,
}

impl Default for MqttMsg {
    fn default() -> MqttMsg {
        MqttMsg {
            topic: "".to_owned(),
            msg: "".to_owned(),
            time: SystemTime::now(),
        }
    }
}
