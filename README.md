# Incident Commander (Rust Backend)

MVP backend service for incident lifecycle management.

## Stack
- Rust
- Axum (HTTP API)
- Tokio (async runtime)
- In-memory state (for v1 bootstrap)

## Run
```bash
source ~/.cargo/env
cargo run
```

Server starts on:
- `http://localhost:3000`

## API (MVP)

### Health
```bash
curl http://localhost:3000/health
```

### Create Incident
```bash
curl -X POST http://localhost:3000/incidents \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "Database latency spike",
    "description": "P99 jumped above SLO",
    "severity": "high"
  }'
```

### List Incidents
```bash
curl http://localhost:3000/incidents
```

### Get Incident
```bash
curl http://localhost:3000/incidents/<INCIDENT_ID>
```

### Acknowledge Incident
```bash
curl -X POST http://localhost:3000/incidents/<INCIDENT_ID>/ack
```

### Resolve Incident
```bash
curl -X POST http://localhost:3000/incidents/<INCIDENT_ID>/resolve
```

## Notes
- Severity enum: `low | medium | high | critical`
- Status flow: `open -> acknowledged -> resolved`
- Data resets on restart (in-memory store)
