# prove-api

REST API server for ZK email proof generation using RISC0 zkVM with CUDA acceleration.

## Endpoints

### GET /ping
Health check endpoint for load balancer.

**Response:**
```json
{
  "status": "ok",
  "uptime_secs": 123
}
```

### POST /generate
Generate a ZK proof for an email pair.

**Request Body (JSON):**
```json
{
  "sender_domain": "example.com",
  "sender_raw": "raw email content...",
  "receiver_domain": "receiver.com", 
  "receiver_raw": "raw reply email content..."
}
```

**Response:**
- Success: `200 OK` with `application/octet-stream` body (bincode serialized `risc0_zkvm::Receipt`)
- Error: `500 Internal Server Error` with JSON error message

## Local Development

```bash
# Run locally (without CUDA)
cargo run --package prove-api

# Run with CUDA
cargo run --package prove-api --features cuda

# Set custom port
PORT=3000 cargo run --package prove-api
```

## Docker Build

```bash
# Build from workspace root
docker build -f prove-api/Dockerfile -t prove-api:cuda .

# Run locally with GPU
docker run --gpus all -p 8080:8080 prove-api:cuda
```

## RunPod Serverless Deployment

### 1. Push to Container Registry

```bash
# Tag and push to Docker Hub or other registry
docker tag prove-api:cuda your-registry/prove-api:cuda
docker push your-registry/prove-api:cuda
```

### 2. RunPod Configuration

Create a new Serverless endpoint in RunPod with:

- **Container Image:** `your-registry/prove-api:cuda`
- **GPU Type:** Select appropriate GPU (e.g., RTX 4090, A100)
- **Container Disk:** 20GB+ (for RISC0 artifacts)
- **Volume Disk:** Optional
- **Exposed Port:** 8080
- **Health Check Path:** `/ping`

### 3. Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | Server port |
| `RUST_LOG` | `info` | Log level (debug, info, warn, error) |

### 4. Load Balancer Integration

The `/ping` endpoint returns health status for load balancer health checks.
Configure your load balancer to:
- Health check: `GET /ping` every 30s
- Timeout: 10s
- Unhealthy threshold: 3 failures

## Example Usage

```bash
# Health check
curl http://localhost:8080/ping

# Generate proof
curl -X POST http://localhost:8080/generate \
  -H "Content-Type: application/json" \
  -d '{
    "sender_domain": "example.com",
    "sender_raw": "...",
    "receiver_domain": "receiver.com",
    "receiver_raw": "..."
  }' \
  --output receipt.bin
```

## Python Client Example

```python
import requests

# Generate proof
response = requests.post(
    "http://localhost:8080/generate",
    json={
        "sender_domain": "example.com",
        "sender_raw": open("sender.eml").read(),
        "receiver_domain": "receiver.com",
        "receiver_raw": open("receiver.eml").read(),
    }
)

if response.status_code == 200:
    with open("receipt.bin", "wb") as f:
        f.write(response.content)
else:
    print(f"Error: {response.json()}")
```
