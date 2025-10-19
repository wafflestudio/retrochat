use anyhow::Result;

pub async fn handle_web_command(host: String, port: u16, open: bool) -> Result<()> {
    let url = format!("http://{host}:{port}");

    // Open browser if requested
    if open {
        println!("🚀 Opening browser at {url}");
        // Try to open browser, but don't fail if it doesn't work
        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("open")
                .arg(&url)
                .spawn()
                .or_else(|_| std::process::Command::new("xdg-open").arg(&url).spawn());
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", &url])
                .spawn();
        }
    }

    // Start the web server
    crate::web::run_server(&host, port).await
}
