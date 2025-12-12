# SMTP Receiver Deployment Guide

Complete guide for deploying the SMTP receiver on a cloud VM.

## Prerequisites

- Cloud VM (AWS EC2, DigitalOcean, Linode, etc.)
- Ubuntu 22.04 or similar Linux distribution
- Domain name with DNS access
- Static IP address

## Step-by-Step Deployment

### 1. VM Setup

#### Recommended Specs

- **CPU**: 4+ cores (ZK proof generation is CPU-intensive)
- **RAM**: 8GB+ (16GB recommended for production)
- **Storage**: 50GB+ SSD
- **OS**: Ubuntu 22.04 LTS

#### Initial Server Setup

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install required packages
sudo apt install -y build-essential curl git pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install Docker (required for Groth16 proofs)
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER
newgrp docker
```

### 2. DNS Configuration

#### A Record

Point your mail server to your VM:

```
Type: A
Host: mail.your-domain.com
Value: <YOUR_VM_IP>
TTL: 3600
```

#### MX Record

Configure mail exchange for your domain:

```
Type: MX
Host: your-domain.com
Value: mail.your-domain.com
Priority: 10
TTL: 3600
```

#### Reverse DNS (PTR)

**Critical for email reputation!**

Configure through your hosting provider's control panel:

```
<YOUR_VM_IP> → mail.your-domain.com
```

Verify with:

```bash
dig -x <YOUR_VM_IP>
host <YOUR_VM_IP>
```

#### SPF Record (Optional but Recommended)

```
Type: TXT
Host: your-domain.com
Value: "v=spf1 ip4:<YOUR_VM_IP> -all"
TTL: 3600
```

### 3. Firewall Configuration

```bash
# Enable UFW
sudo ufw enable

# Allow SSH (IMPORTANT: Do this first!)
sudo ufw allow 22/tcp

# Allow SMTP ports
sudo ufw allow 25/tcp    # Standard SMTP
sudo ufw allow 587/tcp   # Submission
sudo ufw allow 2525/tcp  # Alternative (for testing)

# Check status
sudo ufw status
```

### 4. Clone and Build

```bash
# Clone repository
cd ~
git clone https://github.com/risc0-labs/r0-zkEmail.git
cd r0-zkEmail

# Build the project (this will take a while)
cargo build --release --bin smtp-receiver

# Verify build
ls -lh target/release/smtp-receiver
```

### 5. Configuration

```bash
# Navigate to smtp-receiver directory
cd smtp-receiver

# Generate default config
../target/release/smtp-receiver --generate-config config.toml

# Edit configuration
nano config.toml
```

**Production config.toml:**

```toml
[smtp]
bind_address = "0.0.0.0"
port = 25  # Standard SMTP port
max_message_size = 52428800
allowed_domains = ["gmail.com", "outlook.com"]  # Add your allowed domains

[smtp.tls]
cert_path = "/home/ubuntu/r0-zkEmail/smtp-receiver/certs/cert.pem"
key_path = "/home/ubuntu/r0-zkEmail/smtp-receiver/certs/key.pem"

[storage]
email_dir = "/var/smtp-receiver/emails"
proof_dir = "/var/smtp-receiver/proofs"

[processing]
auto_verify = true
extract_domain_from_sender = true
```

### 6. Create Storage Directories

```bash
# Create directories
sudo mkdir -p /var/smtp-receiver/{emails,proofs}

# Set ownership
sudo chown -R $USER:$USER /var/smtp-receiver

# Set permissions
chmod 750 /var/smtp-receiver/{emails,proofs}
```

### 7. Grant Port Binding Capability

To run on port 25 without root:

```bash
# Grant capability to bind privileged ports
sudo setcap 'cap_net_bind_service=+ep' ../target/release/smtp-receiver

# Verify
getcap ../target/release/smtp-receiver
# Should output: cap_net_bind_service=ep
```

### 8. Create Systemd Service

Create `/etc/systemd/system/smtp-receiver.service`:

```bash
sudo nano /etc/systemd/system/smtp-receiver.service
```

```ini
[Unit]
Description=r0-zkEmail SMTP Receiver
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=ubuntu
Group=ubuntu
WorkingDirectory=/home/ubuntu/r0-zkEmail/smtp-receiver

# Environment variables
Environment="RUST_LOG=info"
Environment="RISC0_DEV_MODE=0"
Environment="PATH=/home/ubuntu/.cargo/bin:/usr/local/bin:/usr/bin:/bin"

# Start command
ExecStart=/home/ubuntu/r0-zkEmail/target/release/smtp-receiver /home/ubuntu/r0-zkEmail/smtp-receiver/config.toml

# Restart policy
Restart=always
RestartSec=10

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/var/smtp-receiver

# Resource limits
LimitNOFILE=65536
MemoryMax=8G

