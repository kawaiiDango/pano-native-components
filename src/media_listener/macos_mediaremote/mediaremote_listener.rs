pub fn listener() -> Result<(), Box<dyn std::error::Error>> {
    println!("macos_mediaremote::listener()");

    loop {
        // todo: This is a dummy implementation that does nothing
        std::thread::sleep(std::time::Duration::from_secs(10));
    }

    Ok(())
}
