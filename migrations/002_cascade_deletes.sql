-- Add ON DELETE CASCADE to foreign keys so parent deletes clean up children.

-- events.application_id -> applications(id)
ALTER TABLE events
    DROP CONSTRAINT IF EXISTS events_application_id_fkey,
    ADD CONSTRAINT events_application_id_fkey
        FOREIGN KEY (application_id) REFERENCES applications(id) ON DELETE CASCADE;

-- events.job_id -> jobs(id)
ALTER TABLE events
    DROP CONSTRAINT IF EXISTS events_job_id_fkey,
    ADD CONSTRAINT events_job_id_fkey
        FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE;

-- applications.job_id -> jobs(id)
ALTER TABLE applications
    DROP CONSTRAINT IF EXISTS applications_job_id_fkey,
    ADD CONSTRAINT applications_job_id_fkey
        FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE;

-- jobs.company_id -> companies(id)
ALTER TABLE jobs
    DROP CONSTRAINT IF EXISTS jobs_company_id_fkey,
    ADD CONSTRAINT jobs_company_id_fkey
        FOREIGN KEY (company_id) REFERENCES companies(id) ON DELETE CASCADE;
