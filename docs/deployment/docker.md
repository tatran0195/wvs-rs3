# FileHub Docker Deployment

## Quick Start

```bash
cd docker
docker compose up -d
```

This starts:

- FileHub on port 8080
- PostgreSQL 16 on port 5432
- Redis 7 on port 6379

## Development Mode

```bash
docker compose --profile dev up app-dev postgres redis
```

Development mode features:

- Hot reload via cargo-watch
- Source code mounted as volumes
- Debug logging enabled
- In-memory cache (no Redis dependency)

## Production Deployment

### Environment Variables

| Variable    | Description         | Default        |
| ----------- | ------------------- | -------------- |
| JWT_SECRET  | JWT signing secret  | (required)     |
| DB_PASSWORD | PostgreSQL password | filehub_secret |
| FILEHUB_ENV | Environment name    | production     |
| RUST_LOG    | Log level filter    | info           |

### Create Admin User

```bash
docker compose exec app filehub-cli admin create \
  --username admin \
  --email admin@example.com
```

### Run Migrations

```bash
docker compose exec app filehub-cli migrate run
```

### Health Check

```bash
curl http://localhost:8080/api/health
```

### Volumes

| Volume        | Purpose                        |
| ------------- | ------------------------------ |
| postgres-data | Database storage               |
| redis-data    | Cache persistence              |
| filehub-data  | File storage, plugins, backups |
| filehub-logs  | Application logs               |

### Scaling

For production, consider:

- External PostgreSQL with connection pooling (PgBouncer)
- Redis Cluster or Sentinel for high availability
- Load balancer with sticky sessions for WebSocket
- Shared storage (NFS/S3) for multi-node deployments
