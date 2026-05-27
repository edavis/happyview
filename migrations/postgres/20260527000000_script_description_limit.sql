ALTER TABLE scripts
    ADD CONSTRAINT scripts_description_length CHECK (length(description) <= 300);
