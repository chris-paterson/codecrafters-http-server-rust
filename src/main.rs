use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

fn main() -> std::io::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        handle_connection(stream?);
    }

    Ok(())
}

struct Request {
    path: String,
}

impl<T: AsRef<str>> From<T> for Request {
    fn from(string: T) -> Self {
        let parts: Vec<_> = string.as_ref().split(" ").collect();
        let path = parts[1].into();

        Request { path }
    }
}

impl Request {
    fn handle_valid_path(self) -> String {
        if self.path == "/" {
            "HTTP/1.1 200 OK\r\n\r\n".into()
        } else {
            "HTTP/1.1 404 Not Found\r\n\r\n".into()
        }
    }

    fn handle_echo(self) -> String {
        let parts: Vec<_> = self.path.split("/").filter(|s| !s.is_empty()).collect();
        println!("{:?}", parts);
        if parts[0] == "echo" {
            let param = parts[1];
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n",
                param.len(),
                param
            );
            return resp.into();
        }
        return "HTTP/1.1 404 Not Found\r\n\r\n".into();
    }
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
            let request = Request::from(x);
            //request.handle_valid_path()
            request.handle_echo()
        }
        None => "HTTP/1.1 404 Not Found\r\n\r\n".into(),
    };

    stream.write_all(response.as_bytes()).unwrap();
}
