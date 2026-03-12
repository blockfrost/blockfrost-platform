-- Step 1: Add the user_id column to the requests table, referencing users.id
ALTER TABLE requests
ADD COLUMN user_id INTEGER NOT NULL;

-- Step 2: Add a foreign key constraint to the user_id column
ALTER TABLE requests
ADD CONSTRAINT fk_requests_user
FOREIGN KEY (user_id) REFERENCES users(id);
