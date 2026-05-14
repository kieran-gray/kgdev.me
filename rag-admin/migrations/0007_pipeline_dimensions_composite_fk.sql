ALTER TABLE embedding_models
    ADD CONSTRAINT embedding_models_id_dimensions_uk UNIQUE (id, dimensions);

ALTER TABLE vector_indexes
    ADD CONSTRAINT vector_indexes_id_dimensions_uk UNIQUE (id, dimensions);

ALTER TABLE pipeline_configurations
    ADD COLUMN dimensions INTEGER;

UPDATE pipeline_configurations pc
SET dimensions = em.dimensions
FROM embedding_models em
WHERE pc.embedding_model_id = em.id;

ALTER TABLE pipeline_configurations
    ALTER COLUMN dimensions SET NOT NULL,
    ADD CONSTRAINT pipeline_configurations_dimensions_positive CHECK (dimensions > 0);

ALTER TABLE pipeline_configurations
    DROP CONSTRAINT pipeline_configurations_embedding_model_id_fkey,
    DROP CONSTRAINT pipeline_configurations_vector_index_id_fkey;

ALTER TABLE pipeline_configurations
    ADD CONSTRAINT pipeline_configurations_embedding_model_fk
        FOREIGN KEY (embedding_model_id, dimensions)
        REFERENCES embedding_models (id, dimensions)
        ON DELETE RESTRICT,
    ADD CONSTRAINT pipeline_configurations_vector_index_fk
        FOREIGN KEY (vector_index_id, dimensions)
        REFERENCES vector_indexes (id, dimensions)
        ON DELETE RESTRICT;

DROP TRIGGER pipeline_configurations_check_dimensions ON pipeline_configurations;
DROP FUNCTION pipeline_configurations_check_dimensions();
