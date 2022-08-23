use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

fn main() {
    // create listener
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    // listen incoming requests
    for stream in listener.incoming() {
        // receive message as stream
        let stream = stream.unwrap();
        
        // handle stream
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    // create reader to read stream
    let buf_reader = BufReader::new(&mut stream);
    // transform stream into a vector constituted by many string lines
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("hello.html").unwrap();
    let length = contents.len();

    let response = 
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();

}