use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    let listener = TcpListener::bind("127.0.0.1:4221").await?;

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                println!("Error handling connection: {}", e);
            }
        });
    }
}

struct Request {
    path: String,
    user_agent: Option<String>,
}

impl<T: AsRef<str>> From<Vec<T>> for Request {
    fn from(strings: Vec<T>) -> Self {
        let request_line: String = strings
            .iter()
            .map(|s| s.as_ref())
            .filter(|s| s.starts_with("GET"))
            .collect();

        let request_parts: Vec<_> = request_line.split(" ").collect();
        let path: String = request_parts[1].into();

        let user_agent_line: String = strings
            .iter()
            .map(|s| s.as_ref())
            .filter(|s| s.starts_with("User-Agent"))
            .collect();

        let user_agent_parts: Vec<_> = user_agent_line.split(" ").collect();

        let user_agent: Option<String> = match user_agent_parts.is_empty() {
            true => None,
            false => {
                if user_agent_parts.len() > 1 {
                    Some(user_agent_parts[1].into())
                } else {
                    None
                }
            }
        };

        Request { path, user_agent }
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

        let mut parts: Vec<&str> = self.path.split("/").filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return HttpResponse::NotFound;
        }

        let first = parts.remove(0);
        return match first {
            "echo" => HttpResponse::Ok(Some(String::from(parts.join("/")))),
            "user-agent" => HttpResponse::Ok(Some(String::from(self.user_agent.unwrap()))),
            _ => HttpResponse::NotFound,
        };
    }
}

async fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let buf_reader = BufReader::new(&mut stream);
    let mut lines = buf_reader.lines();

    let mut http_request = Vec::new();

    while let Some(line) = lines.next_line().await? {
        if line.is_empty() {
            break;
        }
        http_request.push(line);
    }

    println!("Request: {:#?}", http_request);

    let request = Request::from(http_request);
    let response = request.handle_route();

    stream.write_all(response.to_string().as_bytes()).await?;

    Ok(())
}
