use anyhow::Result;
use log::{error, info};
use mailin_embedded::{Handler, Server, SslConfig};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{config::Config, email_handler::EmailHandler};

pub struct SmtpHandler {
    email_handler: Arc<Mutex<EmailHandler>>,
}

impl SmtpHandler {
    pub fn new(email_handler: Arc<Mutex<EmailHandler>>) -> Self {
        Self { email_handler }
    }
}

impl Handler for SmtpHandler {
    fn helo(&mut self, _ip: std::net::IpAddr, _domain: &str) -> mailin_embedded::Response {
        info!("HELO from domain: {}", _domain);
        mailin_embedded::response::OK
    }

    fn mail(&mut self, _ip: std::net::IpAddr, _domain: &str, from: &str) -> mailin_embedded::Response {
        info!("MAIL FROM: {}", from);
        mailin_embedded::response::OK
    }

    fn rcpt(&mut self, to: &str) -> mailin_embedded::Response {
        info!("RCPT TO: {}", to);
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
        mailin_embedded::response::OK
    }

    fn data(&mut self, buf: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn data_end(
        &mut self,
        _domain: &str,
        from: &str,
        _is8bit: bool,
        to: &[String],
        data: Vec<u8>,
    ) -> mailin_embedded::Response {
        info!("DATA transmission complete, processing email...");

        let raw_email = match String::from_utf8(data) {
            Ok(email) => email,
            Err(e) => {
                error!("Failed to parse email as UTF-8: {}", e);
                return mailin_embedded::response::INTERNAL_ERROR;
            }
        };

        let handler = self.email_handler.clone();
        let from = from.to_string();
        let to = to.to_vec();

        tokio::spawn(async move {
            let handler = handler.lock().await;
            if let Err(e) = handler.handle_email(&raw_email, &from, &to).await {
                error!("Failed to handle email: {}", e);
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

        let handler = SmtpHandler::new(self.email_handler.clone());

        let mut server = Server::new(handler);
        server
            .with_name("r0-zkEmail SMTP Receiver")
            .with_ssl(SslConfig::None)
            .map_err(|e| anyhow::anyhow!("Failed to configure server: {}", e))?
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
