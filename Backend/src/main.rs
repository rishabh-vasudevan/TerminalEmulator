#[macro_use]
extern crate rocket;
use chatgpt::prelude::*;
use dotenv::dotenv;
use nix::pty::openpty;
use nix::unistd::read;
use regex::Regex;
use rocket::{serde::json::Json, State};
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::Write;
use std::os::unix::io::{FromRawFd, RawFd};
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct Pty {
    fd: i32,
}

fn create_pty(process: &str) -> Pty {
    let ends = openpty(None, None).expect("openpty failed");
    let master = ends.master;
    let slave = ends.slave;

    let mut builder = Command::new(process);
    builder.stdin(unsafe { Stdio::from_raw_fd(slave) });
    builder.stdout(unsafe { Stdio::from_raw_fd(slave) });
    builder.stderr(unsafe { Stdio::from_raw_fd(slave) });

    match builder.spawn() {
        Ok(_) => {
            let pty = Pty { fd: master };

            pty
        }
        Err(e) => {
            panic!("Failed to create pty: {}", e);
        }
    }
}

fn read_from_fd(fd: RawFd) -> Option<Vec<u8>> {
    let mut read_buffer = [0; 65536];
    let read_result = read(fd, &mut read_buffer);
    match read_result {
        Ok(bytes_read) => Some(read_buffer[..bytes_read].to_vec()),
        Err(_e) => None,
    }
}

fn remove_ansi_escape_codes(input: &str) -> String {
    // Regular expression pattern to match ANSI escape codes
    let ansi_escape_code_regex = Regex::new(r"\x1B\[([0-9]{1,2}(;[0-9]{1,2})?)?[m|K]").unwrap();

    // Replace ANSI escape codes with an empty string
    ansi_escape_code_regex
        .replace_all(input, "")
        .to_string()
        .replace("bash-3.2$", "")
}

#[derive(Deserialize)]
struct Request {
    command: String,
}

async fn run_command_in_chat_gpt(command: String) -> String {
    let chat_gpt_api_key = env::var("chatGPTApi").expect("unable to find Key chatGPTApi");
    let client = ChatGPT::new(chat_gpt_api_key).expect("unable to connect to ChaptGPT client");
    let response = client.send_message(command + " : please return a mac command that can run on bash and nothing else, no special symbols also. Nothing other than the command, please don't give and explanation also. for eg if I ask for a command to display current folder you should just return : pwd").await;
    match response {
        Ok(res) => res.message().content.to_owned(),
        Err(e) => {
            panic!(
                "There was an error in sending the command to chatGPT : {}",
                e
            )
        }
    }
}

#[post("/get_output", data = "<request>")]
async fn send_command_to_terminal(request: Json<Request>, pty: &State<Pty>, output_file: &State<File>) -> String {
    if request.command.chars().nth(0).unwrap() == '#' {
        let response_of_chatgpt = run_command_in_chat_gpt(request.command.to_string());
        return request.command.to_string()[1..].to_string()
            + ":\n"
            + response_of_chatgpt.await.as_str();
    }
    let fd_val = pty.fd;
    let mut output = output_file
        .try_clone()
        .expect("Unable to clone output Buffer");
    match write!(output, "{} \n", request.command) {
        Ok(_) => (),
        Err(e) => panic!("There was some error in writing the output : {:?}", e),
    }
    match output.flush() {
        Ok(_) => (),
        Err(_) => panic!("There was some error in flushing the output"),
    }
    loop {
        match read_from_fd(fd_val) {
            Some(read_bytes) => {
                let std_output = String::from_utf8(read_bytes).unwrap();
                return remove_ansi_escape_codes(&std_output);
            }
            None => continue,
        }
    }
}

#[launch]
fn rocket() -> _ {
    let shell = "/bin/bash";
    let pty = create_pty(&shell);
    println!("The FD for the PTY is {}", pty.fd);
    let output_file: File = unsafe { File::from_raw_fd(pty.fd) };
    dotenv().expect("Could not load .env file");
    rocket::build()
        .mount("/", routes![send_command_to_terminal])
        .manage(pty)
        .manage(output_file)
}
