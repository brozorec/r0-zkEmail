use anyhow::Result;
use log::info;
use mailin_embedded::{Handler, Server, SslConfig};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{config::Config, email_handler::EmailHandler};

#[derive(Clone)]
pub struct SmtpHandler {
    email_handler: Arc<Mutex<EmailHandler>>,
    from: Arc<Mutex<Option<String>>>,
    to: Arc<Mutex<Vec<String>>>,
    data: Arc<Mutex<Vec<u8>>>,
}

impl SmtpHandler {
    pub fn new(email_handler: Arc<Mutex<EmailHandler>>) -> Self {
        Self {
            email_handler,
            from: Arc::new(Mutex::new(None)),
            to: Arc::new(Mutex::new(Vec::new())),
            data: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Handler for SmtpHandler {
    fn helo(&mut self, _ip: std::net::IpAddr, _domain: &str) -> mailin_embedded::Response {
        info!("HELO from domain: {}", _domain);
        mailin_embedded::response::OK
    }

    fn mail(&mut self, _ip: std::net::IpAddr, _domain: &str, from: &str) -> mailin_embedded::Response {
        info!("MAIL FROM: {}", from);
        if let Ok(mut f) = self.from.try_lock() {
            *f = Some(from.to_string());
        }
        mailin_embedded::response::OK
    }

    fn rcpt(&mut self, to: &str) -> mailin_embedded::Response {
        info!("RCPT TO: {}", to);
        if let Ok(mut t) = self.to.try_lock() {
            t.push(to.to_string());
        }
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
        // Clear previous data
        if let Ok(mut d) = self.data.try_lock() {
            d.clear();
        }
        mailin_embedded::response::OK
    }

    fn data(&mut self, buf: &[u8]) -> std::io::Result<()> {
        // Accumulate email data
        if let Ok(mut d) = self.data.try_lock() {
            d.extend_from_slice(buf);
        }
        Ok(())
    }

    fn data_end(&mut self) -> mailin_embedded::Response {
        info!("DATA transmission complete, processing email...");
        
        // Get accumulated data
        let data = if let Ok(d) = self.data.try_lock() {
            d.clone()
        } else {
            return mailin_embedded::response::INTERNAL_ERROR;
        };
        
        let raw_email = match String::from_utf8(data) {
            Ok(email) => email,
            Err(_) => return mailin_embedded::response::INTERNAL_ERROR,
        };
        
        let from = if let Ok(f) = self.from.try_lock() {
            f.clone().unwrap_or_else(|| "unknown".to_string())
        } else {
            "unknown".to_string()
        };
        
        let to = if let Ok(t) = self.to.try_lock() {
            t.clone()
        } else {
            Vec::new()
        };
        
        // Process email asynchronously
        let handler = self.email_handler.clone();
        tokio::spawn(async move {
            if let Ok(_) = handler.lock().await.handle_email(&raw_email, &from, &to).await {
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
