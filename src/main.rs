use itertools::Itertools;
use std::io::Write;
use std::path::PathBuf;
use std::{env, fmt, fs};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    method: String,
    path: String,
    user_agent: Option<String>,
    body: String,
}

impl Request {
    pub fn new<T: AsRef<str>>(header_lines: Vec<T>, body: String) -> Self {
        let request_line: String = header_lines
            .iter()
            .map(|s| s.as_ref())
            .filter(|s| s.starts_with("GET") || s.starts_with("POST"))
            .collect();

        let request_parts: Vec<_> = request_line.split(' ').collect();
        let method: String = request_parts[0].into();
        let path: String = request_parts[1].into();

        let user_agent_line: String = header_lines
            .iter()
            .map(|s| s.as_ref())
            .filter(|s| s.starts_with("User-Agent"))
            .collect();

        let user_agent_parts: Vec<_> = user_agent_line.split(' ').collect();

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

        Request {
            method,
            path,
            user_agent,
            body,
        }
    }
}

enum HttpResponse {
    NotFound,

    /// body
    Ok(Option<String>),

    File(String),

    Created,
}

impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let response = match self {
            HttpResponse::NotFound => "HTTP/1.1 404 Not Found\r\n\r\n".to_string(),
            HttpResponse::Created => "HTTP/1.1 201 Created\r\n\r\n".to_string(),
            HttpResponse::Ok(body) => match body {
                None => "HTTP/1.1 200 OK\r\n\r\n".to_string(),
                Some(body) => {
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}",
                        body.len()
                    );
                    format!("{}\r\n\r\n{}", header, body)
                }
            },
            HttpResponse::File(content) => {
                let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}",
                        content.len()
                    );
                format!("{}\r\n\r\n{}", header, content)
            }
        };
        write!(f, "{}", response)
    }
}

impl Request {
    fn handle_route(self, env: &ProgramEnv) -> HttpResponse {
        match self.method.as_ref() {
            "GET" => self.handle_get(env),
            "POST" => self.handle_post(env),
            _ => HttpResponse::NotFound,
        }
    }

    fn handle_get(self, env: &ProgramEnv) -> HttpResponse {
        if self.path == "/" {
            return HttpResponse::Ok(None);
        }

        let mut parts: Vec<&str> = self.path.split('/').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return HttpResponse::NotFound;
        }

        let first = parts.remove(0);
        match first {
            "echo" => HttpResponse::Ok(Some(parts.join("/"))),
            "user-agent" => HttpResponse::Ok(Some(self.user_agent.unwrap())),
            "files" => match env.files_dir.clone() {
                None => HttpResponse::NotFound,
                Some(mut files_dir) => {
                    let filename = parts.join("/");
                    files_dir.push(&filename);

                    match fs::read_to_string(files_dir) {
                        Err(_) => HttpResponse::NotFound,
                        Ok(content) => HttpResponse::File(content),
                    }
                }
            },
            _ => HttpResponse::NotFound,
        }
    }

    fn handle_post(self, env: &ProgramEnv) -> HttpResponse {
        let mut parts: Vec<&str> = self.path.split('/').filter(|s| !s.is_empty()).collect();
        let first = parts.remove(0);

        match first {
            "files" => match env.files_dir.clone() {
                None => HttpResponse::NotFound,
                Some(mut files_dir) => {
                    let filename = parts.join("/");
                    files_dir.push(&filename);
                    let result = fs::File::create(files_dir)
                        .and_then(|mut file| file.write_all(self.body.as_bytes()));

                    match result {
                        Ok(_) => HttpResponse::Created,
                        Err(_) => HttpResponse::NotFound,
                    }
                }
            },
            _ => HttpResponse::NotFound,
        }
    }
}

async fn handle_connection(mut stream: TcpStream, env: ProgramEnv) -> std::io::Result<()> {
    let mut buffer = [0u8; 1024];
    match stream.read(&mut buffer).await {
        Err(e) => Err(e),
        Ok(0) => Ok(()),
        Ok(bytes_read) => {
            let content = String::from_utf8_lossy(&buffer[..bytes_read]);

            let Some((headers, body)) = content.split_once("\r\n\r\n") else {
                    let response = HttpResponse::NotFound;
                    stream.write_all(response.to_string().as_bytes()).await?;
                    return Ok(());
                };

            let header_lines: Vec<_> = headers.split("\r\n").map(|l| l.to_string()).collect();
            println!(
                "Received data: headers = {:#?}\nbody = {:#?}",
                header_lines, body
            );

            let request = Request::new(header_lines, body.into());
            let response = request.handle_route(&env);
            stream.write_all(response.to_string().as_bytes()).await?;
            Ok(())
        }
    }
}
