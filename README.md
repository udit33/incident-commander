# Incident Commander (Rust Backend)

MVP backend service for incident lifecycle management.

## Project Docs
- [Architecture](./ARCHITECTURE.md)
- [Roadmap](./docs/ROADMAP.md)
- [ADRs](./docs/adr)
- [Contributing](./CONTRIBUTING.md)

## Stack
- Rust
- Axum (HTTP API)
- Tokio (async runtime)
- SQLite + sqlx (persistent storage)

## Run
```bash
source ~/.cargo/env
cargo run
```

Server starts on:
- `http://localhost:3000`

Database:
- Default: `sqlite://./incident_commander.db`
- Override with `DATABASE_URL`

Auth:
- Optional API key auth for all incident endpoints via `API_KEY`
- Send header: `x-api-key: <your-key>`
- `/health` stays public

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

### List Incidents (filter + pagination)
```bash
curl "http://localhost:3000/incidents?status=open&severity=high&limit=20&offset=0"
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

### Add Incident Note
```bash
curl -X POST http://localhost:3000/incidents/<INCIDENT_ID>/notes \
  -H 'Content-Type: application/json' \
  -d '{"note":"Investigating DB connection pool saturation"}'
```

### Get Incident Timeline
```bash
curl http://localhost:3000/incidents/<INCIDENT_ID>/timeline
```

## Notes
- Severity enum: `low | medium | high | critical`
- Status flow: `open -> acknowledged -> resolved`
- Every incident stores an event timeline (`created`, `status_changed`, `note_added`)
- Data resets on restart (in-memory store)
