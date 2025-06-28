use {
    interprocess::local_socket::{
        GenericNamespaced, ListenerOptions, ToNsName,
        traits::tokio::{Listener, Stream},
    },
    std::time::Duration,
    tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        time::timeout,
    },
};

const PIPE_NAME: &str = "pano-scrobbler-ipc";

pub async fn commands_listener(
    jni_callback: impl Fn(String, String, String) + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = PIPE_NAME.to_ns_name::<GenericNamespaced>()?;

    let listener = ListenerOptions::new()
        .name(name)
        .reclaim_name(true)
        .create_tokio();

    let listener = match listener {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error creating pipe listener: {e}");
            return Ok(()); // dont actually return the error, as I will be using try_join
        }
    };

    let mut buffer = String::with_capacity(128);

    loop {
        match listener.accept().await {
            Ok(conn) => {
                let mut conn = BufReader::new(conn);

                match conn.read_line(&mut buffer).await {
                    Ok(_) => {
                        let parts: Vec<&str> = buffer.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            let command = parts[0].trim();
                            let arg = parts[1].trim();

                            jni_callback(
                                "onReceiveIpcCommand".to_string(),
                                command.to_string(),
                                arg.to_string(),
                            );
                        }

                        buffer.clear();
                    }
                    Err(e) => {
                        eprintln!("Error reading from pipe: {e}");
                        buffer.clear();
                    }
                }
            }
            Err(e) => {
                eprintln!("There was an error with an incoming connection: {e}");
                continue;
            }
        };
    }
}

async fn connect() -> Result<interprocess::local_socket::tokio::Stream, Box<dyn std::error::Error>>
{
    let name = PIPE_NAME.to_ns_name::<GenericNamespaced>()?;

    match timeout(
        Duration::from_millis(500),
        interprocess::local_socket::tokio::Stream::connect(name),
    )
    .await
    {
        Ok(Ok(stream)) => Ok(stream),
        Err(e) => Err(Box::new(e)),
        Ok(Err(e)) => Err(Box::new(e)),
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn send_command(command: &str, arg: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = connect().await?;

    conn.write_all(command.as_bytes()).await?;
    conn.write_all(b" ").await?;
    conn.write_all(arg.as_bytes()).await?;

    conn.write_all(b"\n").await?;

    Ok(())
}
