-- Step 1: Drop the foreign key constraint from the requests table
ALTER TABLE requests
DROP CONSTRAINT fk_requests_user;

-- Step 2: Remove the user_id column from the requests table
ALTER TABLE requests
DROP COLUMN user_id;
