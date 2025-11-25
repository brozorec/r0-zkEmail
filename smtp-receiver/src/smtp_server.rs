use anyhow::Result;
use log::info;
use mailin_embedded::{Handler, Server, SslConfig};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::Mutex;

use crate::{config::Config, email_handler::EmailHandler};

#[derive(Clone)]
pub struct SmtpHandler {
    email_handler: Arc<Mutex<EmailHandler>>,
    runtime: Handle,
    // Per-connection state (reset on data_start)
    from: String,
    to: Vec<String>,
    data: Vec<u8>,
}

impl SmtpHandler {
    pub fn new(email_handler: Arc<Mutex<EmailHandler>>, runtime: Handle) -> Self {
        Self {
            email_handler,
            runtime,
            from: String::new(),
            to: Vec::new(),
            data: Vec::new(),
        }
    }
}

impl Handler for SmtpHandler {
    fn helo(&mut self, _ip: std::net::IpAddr, domain: &str) -> mailin_embedded::Response {
        info!("HELO from domain: {}", domain);
        mailin_embedded::response::OK
    }

    fn mail(&mut self, _ip: std::net::IpAddr, _domain: &str, from: &str) -> mailin_embedded::Response {
        info!("MAIL FROM: {}", from);
        self.from = from.to_string();
        mailin_embedded::response::OK
    }

    fn rcpt(&mut self, to: &str) -> mailin_embedded::Response {
        info!("RCPT TO: {}", to);
        self.to.push(to.to_string());
        mailin_embedded::response::OK
    }

    fn data_start(
        &mut self,
        _domain: &str,
        _from: &str,
        _is8bit: bool,
        _to: &[String],
    ) -> mailin_embedded::Response {
        info!("Starting DATA transmission");
        self.data.clear();
        mailin_embedded::response::OK
    }

    fn data(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.data.extend_from_slice(buf);
        Ok(())
    }

    fn data_end(&mut self) -> mailin_embedded::Response {
        info!("DATA transmission complete, processing email...");

        let raw_email = match String::from_utf8(std::mem::take(&mut self.data)) {
            Ok(email) => email,
            Err(_) => return mailin_embedded::response::INTERNAL_ERROR,
        };

        let from = std::mem::take(&mut self.from);
        let to = std::mem::take(&mut self.to);

        let handler = self.email_handler.clone();
        self.runtime.spawn(async move {
            if handler.lock().await.handle_email(&raw_email, &from, &to).await.is_ok() {
                info!("Email processed successfully");
            }
        });

        mailin_embedded::response::OK
    }
}

pub struct SmtpServer {
    config: Config,
    email_handler: Arc<Mutex<EmailHandler>>,
}

impl SmtpServer {
    pub async fn new(config: Config) -> Result<Self> {
        let email_handler = Arc::new(Mutex::new(EmailHandler::new(config.clone()).await?));

        Ok(Self {
            config,
            email_handler,
        })
    }

    pub fn run(&self) -> Result<()> {
        let bind_addr = format!("{}:{}", self.config.smtp.bind_address, self.config.smtp.port);
        info!("Starting SMTP server on {}", bind_addr);

        let runtime = Handle::current();
        let handler = SmtpHandler::new(self.email_handler.clone(), runtime);

        let mut server = Server::new(handler);
        
        let ssl_config = SslConfig::SelfSigned {
            cert_path: self.config.smtp.tls.cert_path.to_string_lossy().into_owned(),
            key_path: self.config.smtp.tls.key_path.to_string_lossy().into_owned(),
        };

        server
            .with_name("r0-zkEmail SMTP Receiver")
            .with_ssl(ssl_config)
            .map_err(|e| anyhow::anyhow!("Failed to configure SSL: {}", e))?
            .with_addr(&bind_addr)
            .map_err(|e| anyhow::anyhow!("Failed to bind to address: {}", e))?;

        info!("SMTP server listening on {}", bind_addr);
        info!("Ready to receive emails...");

        server
            .serve()
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}
