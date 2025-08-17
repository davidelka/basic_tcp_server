use std::fs;
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};

use log;
use smt_web_server::{ServerError, ThreadPool};

fn main() {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind port 7878");
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(e) => {
                log::error!("Error in stream listener {e}");
                continue;
            }
        };

        pool.execute(|| handle_connection(stream));
        log::info!("Connection established!");
    }
}

fn handle_connection(mut stream: TcpStream) -> Result<(), ServerError> {
    let buf_reader = BufReader::new(&stream);

    let request_line = match buf_reader.lines().next() {
        Some(request_line) => match request_line {
            Ok(request) => request,
            Err(e) => {
                log::error!("Error reading request line {}", e);
                return Ok(());
            }
        },
        None => {
            log::error!("Failed to read from stream");
            return Ok(());
        }
    };

    let (status_line, filename) = if request_line == "GET / HTTP/1.1" {
        ("HTTP/1.1 200 OK", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };

    let contents = fs::read_to_string(filename);

    match contents {
        Ok(contents) => {
            let contents = contents.trim();
            let length = contents.len();

            let response = format!(
                "{status_line}\r\nContent-Length: {length}\r\n\r\n{}",
                contents
            );

            stream
                .write_all(response.as_bytes())
                .map_err(|_| ServerError::InternalError("Failed to write to stream".to_owned()))
        }
        Err(e) => {
            log::error!("Error: {}", e);

            let status_line = "HTTP/1.1 500 OK";
            let response = format!("{status_line}");

            stream
                .write_all(response.as_bytes())
                .map_err(|_| ServerError::InternalError("Failed to write to stream".to_owned()))
        }
    }
}
