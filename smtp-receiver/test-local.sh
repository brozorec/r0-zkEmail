#!/bin/bash

# Local testing script for SMTP receiver
# Requires swaks: brew install swaks (macOS) or apt install swaks (Linux)

set -e

echo "=== SMTP Receiver Local Test ==="
echo ""

# Check if swaks is installed
if ! command -v swaks &> /dev/null; then
    echo "Error: swaks is not installed"
    echo "Install with: brew install swaks (macOS) or apt install swaks (Linux)"
    exit 1
fi

# Configuration
SMTP_HOST=${SMTP_HOST:-localhost}
SMTP_PORT=${SMTP_PORT:-2525}
FROM_EMAIL=${FROM_EMAIL:-test@example.com}
TO_EMAIL=${TO_EMAIL:-recipient@yourdomain.com}

echo "Configuration:"
echo "  SMTP Server: $SMTP_HOST:$SMTP_PORT"
echo "  From: $FROM_EMAIL"
echo "  To: $TO_EMAIL"
echo ""

# Check if server is running
echo "Checking if SMTP server is running..."
if ! nc -z $SMTP_HOST $SMTP_PORT 2>/dev/null; then
    echo "Error: SMTP server is not running on $SMTP_HOST:$SMTP_PORT"
    echo ""
    echo "Start the server with:"
    echo "  RISC0_DEV_MODE=1 RUST_LOG=info cargo run --bin smtp-receiver"
    exit 1
fi

echo "✓ SMTP server is running"
echo ""

# Send test email
echo "Sending test email..."
swaks \
    --to "$TO_EMAIL" \
    --from "$FROM_EMAIL" \
    --server "$SMTP_HOST:$SMTP_PORT" \
    --header "Subject: Test Email from SMTP Receiver" \
    --body "This is a test email sent at $(date)" \
    --header "X-Test-ID: $(date +%s)"

echo ""
echo "✓ Email sent successfully"
echo ""
echo "Check received_emails/ directory for the saved .eml file"
echo "Check proofs/ directory for verification results (if auto_verify is enabled)"
