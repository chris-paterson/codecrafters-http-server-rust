use itertools::Itertools;
use std::path::PathBuf;
use std::{env, fs};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone, Debug)]
struct ProgramEnv {
    files_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut files_dir: Option<PathBuf> = None;
    for (current, next) in args.into_iter().skip(1).tuples() {
        if current == "--directory" {
            files_dir = Some(PathBuf::from(next));
        }
    }

    let program_env = ProgramEnv { files_dir };

    let listener = TcpListener::bind("127.0.0.1:4221").await?;

    loop {
        let (stream, _) = listener.accept().await?;

        let env_clone = program_env.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, env_clone).await {
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

    File(Vec<u8>),
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
            HttpResponse::File(data) => {
                let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}",
                        data.len()
                    );
                let body = format!("{:?}", data);
                return format!("{}\r\n\r\n{}", header, body);
            }
        }
    }
}

impl Request {
    fn handle_route(self, env: &ProgramEnv) -> HttpResponse {
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
            "files" => {
                match env.files_dir.clone() {
                    None => return HttpResponse::NotFound,
                    Some(mut files_dir) => {
                        let filename = parts.join("/");
                        files_dir.push(&filename);

                        match fs::read_to_string(files_dir) {
                            Err(_) => return HttpResponse::NotFound,
                            Ok(content) => {
                                return HttpResponse::Ok(Some(content));
                            }
                        }
                    }
                };
            }
            _ => HttpResponse::NotFound,
        };
    }
}

async fn handle_connection(mut stream: TcpStream, env: ProgramEnv) -> std::io::Result<()> {
    let buf_reader = BufReader::new(&mut stream);
    let mut lines = buf_reader.lines();

    let mut http_request = Vec::new();

    while let Some(line) = lines.next_line().await? {
        if line.is_empty() {
            break;
        }
        http_request.push(line);
    }

    println!("Env: {:?}\nRequest: {:#?}", env, http_request);

    let request = Request::from(http_request);
    let response = request.handle_route(&env);

    stream.write_all(response.to_string().as_bytes()).await?;

    Ok(())
}