[Install]
WantedBy=multi-user.target
```

**Adjust paths** based on your username and installation location.

### 9. Enable and Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable smtp-receiver

# Start service
sudo systemctl start smtp-receiver

# Check status
sudo systemctl status smtp-receiver

# View logs
sudo journalctl -u smtp-receiver -f
```

### 10. Verify Deployment

#### Check Service Status

```bash
# Service should be active
sudo systemctl status smtp-receiver

# Check if port is listening
sudo netstat -tlnp | grep :25
# or
sudo ss -tlnp | grep :25
```

#### Test DNS Configuration

```bash
# Test MX record
dig MX your-domain.com

# Test A record
dig mail.your-domain.com

# Test reverse DNS
dig -x <YOUR_VM_IP>
```

#### Send Test Email

From another machine:

```bash
# Install swaks if needed
sudo apt install swaks

# Send test email
swaks --to test@your-domain.com \
      --from sender@gmail.com \
      --server mail.your-domain.com:25 \
      --header "Subject: Test Email" \
      --body "Testing SMTP receiver"
```

#### Check Received Emails

```bash
# List received emails
ls -lh /var/smtp-receiver/emails/

# View latest email
cat /var/smtp-receiver/emails/*.eml | tail -n 50

# Check proofs
ls -lh /var/smtp-receiver/proofs/
cat /var/smtp-receiver/proofs/*.json | jq .
```

## Monitoring and Maintenance

### Log Management

```bash
# View real-time logs
sudo journalctl -u smtp-receiver -f

# View last 100 lines
sudo journalctl -u smtp-receiver -n 100

# View logs from today
sudo journalctl -u smtp-receiver --since today

# View errors only
sudo journalctl -u smtp-receiver -p err
```

### Log Rotation

Create `/etc/logrotate.d/smtp-receiver`:

```bash
sudo nano /etc/logrotate.d/smtp-receiver
```

```
/var/log/smtp-receiver/*.log {
    daily
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 ubuntu ubuntu
    sharedscripts
    postrotate
        systemctl reload smtp-receiver > /dev/null 2>&1 || true
    endscript
}
```

### Disk Space Monitoring

```bash
# Check disk usage
df -h /var/smtp-receiver

# Check email directory size
du -sh /var/smtp-receiver/emails/

# Clean old emails (older than 30 days)
find /var/smtp-receiver/emails/ -name "*.eml" -mtime +30 -delete
```

### Performance Monitoring

```bash
# Monitor CPU and memory
htop

# Check service resource usage
systemctl status smtp-receiver

# Monitor Docker (for Groth16 proofs)
docker stats
```

## Security Hardening

### fail2ban Configuration

Protect against brute force attacks:

```bash
# Install fail2ban
sudo apt install fail2ban

# Create filter
sudo nano /etc/fail2ban/filter.d/smtp-receiver.conf
```

```ini
[Definition]
failregex = ^.*Failed connection from <HOST>.*$
            ^.*Invalid command from <HOST>.*$
ignoreregex =
```

```bash
# Create jail
sudo nano /etc/fail2ban/jail.d/smtp-receiver.conf
```

```ini
[smtp-receiver]
enabled = true
port = 25,587,2525
filter = smtp-receiver
logpath = /var/log/syslog
maxretry = 5
bantime = 3600
findtime = 600
```

```bash
# Restart fail2ban
sudo systemctl restart fail2ban
sudo fail2ban-client status smtp-receiver
```

### TLS/SSL Certificate Setup

TLS is required. Generate certificates before running the server.

#### Option 1: Self-Signed (Development/Testing)

```bash
# Create certs directory
mkdir -p certs

# Generate self-signed certificate (valid for 365 days)
openssl req -x509 -newkey rsa:4096 -keyout certs/key.pem -out certs/cert.pem -days 365 -nodes \
    -subj "/CN=mail.your-domain.com"
```

#### Option 2: Let's Encrypt (Production)

```bash
# Install certbot
sudo apt install certbot

# Generate certificate (requires port 80 open and DNS configured)
sudo certbot certonly --standalone -d mail.your-domain.com

# Certificates will be at:
#   /etc/letsencrypt/live/mail.your-domain.com/fullchain.pem
#   /etc/letsencrypt/live/mail.your-domain.com/privkey.pem

# Copy to your certs directory (or update config paths)
sudo cp /etc/letsencrypt/live/mail.your-domain.com/fullchain.pem certs/cert.pem
sudo cp /etc/letsencrypt/live/mail.your-domain.com/privkey.pem certs/key.pem
sudo chown $USER:$USER certs/*.pem
chmod 600 certs/key.pem
```

#### Option 3: Let's Encrypt with Auto-Renewal

```bash
# Set up auto-renewal cron job
sudo crontab -e
```

