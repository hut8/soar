-- Add unparsed_data column to fixes table to store unparsed packet portions
ALTER TABLE fixes ADD COLUMN unparsed_data VARCHAR;
