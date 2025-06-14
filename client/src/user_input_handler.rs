pub enum UserCommand {
    Close,
    KeepAlive,
    ListUsers,
    CreateRoom(String),
    ListRooms,
    JoinRoom(String),
    DeleteRoom(String),
}

pub struct UserInputHandler {}

impl UserInputHandler {
    pub async fn handle(
        input: &str,
    ) -> Result<UserCommand, Box<dyn std::error::Error + Send + Sync>> {
        let input = input.trim();

        match input {
            "" => return Ok(UserCommand::KeepAlive),
            "create room" => {
                println!("Usage: create room <name>");
                return Ok(UserCommand::KeepAlive);
            }
            create_cmd if create_cmd.starts_with("create room ") => {
                let create_cmd_parts: Vec<&str> = create_cmd.split(" ").collect();

                if create_cmd_parts.len() != 3 {
                    println!("Usage: create room <name>");
                    return Ok(UserCommand::KeepAlive);
                }

                let room_name = create_cmd_parts[2];

                return Ok(UserCommand::CreateRoom(room_name.to_string()));
            }
            "delete room" => {
                println!("Usage: delete room <name>");
                return Ok(UserCommand::KeepAlive);
            }
            delete_cmd if delete_cmd.starts_with("delete room ") => {
                let delete_cmd_parts: Vec<&str> = delete_cmd.split(" ").collect();

                if delete_cmd_parts.len() != 3 {
                    println!("Usage: delete room <name>");
                    return Ok(UserCommand::KeepAlive);
                }

                let room_name = delete_cmd_parts[2];

                return Ok(UserCommand::DeleteRoom(room_name.to_string()));
            }
            "join room" => {
                println!("Usage: join room <name>");
                return Ok(UserCommand::KeepAlive);
            }
            join_cmd if join_cmd.starts_with("join room ") => {
                let join_cmd_parts: Vec<&str> = join_cmd.split(" ").collect();

                if join_cmd_parts.len() != 3 {
                    println!("Usage: join room <name>");
                    return Ok(UserCommand::KeepAlive);
                }

                let room_name = join_cmd_parts[2];

                return Ok(UserCommand::JoinRoom(room_name.to_string()));
            }
            "list users" => return Ok(UserCommand::ListUsers),
            "list rooms" => return Ok(UserCommand::ListRooms),
            "exit" => return Ok(UserCommand::Close),
            _ => {
                println!("Unknown command");
                return Ok(UserCommand::KeepAlive);
            }
        }
    }
}
