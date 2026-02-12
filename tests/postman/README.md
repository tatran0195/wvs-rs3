# FileHub API Postman Collection

This directory contains the production-ready Postman collection and environment for the FileHub API.

## Files

- `filehub_collection.json`: The complete API collection with over 60 endpoints.
- `filehub_environment.json`: A template environment for local development.

## Getting Started

1. **Import Files**:
   - Open Postman.
   - Click **Import** and select both `filehub_collection.json` and `filehub_environment.json`.

2. **Configure Environment**:
   - Select the **FileHub Local** environment in Postman.
   - For production use, duplicate this environment and update the `base_url` to your production API endpoint (e.g., `https://api.filehub.com/api`).

3. **Authentication**:
   - The collection uses **Bearer Token** authentication.
   - Go to the **Auth > Login** request.
   - Ensure `admin_username` and `admin_password` are set in your environment variables.
   - Send the request. A test script will automatically capture the `access_token` and `refresh_token` and save them to your environment.

4. **Running Requests**:
   - All other requests in the collection will inherit the `access_token` from the environment.

## Key Features

- **Logical Organization**: Requests are grouped by domain (Files, Folders, Admin, etc.).
- **Automated Token Management**: No need to manually copy-paste tokens.
- **Validation Tests**: Built-in tests for status codes and response times.
- **Production-Ready**: DTO-aligned request bodies and proper URL parameterization.

## Directory Structure

```text
tests/postman/
├── filehub_collection.json
├── filehub_environment.json
└── README.md
```
