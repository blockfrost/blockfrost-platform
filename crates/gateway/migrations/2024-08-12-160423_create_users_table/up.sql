CREATE TABLE
    users (
        id SERIAL PRIMARY KEY,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        user_id VARCHAR(255) NOT NULL,
        email VARCHAR(255) NOT NULL,
        secret VARCHAR(255) NOT NULL
    );
