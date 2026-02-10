-- Admin user (password: "admin123")
INSERT INTO users (id, username, email, password_hash, display_name, role, status, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'admin',
    'admin@test.com',
    '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQ$hash_placeholder',
    'Test Admin',
    'admin',
    'active',
    NOW(),
    NOW()
);

-- Manager user (password: "manager123")
INSERT INTO users (id, username, email, password_hash, display_name, role, status, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000002',
    'manager',
    'manager@test.com',
    '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQ$hash_placeholder',
    'Test Manager',
    'manager',
    'active',
    NOW(),
    NOW()
);

-- Viewer user (password: "viewer123")
INSERT INTO users (id, username, email, password_hash, display_name, role, status, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000003',
    'viewer',
    'viewer@test.com',
    '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQ$hash_placeholder',
    'Test Viewer',
    'viewer',
    'active',
    NOW(),
    NOW()
);

-- Locked user (password: "locked123")
INSERT INTO users (id, username, email, password_hash, display_name, role, status, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000004',
    'locked_user',
    'locked@test.com',
    '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQ$hash_placeholder',
    'Locked User',
    'viewer',
    'locked',
    NOW(),
    NOW()
);

-- Default storage
INSERT INTO storages (id, name, description, provider_type, config, status, is_default, quota_bytes, used_bytes, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000010',
    'Test Local Storage',
    'Default test storage',
    'local',
    '{"root_path": "/tmp/filehub_test/storage/local"}',
    'active',
    TRUE,
    1073741824,
    0,
    NOW(),
    NOW()
);

-- Root folder
INSERT INTO folders (id, storage_id, parent_id, name, path, depth, owner_id, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000020',
    '00000000-0000-0000-0000-000000000010',
    NULL,
    'root',
    '/root',
    0,
    '00000000-0000-0000-0000-000000000001',
    NOW(),
    NOW()
);

-- Subfolder
INSERT INTO folders (id, storage_id, parent_id, name, path, depth, owner_id, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000021',
    '00000000-0000-0000-0000-000000000010',
    '00000000-0000-0000-0000-000000000020',
    'documents',
    '/root/documents',
    1,
    '00000000-0000-0000-0000-000000000001',
    NOW(),
    NOW()
);

-- Test file
INSERT INTO files (id, folder_id, storage_id, name, storage_path, mime_type, size_bytes, owner_id, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000030',
    '00000000-0000-0000-0000-000000000020',
    '00000000-0000-0000-0000-000000000010',
    'test.txt',
    '/root/test.txt',
    'text/plain',
    1024,
    '00000000-0000-0000-0000-000000000001',
    NOW(),
    NOW()
);