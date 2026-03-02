CREATE TABLE
    requests (
        id SERIAL PRIMARY KEY,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        route VARCHAR(255) NOT NULL,
        mode VARCHAR(255) NOT NULL,
        ip_address VARCHAR(45) NOT NULL,
        port INTEGER NOT NULL,
        reward_address VARCHAR(255) NOT NULL
    );
