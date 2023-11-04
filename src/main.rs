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

enum HttpResponse {
    NotFound,

    /// body
    Ok(Option<String>),
}

impl HttpResponse {
    fn to_string(&self) -> String {
        match self {
            HttpResponse::NotFound => String::from("HTTP/1.1 404 Not Found\r\n\r\n"),
            HttpResponse::Ok(body) => match body {
                None => return String::from("HTTP/1.1 200 OK\r\n\r\n"),
                Some(body) => {
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}",
                        body.len()
                    );
                    let body = format!("{}", body);
                    return format!("{}\r\n\r\n{}", header, body);
                }
            },
        }
    }
}

impl Request {
    fn handle_route(self) -> HttpResponse {
        if self.path == "/" {
            return HttpResponse::Ok(None);
        }

        if let Some((path, params)) = self
            .path
            .chars()
            .skip(1)
            .collect::<String>()
            .split_once("/")
        {
            println!("path={path}, params={params}");
            return match path {
                "echo" => HttpResponse::Ok(Some(String::from(params))),
                _ => HttpResponse::NotFound,
            };
        };

        return HttpResponse::NotFound;
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
            request.handle_route()
        }
        None => HttpResponse::NotFound,
    };

    stream.write_all(response.to_string().as_bytes()).unwrap();
}
