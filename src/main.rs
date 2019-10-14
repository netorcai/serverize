use clap::{App, Arg, SubCommand};
use std::io::ErrorKind::{BrokenPipe, ConnectionReset, NotConnected};
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener};
use std::process::{Command, Stdio};
use std::thread;

fn main() {
    let command = SubCommand::with_name("command")
        .about("runs a command with client sockets as stdin/stdout")
        .arg(Arg::with_name("command").multiple(true).required(true));

    let client = SubCommand::with_name("client")
        .about("runs a command, wait for it to connect back to us and plug it into the client")
        .arg(
            Arg::with_name("client_command")
                .multiple(true)
                .required(true),
        );

    let app = App::new("serverize")
        .version("0.1")
        .about("turn anything into a server!")
        .author("Florent Becker")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .help("Sets a port to listen for client connections")
                .takes_value(true),
        )
        .subcommand(command)
        .subcommand(client);

    let matches = app.get_matches();

    let port = matches.value_of("port").unwrap_or("4567");

    match matches.subcommand() {
        ("command", Option::Some(submatches)) => {
            let command = {
                let mut command = Vec::new();
                for word in submatches.values_of("command").unwrap() {
                    command.push(word.to_string());
                }
                command
            };
            serverize_command(port, command).unwrap()
        }
        ("client", Option::Some(submatches)) => {
            let command = {
                let mut command = Vec::new();
                for word in submatches.values_of("client_command").unwrap() {
                    command.push(word.to_string());
                }
                command
            };
            serverize_client(port, command).unwrap()
        }
        _ => println!("subcommand not found"),
    }
}

fn serverize_command(port: &str, command: Vec<String>) -> std::io::Result<()> {
    let addr = format!("[::]:{}", port);
    let listener = TcpListener::bind(addr)?;

    for stream in listener.incoming() {
        let command = command.clone();
        thread::spawn(move || serve_command(&command, stream?));
    }
    Ok(())
}

fn serve_command(command: &[String], stream: std::net::TcpStream) -> std::io::Result<()> {
    let args = &command[1..];
    let running_cmd = Command::new(command[0].clone())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect(&format!("failed to run {}", &command[0]));

    {
        let stdin = running_cmd.stdin.expect("cannot open stdin");
        let stdout = running_cmd.stdout.expect("cannot open stdout");
        let stream_in = stream.try_clone()?;
        let stream_out = stream.try_clone()?;
        let h_in = thread::spawn(move || stdin_thread_buf(BufReader::new(stream_in), stdin));
        let h_out = thread::spawn(move || stdin_thread_buf(BufReader::new(stdout), stream_out));
        h_in.join().ok(); // if one of these threads panics, so be it!
        h_out.join().ok();

        match stream.shutdown(Shutdown::Both) {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.kind() == NotConnected {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }
}

fn stdin_thread_buf(r: impl BufRead, mut w: impl Write) -> std::io::Result<()> {
    for l in r.lines() {
        let l = l?;

        match write!(w, "{}\n", &l) {
            Ok(_) => (),
            Err(e) => {
                if e.kind() == BrokenPipe || e.kind() == ConnectionReset {
                    return Ok(());
                } else {
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

fn serverize_client(port: &str, command: Vec<String>) -> std::io::Result<()> {
    let addr = format!("[::]:{}", port);
    let listener = TcpListener::bind(addr)?;

    for stream in listener.incoming() {
        let command = command.clone();
        thread::spawn(move || serve_client(&command, stream?));
    }

    Ok(())
}

fn serve_client(
    serverizee_cmd: &[String],
    stream: std::net::TcpStream,
) -> Result<(), std::io::Error> {
    println!("serving {:?}", serverizee_cmd);

    let listener = TcpListener::bind("[::]:0")?;
    let port = listener.local_addr()?.port();

    let args = {
        let mut acc = Vec::new();
        for arg in serverizee_cmd[1..].iter() {
            if arg == "HOST" {
                acc.push(String::from("localhost"))
            } else if arg == "PORT" {
                acc.push(format!("{}", port))
            } else {
                acc.push(arg.clone())
            }
        }
        acc
    };

    let cmd = serverizee_cmd[0].clone();
    thread::spawn(move || {
        Command::new(&cmd)
            .args(args)
            .spawn()
            .expect(&format!("failed to run {}", &cmd))
    });

    match listener.incoming().next() {
        Some(Ok(serverizee_stream)) => {
            let serverizee_stream_c = serverizee_stream.try_clone()?;
            let stream_c = stream.try_clone()?;
            thread::spawn(move || stdin_thread_buf(BufReader::new(stream), serverizee_stream));
            thread::spawn(move || stdin_thread_buf(BufReader::new(serverizee_stream_c), stream_c));
        }
        Some(Err(e)) => return Err(e),
        None => return Ok(()),
    }

    Ok(())
}
