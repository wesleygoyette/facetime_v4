use core::error::Error;
pub enum UserCommand {
    Close,
    KeepAlive,
    ListUsers,
}

pub struct UserInputHandler {}

impl UserInputHandler {
    pub async fn handle(input: &str) -> Result<UserCommand, Box<dyn Error>> {
        let input = input.trim();

        match input {
            "" => return Ok(UserCommand::KeepAlive),
            "list users" => return Ok(UserCommand::ListUsers),
            "exit" => return Ok(UserCommand::Close),
            _ => {
                println!("Unknown command");
                return Ok(UserCommand::KeepAlive);
            }
        }
    }
}
