-- Add geographic and temporal targeting to widgets
ALTER TABLE widgets
  ADD COLUMN zip_code TEXT,
  ADD COLUMN city TEXT,
  ADD COLUMN county_id UUID REFERENCES counties(id),
  ADD COLUMN start_date DATE,
  ADD COLUMN end_date DATE;

CREATE INDEX idx_widgets_county_id ON widgets(county_id);

ALTER TABLE widgets ADD CONSTRAINT chk_widget_date_range
  CHECK (start_date IS NULL OR end_date IS NULL OR end_date >= start_date);