Add:
```
0 0 1 * * certbot renew --quiet && cp /etc/letsencrypt/live/mail.your-domain.com/fullchain.pem /home/ubuntu/r0-zkEmail/smtp-receiver/certs/cert.pem && cp /etc/letsencrypt/live/mail.your-domain.com/privkey.pem /home/ubuntu/r0-zkEmail/smtp-receiver/certs/key.pem && systemctl restart smtp-receiver
```

#### Configure TLS in config.toml

```toml
[smtp.tls]
cert_path = "./certs/cert.pem"
key_path = "./certs/key.pem"
```

#### Verify TLS is Working

```bash
# Test with openssl
openssl s_client -connect mail.your-domain.com:25 -starttls smtp

# Or with swaks
swaks --to test@example.com --from sender@test.com --server mail.your-domain.com:25 --tls
```

## Troubleshooting

### Service Won't Start

```bash
# Check logs for errors
sudo journalctl -u smtp-receiver -n 50

# Check binary permissions
ls -l /home/ubuntu/r0-zkEmail/target/release/smtp-receiver
getcap /home/ubuntu/r0-zkEmail/target/release/smtp-receiver

# Test manually
cd /home/ubuntu/r0-zkEmail/smtp-receiver
RUST_LOG=debug ../target/release/smtp-receiver config.toml
```

### Port Already in Use

```bash
# Check what's using port 25
sudo lsof -i :25

# If Postfix is running
sudo systemctl stop postfix
sudo systemctl disable postfix
```

### Emails Not Being Received

```bash
# Test connectivity
telnet mail.your-domain.com 25

# Check firewall
sudo ufw status

# Check DNS
dig MX your-domain.com
```

### DKIM Verification Failures

```bash
# Check logs for specific errors
sudo journalctl -u smtp-receiver | grep -i dkim

# Test DNS resolution
dig TXT default._domainkey.gmail.com

# Verify email has DKIM headers
cat /var/smtp-receiver/emails/latest.eml | grep -i "dkim-signature"
```

### High Memory Usage

```bash
# Check memory
free -h

# Limit memory in systemd service
# Add to [Service] section:
MemoryMax=8G
MemoryHigh=6G
```

## Backup and Recovery

### Backup Strategy

```bash
# Create backup script
sudo nano /usr/local/bin/backup-smtp-receiver.sh
```

```bash
#!/bin/bash
BACKUP_DIR="/backup/smtp-receiver"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p $BACKUP_DIR

# Backup emails and proofs
tar -czf $BACKUP_DIR/emails_$DATE.tar.gz /var/smtp-receiver/emails/
tar -czf $BACKUP_DIR/proofs_$DATE.tar.gz /var/smtp-receiver/proofs/

# Backup config
cp /home/ubuntu/r0-zkEmail/smtp-receiver/config.toml $BACKUP_DIR/config_$DATE.toml

# Keep only last 7 days
find $BACKUP_DIR -name "*.tar.gz" -mtime +7 -delete

echo "Backup completed: $DATE"
```

```bash
# Make executable
sudo chmod +x /usr/local/bin/backup-smtp-receiver.sh

# Add to crontab (daily at 2 AM)
sudo crontab -e
```

```
0 2 * * * /usr/local/bin/backup-smtp-receiver.sh >> /var/log/smtp-backup.log 2>&1
```

## Updating

```bash
# Stop service
sudo systemctl stop smtp-receiver

# Pull latest code
cd ~/r0-zkEmail
git pull

# Rebuild
cargo build --release --bin smtp-receiver

# Re-apply capability
sudo setcap 'cap_net_bind_service=+ep' target/release/smtp-receiver

# Restart service
sudo systemctl start smtp-receiver

# Check status
sudo systemctl status smtp-receiver
```

## Production Checklist

- [ ] VM with adequate resources (4+ CPU, 8GB+ RAM)
- [ ] Static IP address assigned
- [ ] DNS A record configured
- [ ] DNS MX record configured
- [ ] Reverse DNS (PTR) configured
- [ ] SPF record configured (optional)
- [ ] Firewall rules configured
- [ ] TLS certificates generated and configured
- [ ] SMTP receiver built and installed
- [ ] Configuration file created with TLS paths
- [ ] Storage directories created with correct permissions
- [ ] Port binding capability granted
- [ ] Systemd service created and enabled
- [ ] Service running and listening on port 25
- [ ] TLS working (test with `openssl s_client`)
- [ ] Test email sent and received successfully
- [ ] DKIM verification working
- [ ] Logs being written correctly
- [ ] Log rotation configured
- [ ] Backup script configured
- [ ] Monitoring set up
- [ ] fail2ban configured (optional)
- [ ] Certificate auto-renewal configured (if using Let's Encrypt)
