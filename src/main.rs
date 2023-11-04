use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;

fn main() -> std::io::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        handle_connection(stream?);
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:#?}", http_request);

    let response = match http_request.first() {
        Some(x) => {
            // GET /index.html HTTP/1.1
            let parts: Vec<_> = x.split(" ").collect();
            let path = parts[1];

            if path == "/" {
                "HTTP/1.1 200 OK\r\n\r\n"
            } else {
                "HTTP/1.1 404 Not Found\r\n\r\n"
            }
        }
        None => "HTTP/1.1 404 Not Found\r\n\r\n",
    };

    stream.write_all(String::from(response).as_bytes()).unwrap();
}
