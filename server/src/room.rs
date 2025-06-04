use shared::RoomStreamID;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Room {
    pub name: String,
    pub username_to_rsid: HashMap<String, RoomStreamID>,
}
